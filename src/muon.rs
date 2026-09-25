//! The fixed Metal F32 optimizer comparison; training-only, no inference routing.
use super::*;
use neural::{Backend, RuntimeProfile, checkpoint::OptimizerProtocol};
use recovery::{ObservedCall,RunControl};

const CONTRACT:&str="R3-METAL-F32-MUON-QUALITY-1.0";
const ARMS:[&str;2]=["A","M"];
const EVENT_CONTRACT:&str="R3-SELECTED-EVENT-ID-PROTOCOL-1.1";
const FIT_CONTRACT:&str="R3-FULL-RESPONSE-FIT-1.0";
const FIT_POSTHOC:&str="R3-FULL-RESPONSE-FIT-POSTHOC-1.0";
const FIRST_CONTRACT:&str="R3-COST-BOUNDED-FIRST-DECISION-1.0";
const FIRST_EVAL_ONLY:&str="R3-FIRST-DECISION-EVAL-ONLY-1.0";
const VALUE_READING:&str="R3-VALUE-READING-ISOLATION-1.0";
const FIT_STEPS:[usize;5]=[256,768,1536,2304,3072];
const EVENT_SYSTEM:&str="제공된 기록과 질문만으로 답하세요. 질문에서 지정한 출력 형식만 사용하세요. 기록에 없는 정보를 만들지 마세요. 근거가 없거나 모호하면 구별해서 유보하세요. 순서만으로 원인을 단정하지 마세요.";
const EVENT_TASK:&str="유효한 현재 기록의 사건 번호만 8자리 숫자로 답하라.";
#[path="muon_diagnosis.rs"]
mod diagnosis;
#[derive(Subcommand)]
pub enum Action {
    FirstPrepare { #[arg(long)] original:PathBuf, #[arg(long)] output:PathBuf },
    FirstEvalPrepare { #[arg(long)] original:PathBuf, #[arg(long)] output:PathBuf },
    FirstEvalRevise { #[arg(long)] root:PathBuf },
    FirstEvalPostMainRevise { #[arg(long)] root:PathBuf },
    FirstEvalPostMainAdmit { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    FirstEvalAdmit { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    FirstEval { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["evaluate","report","review-c","review-w"])] phase:String },
    /// No-learning, C-only input isolation. Preparation makes no model call.
    ValueReadingPrepare { #[arg(long)] evaluation:PathBuf, #[arg(long)] output:PathBuf,
        #[arg(long)] prior_executable:PathBuf, #[arg(long)] test_executable:PathBuf },
    ValueReadingRevise { #[arg(long)] root:PathBuf },
    ValueReading { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["parity","u-train","u-dev","s-train","s-dev","report","review"])] phase:String },
    FullFitPosthocPrepare { #[arg(long)] original:PathBuf, #[arg(long)] output:PathBuf },
    FullFitPosthocAdmit { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    FullFitPosthoc { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["evaluate","report","review"])] phase:String },
    FullFitPrepare { #[arg(long)] prior_study:PathBuf, #[arg(long)] review_b:PathBuf, #[arg(long)] output:PathBuf },
    EventPrepare { #[arg(long)] diagnosis:PathBuf, #[arg(long)] audit:PathBuf, #[arg(long)] output:PathBuf },
    /// Separately authorized, read-only endpoint observations; never resumes training.
    Diagnose { #[command(subcommand)] command:diagnosis::Action },
    Prepare { #[arg(long)] parent:PathBuf, #[arg(long)] word_root:PathBuf, #[arg(long)] output:PathBuf },
    Admit { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    Baseline { #[arg(long)] root:PathBuf },
    Train { #[arg(long)] root:PathBuf },
    Report { #[arg(long)] root:PathBuf },
    Review { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["A","M","F","I","C","W"])] arm:String },
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
    #[serde(default,skip_serializing_if="Option::is_none")]
    event:Option<EventProtocol>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    full_fit:Option<FullFit>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    first_decision:Option<FirstDecision>,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct FirstDecision {
    original:PathBuf,plan_hash:String,terminal_hash:String,
    posthoc:PathBuf,posthoc_plan_hash:String,
    source_index:usize,role_map:[u8;32],backward_cap:usize,trace_digest:String,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct FullFit {
    predecessor:PathBuf, predecessor_hash:String, terminal_hash:String,
    source_tape_hash:String, source_offset:usize, train_order:Vec<usize>,
    prior_full_exposure:Vec<usize>, semantic_other_exposure:Vec<usize>,
    exact_identities:Vec<String>, schedule:Vec<usize>,
}
/// Evaluation authority only. Deliberately not a Study/training plan.
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct FitPosthoc {
    contract:String,root:PathBuf,original:PathBuf,plan_hash:String,terminal_hash:String,
    endpoint:Progress,trace_hash:String,prior_usage:(f64,usize,usize),original_bytes:u64,
    source:String,runtime:RuntimeProfile,
}
/// One registered observation of already committed C/W natives; no training state is written.
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct FirstEvalOnly {
    contract:String,mode:String,root:PathBuf,original:PathBuf,plan_hash:String,terminal_hash:String,
    endpoints:Vec<Progress>,trace_digest:String,prior_usage:(f64,usize,usize),
    refs:BTreeMap<PathBuf,String>,original_bytes:u64,source:String,runtime:RuntimeProfile,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct EventProtocol {
    optimizer:OptimizerProtocol, original:PathBuf, original_hash:String,
    corpus:[PathBuf;2], corpus_hashes:[String;2], refs:BTreeMap<PathBuf,String>,
    retention:BTreeMap<String,binary::Value>, parent_exposure:Vec<usize>,
}
impl Study {
    fn arms(&self)->&[&'static str]{if self.first_decision.is_some(){&["C","W"]}else if self.full_fit.is_some(){&["F"]}else if self.event.is_some(){&["F","I"]}else{&ARMS}}
    fn clock(&self,local:usize)->usize{self.event.as_ref().map_or(0,|e|e.optimizer.local_step)+local}
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
fn event_label(request:&ModelRequest,id_only:bool)->Result<(String,i64)> {
    let task=if id_only{EVENT_TASK}else{phrases(Intent::Current)[0]};
    let mut query=request.input.splitn(3,' ');let entity=query.next().unwrap_or("");let context=query.next().unwrap_or("");
    if entity.is_empty()||context.is_empty()||query.next()!=Some(task){return Err(bad("ambiguous event output task"));}
    if request.evidence.items.len()!=2{return Err(bad("event protocol needs both records"));}
    let mut selected=vec![];let mut ids=BTreeSet::new();
    for r in &request.evidence.items {
        if r.event_id.to_string().len()!=8||r.event_id<0||!ids.insert(r.event_id){return Err(bad("event ID schema/duplicate"));}
        let (record_entity,record_context,value)=parsed_record(r)?;
        if entity==record_entity&&context==record_context&&r.version_status=="current"{selected.push((value.to_owned(),r.event_id));}
    }
    if selected.len()!=1{return Err(bad("event selector missing/ambiguous current record"));}
    Ok(selected.remove(0))
}
fn event_episode(e:&Episode,id_only:bool)->Result<Episode>{
    let(value,id)=event_label(&e.request,false)?;
    if e.answer!=format!("{value}입니다. [event:{id}]"){return Err(bad("independent event label disagrees with source"));}
    let mut out=e.clone();out.request.system=EVENT_SYSTEM.into();
    if id_only {
        let prefix=e.request.input.strip_suffix(phrases(Intent::Current)[0]).ok_or_else(||bad("event source task"))?;
        out.request.input=format!("{prefix}{EVENT_TASK}");out.answer=id.to_string();
    }
    if event_label(&out.request,id_only)?!=(value,id){return Err(bad("event protocol changed selector"));}
    Ok(out)
}
fn event_inputs(s:&Study,arm:usize)->Result<data::native::Corpus>{
    let e=s.event.as_ref().ok_or_else(||bad("event policy absent"))?;
    for(p,h)in &e.refs{if file_hash(p)?!=*h||pending_path(p).exists(){return Err(bad("frozen event source changed"));}}
    if file_hash(&e.original)?!=e.original_hash{return Err(bad("source tape changed"));}
    let corpus_arm=if s.first_decision.is_some(){0}else{arm};
    verified_corpus(&e.corpus[corpus_arm],&e.corpus_hashes[corpus_arm])
}
fn event_length(prompt:usize,gold:usize,output_limit:usize)->Result<usize>{
    let target=gold.checked_add(1).ok_or_else(||bad("event target length overflow"))?;
    let total=prompt.checked_add(target).ok_or_else(||bad("event sequence length overflow"))?;
    if total>256||target>output_limit{return Err(bad("event prompt+gold+EOS/output bound"));}Ok(total)
}
fn event_prepare(diagnosis:&Path,audit:&Path,output:&Path)->Result<()> {
    let(old,p,closed,mut refs)=diagnosis::event_parent(diagnosis)?;
    let(c,tm,dm)=inputs(&old)?;let l=checkpoint::load(&p.native,Device::Cpu,true)?;
    let st=l.manifest.training.as_ref().ok_or_else(||bad("A512 training state missing"))?;
    let op=l.manifest.optimizer_protocol.clone().ok_or_else(||bad("A512 optimizer missing"))?;
    let mut expected=OptimizerProtocol::new(&l.model.config,false,digest(&old.runtime)?,old.parent_step)?;expected.local_step=512;
    if old.event.is_some()||p.local!=512||st.step!=14848||op!=expected||st.sampler_state!=512||st.config!=old.config
        ||st.resume_binding!=Some(bind(&old,0,st,&l.tokenizer)?)||old.parent_step!=14336
        ||l.tokenizer.semantic_id()!=old.tokenizer||l.model.adapter().is_some()||c.train.len()!=7680||c.validation.len()!=3456
        ||old.config.lr!=3e-5||old.config.first_target_weight!=1.||old.config.seq_len!=256{return Err(bad("event exact A512 parent/Adam/objective"));}
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let mut previous=old.runtime.clone();previous.binary=runtime.binary.clone();
    if previous!=runtime{return Err(bad("event runtime differs from accepted parent"));}
    let tape=old.tape.get(512..640).ok_or_else(||bad("source tape suffix"))?.to_vec();
    let mut costs=vec![];let mut derived=vec![];let mut bindings=vec![];let mut length_table=vec![];
    for mode in 0..2 {
        let mut train=c.train.clone();let mut dev=c.validation.clone();
        for e in train[6144..].iter_mut().chain(dev[3072..].iter_mut()){*e=event_episode(e,mode==1)?;}
        let ss=samples_with_framing(&train,&l.tokenizer,256,neural::Framing::QuestionEvidence)?;
        let mut seen=BTreeSet::new();let mut current=vec![];let mut lengths:BTreeMap<&str,Vec<[usize;5]>>=BTreeMap::new();
        for (i,e)in train.iter().chain(&dev).enumerate(){
            let prompt=l.tokenizer.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"event-audit")?;
            let target=l.tokenizer.encode(e.answer.as_bytes())?;
            if e.request.evidence.items.len()!=2||!prompt.excluded.is_empty()||event_length(prompt.token_ids.len(),target.len(),e.request.limits.max_tokens as usize).is_err()
                ||e.request.limits.max_tokens!=32||l.tokenizer.decode(&target)?!=e.answer{return Err(bad(&format!("event prompt/target/evidence bounds mode={mode} row={i} evidence={} excluded={} prompt={} target={} max_tokens={}",e.request.evidence.items.len(),prompt.excluded.len(),prompt.token_ids.len(),target.len()+1,e.request.limits.max_tokens)));}
            let index=if i<train.len(){i}else{i-train.len()};let is_word=if i<train.len(){index>=6144}else{index>=3072};
            if is_word&&mode==1&&(target.len()!=8||target.iter().any(|&t|l.tokenizer.decode_bytes(&[t]).map_or(true,|v|v.len()!=1))){return Err(bad("ID_ONLY must be eight digit tokens plus EOS"));}
            let sample=samples_with_framing(std::slice::from_ref(e),&l.tokenizer,256,neural::Framing::QuestionEvidence)?;
            if sample[0].tokens[..sample[0].response_start]!=prompt.token_ids{return Err(bad("event train/generation prefix differs"));}
            let h=digest(&prompt.token_ids)?;if i<train.len(){seen.insert(h);}else if seen.contains(&h){return Err(bad("event train/dev prompt overlap"));}
            let original=if i<train.len(){&c.train[index]}else{&c.validation[index]};
            if (!is_word&&digest(e)?!=digest(original)?)||e.request.evidence!=original.request.evidence{return Err(bad("review/record preservation"));}
            if is_word{current.push(binary::record!({"source":digest(original)?,"episode":digest(e)?,"prompt":digest(&prompt.token_ids)?,"target":digest(&target)?,"mode":if mode==0{"FULL"}else{"ID_ONLY"}}));
                let split=if i<train.len(){"train"}else if index<3264{"dev"}else{"renamed"};
                lengths.entry(split).or_default().push([prompt.token_ids.len(),target.len(),1,e.request.evidence.items.len(),prompt.excluded.len()]);}
        }
        for(split,rows)in lengths{let minmax=|at:usize|[rows.iter().map(|r|r[at]).min().unwrap(),rows.iter().map(|r|r[at]).max().unwrap()];
            let totals=rows.iter().map(|r|r[0]+r[1]+r[2]).collect::<Vec<_>>();
            let summary=binary::record!({"mode":if mode==0{"FULL"}else{"ID_ONLY"},"split":split,"count":rows.len(),"prompt_min_max":minmax(0),"gold_min_max":minmax(1),"eos_min_max":minmax(2),"total_min_max":[totals.iter().min(),totals.iter().max()],"provided_min_max":minmax(3),"excluded_min_max":minmax(4),"violations":totals.iter().filter(|&&n|n>256).count()});
            println!("EVENT_LENGTH {summary}");length_table.push(summary);}
        let(mut input,mut target,mut padding,mut max_batch)=(0usize,0usize,0usize,0usize);let mut exposure=vec![0usize;train.len()];
        for row in &tape{if row[..4].iter().any(|&i|i>=6144)||row[4..].iter().any(|&i|!(6144..7680).contains(&i)){return Err(bad("event exact review4/word4 tape"));}
            let n=row.iter().map(|&i|ss[i].tokens.len()-1).max().unwrap();
            max_batch=max_batch.max(n);
            for &i in row{exposure[i]+=1;input+=ss[i].tokens.len()-1;target+=ss[i].tokens.len()-ss[i].response_start;padding+=n-(ss[i].tokens.len()-1);}}
        costs.push(binary::record!({"input":input,"target":target,"padding":padding,"exposure":exposure,"rows":tape.len()*8,"max_shifted_batch_length":max_batch}));
        let mut manifest=c.manifest.clone();manifest.generator=EVENT_CONTRACT.into();manifest.train=data::native::split("train",&train);manifest.validation=data::native::split("validation",&dev);
        derived.push(data::native::from_episodes(manifest,train,dev)?);bindings.push(current);
    }
    let train_groups=c.train[6144..].iter().map(|e|e.binding.clone()).collect::<BTreeSet<_>>();
    let train_ids=c.train[6144..].iter().flat_map(|e|e.request.evidence.items.iter().map(|r|r.event_id)).collect::<BTreeSet<_>>();
    if c.validation[3072..].iter().any(|e|train_groups.contains(&e.binding)||e.request.evidence.items.iter().any(|r|train_ids.contains(&r.event_id))){return Err(bad("event semantic/ID split overlap"));}
    for(i,(f,id))in derived[0].train.iter().zip(&derived[1].train).enumerate(){if i>=6144&&f.request.input==id.request.input{return Err(bad("conflicting targets at identical request"));}}
    let mut parent_exposure=vec![0usize;c.train.len()];for &i in old.tape[..512].iter().flatten(){parent_exposure[i]+=1;}
    let mut retention=BTreeMap::new();
    for name in ["value","citation","S1Q1","word"]{
        let row=closed["outputs"].as_array().and_then(|v|v.iter().find(|r|r["arm"]=="A"&&r["panel"]==name)).ok_or_else(||bad("accepted A512 baseline absent"))?;
        if row["score"]["joint"]["total"]!=64{return Err(bad("A512 retention denominator"));}retention.insert(name.into(),row["score"].clone());
    }
    for path in [audit.canonicalize()?,old.word_root.join("tokenizer.r3b"),old.word_root.join("metadata.r3b")]{refs.insert(path.clone(),file_hash(&path)?);}
    std::fs::create_dir(output)?;let root=output.canonicalize()?;
    let paths=[root.join("FULL.r3cor"),root.join("ID_ONLY.r3cor")];
    for a in 0..2{data::native::write(&paths[a],&derived[a],true)?;}
    let mut s=old.clone();s.contract=EVENT_CONTRACT.into();s.root=root.clone();s.parent=p.native;s.parent_hash=p.physical;s.parent_step=st.step;
    s.parent_content=l.model.weights_content_id()?;s.parent_adam=optimizer_hash(&l.optimizer)?;s.source=sources()?;s.runtime=runtime;s.tape=tape;
    s.max_updates=128;s.generation_cap=4096;s.teacher_cap=288;s.active_cap=3600.;s.segment_cap=900.;s.bytes_cap=1<<30;
    s.config.budget_start_step=st.step;s.config.max_steps=st.step+128;s.config.budget_start_tokens=st.consumed_tokens;
    s.config.max_tokens=st.consumed_tokens+costs.iter().map(|v|v["input"].as_u64().unwrap()).max().unwrap();s.config.warmup=0;
    s.costs=binary::record!({"FULL":costs[0],"ID_ONLY":costs[1],"original_tape":digest(&old.tape)?,"suffix":digest(&s.tape)?,"parent_exposure":parent_exposure});
    s.event=Some(EventProtocol{optimizer:op,original:old.root.join("plan.r3b"),original_hash:file_hash(&old.root.join("plan.r3b"))?,corpus:paths.clone(),corpus_hashes:[file_hash(&paths[0])?,file_hash(&paths[1])?],refs,retention,parent_exposure});
    for a in s.arms(){std::fs::create_dir(root.join(a))?;}
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"policy":digest(&s)?,"source":s.source,"parent":s.parent_hash,"weights":s.parent_content,"adam":s.parent_adam,"model_step":14848,"optimizer_step":512,"cursor":0,"data_bindings":bindings,"length_table":length_table,"costs":s.costs,"train_count":tm.len(),"dev_count":dm.len(),"generation":0,"teacher":0,"optimizer":0}))?;
    println!("EVENT_PREPARED policy={} native={} weights={} Adam={} tape512..640 calls0 A_PENDING",digest(&s)?,s.parent_hash,s.parent_content,s.parent_adam);Ok(())
}
fn event_score(name:&str,es:&[Episode],ms:&[Meta],rows:&[binary::Value],tok:&ByteBpe)->Result<binary::Value>{
    if !name.starts_with("ID_ONLY-"){return scored(name,es,ms,rows,tok);}
    if es.is_empty()||es.len()%4!=0{return Err(bad("ID_ONLY complete four-view panel"));}
    let base=score(rows,es,ms)?;let mut exact=vec![];let mut classes=[0usize;4];let mut digits=[0usize;8];let mut first=BTreeMap::<String,usize>::new();let mut lengths=BTreeMap::<usize,usize>::new();
    for(e,r)in es.iter().zip(rows){
        verify_output_result(r,tok)?;let(_,id)=event_label(&e.request,true)?;
        if e.answer!=id.to_string(){return Err(bad("ID_ONLY frozen target mismatch"));}
        let actual=r["actual"].as_str();let completed=r["generation_completed"]==true&&r["finish_reason"]=="stop"&&r["error"].is_null();
        let valid=completed&&actual.is_some_and(|a|a.len()==8&&a.bytes().all(|b|b.is_ascii_digit()));
        let good=valid&&actual==Some(e.answer.as_str());exact.push(good);
        let kind=if good{0}else if !valid{3}else if e.request.evidence.items.iter().any(|e|Some(e.event_id.to_string().as_str())==actual){1}else{2};classes[kind]+=1;
        if let Some(a)=actual{*lengths.entry(a.len()).or_default()+=1;
            for(i,(&x,&y))in e.answer.as_bytes().iter().zip(a.as_bytes()).take(8).enumerate(){digits[i]+=usize::from(completed&&x==y&&a.len()==8);}
            let d=e.answer.as_bytes().iter().zip(a.as_bytes()).position(|(x,y)|x!=y).or_else(||(a.len()!=8).then_some(a.len().min(8)));
            *first.entry(d.map_or("none".into(),|n|n.to_string())).or_default()+=1;
        }else{*first.entry("no_text".into()).or_default()+=1;}
    }
    let(mut qb,mut sb,mut all4)=(0,0,0);
    for(g,meta)in exact.chunks_exact(4).zip(ms.chunks_exact(4)){
        if meta.iter().enumerate().any(|(i,m)|m.view!=i||m.base!=meta[0].base){return Err(bad("ID_ONLY view order"));}
        qb+=usize::from(g[0]&&g[1])+usize::from(g[2]&&g[3]);sb+=usize::from(g[0]&&g[2])+usize::from(g[1]&&g[3]);all4+=usize::from(g.iter().all(|&x|x));
    }
    if base.exact!=classes[0]{return Err(bad("ID_ONLY strict score disagreement"));}
    Ok(binary::record!({"scorer":"selected-event-id-v1","mode":"ID_ONLY","joint":{"total":es.len(),"full":classes[0],"query_both":qb,"swap_both":sb,"all4":all4,"eos":base.eos,"errors":base.errors,"exact":exact},
        "value_swap_invariance_both":sb,"id_exact":classes[0],"other_provided_id":classes[1],"valid_outside_id":classes[2],"parse_failure_rows":classes[3],"digit_correct":digits,"first_digit_difference":first,"text_byte_lengths":lengths,"value_metric":"N/A; assignment swap preserves ID"}))
}
fn event_gate(r:&binary::Value)->bool{
    r["joint"]["total"]==192&&r["joint"]["full"].as_u64().is_some_and(|v|v>=183)
        &&r["joint"]["query_both"].as_u64().is_some_and(|v|v>=88)&&r["joint"]["all4"].as_u64().is_some_and(|v|v>=44)
        &&r["joint"]["errors"]==0&&r["valid_outside_id"]==0&&r["parse_failure_rows"]==0
}
fn event_panels(s:&Study,final_eval:bool,baseline:bool)->Result<Vec<Panel>>{
    let(c,tm,dm)=inputs(s)?;let mut panels=vec![];
    if !baseline{for(name,at)in [("value",0),("citation",512),("S1Q1",2560)]{panels.push((name.into(),c.validation[at..at+64].to_vec(),dm[at..at+64].to_vec()));}}
    for mode in 0..2{let corpus=event_inputs(s,mode)?;let prefix=if mode==0{"FULL"}else{"ID_ONLY"};
        for(name,at)in [("word",3072),("renamed",3264)]{if name=="renamed"&&!final_eval&&!baseline{continue;}
            panels.push((format!("{prefix}-{name}"),corpus.validation[at..at+192].to_vec(),dm[at..at+192].to_vec()));}
        if final_eval&&!baseline{let ids=(0..2).flat_map(|v|(0..16).flat_map(move|b|(0..4).map(move|q|6144+v*768+b*4+q))).collect::<Vec<_>>();
            panels.push((format!("{prefix}-train128"),ids.iter().map(|&i|corpus.train[i].clone()).collect(),ids.iter().map(|&i|tm[i].clone()).collect()));}
    }
    if final_eval&&!baseline{panels.push(("OLD_FULL".into(),c.validation[3072..3136].to_vec(),dm[3072..3136].to_vec()));}Ok(panels)
}
fn event_binding(s:&Study,arm:&str,step:usize,model:&str,panel:&Panel)->Result<binary::Value>{
    Ok(binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"arm":arm,"local":step,"absolute":s.parent_step+step,"model":model,"tokenizer":s.tokenizer,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,"panel":panel.0,"planned":panel.1.len(),"scorer":"selected-event/full-v1","call_protocol":1}))
}
fn event_panel_score(s:&Study,arm:&str,step:usize,l:&checkpoint::Loaded,panel:&Panel,limit:usize,ctl:&mut RunControl)->Result<binary::Value>{
    let dir=if arm=="parent"{s.root.clone()}else{s.root.join(arm)};let label=format!("eval-{step}-{}",panel.0);
    let binding=event_binding(s,arm,step,&l.model.weights_content_id()?,panel)?;
    let rows=generated_until(l,&dir,&label,&panel.1,&binding,ctl,limit)?;
    let mut score=event_score(&panel.0,&panel.1[..limit],&panel.2[..limit],&rows[..limit],&l.tokenizer)?;
    score["binding"]=binding;score["rows_digest"]=binary::record!(digest(&&rows[..limit])?);
    let path=dir.join(format!("{label}-score-{limit}.r3b"));
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("event stored score mismatch"));}}else{publish_confirmed(&path,&score)?;}
    println!("EVENT_SCORE arm={arm} step={step} panel={} FULL={} QB={} SB={} ALL4={} outside={} malformed={} EOS={}",panel.0,score["joint"]["full"],score["joint"]["query_both"],score["joint"]["swap_both"],score["joint"]["all4"],score["valid_outside_id"],score["parse_failure_rows"],score["joint"]["eos"]);Ok(score)
}
fn event_baseline(s:&Study)->Result<()> {
    if !history(s)?.is_empty(){return Err(bad("baseline after new training"));}
    let(i,mut ctl)=start_observation(s,"baseline",&binary::record!({"policy":digest(s)?,"parent":s.parent_hash,"planned":768}))?;
    let mut sufficient=true;
    let result=(||->Result<()>{let l=checkpoint::load(&s.parent,Backend::Metal0.open()?,false)?;
        for panel in event_panels(s,false,true)?{let r=event_panel_score(s,"parent",0,&l,&panel,panel.1.len(),&mut ctl)?;if panel.0.starts_with("ID_ONLY"){sufficient&=event_gate(&r);}}
        ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,"baseline",i,&mut ctl,&result,binary::record!({"id_sufficient":sufficient&&result.is_ok(),"classification":if sufficient&&result.is_ok(){"BASELINE_ID_CONTRACT_SUFFICIENT"}else{"BOUNDED_COMPARISON_REQUIRED"}}))?;result
}
fn event_teacher_cases(s:&Study,mode:usize)->Result<Vec<Episode>>{
    let c=event_inputs(s,mode)?;
    let ids=(0..2).flat_map(|v|(0..4).flat_map(move|b|(0..4).map(move|q|6144+v*768+b*4+q)));
    Ok(ids.map(|i|c.train[i].clone()).chain(c.validation[3072..3104].iter().cloned()).collect())
}
fn event_evaluate(s:&Study,arm:usize,l:&mut checkpoint::Loaded,p:&Progress,final_eval:bool,ctl:&mut RunControl)->Result<BTreeMap<String,binary::Value>>{
    l.model.refresh_identity()?;let mut out=BTreeMap::new();
    for panel in event_panels(s,final_eval,false)?{let n=if final_eval{panel.1.len()}else{64};
        out.insert(panel.0.clone(),event_panel_score(s,s.arms()[arm],p.local,l,&panel,n,ctl)?);}
    if final_eval{for mode in 0..2{
        let es=event_teacher_cases(s,mode)?;let label=format!("teacher-{}-{}",p.local,if mode==0{"FULL"}else{"ID_ONLY"});
        let binding=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"model":l.model.weights_content_id()?,"arm":s.arms()[arm],"local":p.local,"mode":mode,"cases":digest(&es)?,"planned":64,"call_protocol":1});
        let rows=teacher_prefix(&s.root.join(s.arms()[arm]),&label,&binding,l,&es,ctl)?;
        diagnosis::validate_teacher_forward(&s.runtime,s.config.seq_len,&es,&rows,&l.tokenizer)?;
        let score=binary::record!({"binding":binding,"train":diagnosis::teacher_scores(&es[..32],&rows[..32],&l.tokenizer)?,"dev":diagnosis::teacher_scores(&es[32..],&rows[32..],&l.tokenizer)?,"rows_digest":digest(&rows)?});
        let path=s.root.join(s.arms()[arm]).join(format!("{label}-score.r3b"));if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("event teacher score mismatch"));}}else{publish_confirmed(&path,&score)?;}
    }}Ok(out)
}
fn event_decision(s:&Study,arm:usize,p:&Progress,scores:&BTreeMap<String,binary::Value>)->Result<Option<String>>{
    let (warning,stop)=event_retention(s,scores)?;
    let value=binary::record!({"policy":digest(s)?,"local":p.local,"scores":scores,"warnings":warning,"stop":stop});
    let path=s.root.join(s.arms()[arm]).join(format!("decision-{}.r3b",p.local));if path.exists(){if read_confirmed::<binary::Value>(&path)?!=value{return Err(bad("event guard decision changed"));}}else{publish_confirmed(&path,&value)?;}Ok(stop)
}
fn event_retention(s:&Study,scores:&BTreeMap<String,binary::Value>)->Result<(Vec<&'static str>,Option<String>)>{
    let e=s.event.as_ref().unwrap();let mut warning=vec![];let mut stop=None;
    for name in ["value","citation","S1Q1"]{let r=&scores[name];
        let full=r["joint"]["full"].as_u64().ok_or_else(||bad("event guard full missing"))?;
        let before=e.retention[name]["joint"]["full"].as_u64().ok_or_else(||bad("parent guard full missing"))?;
        let errors=r["joint"]["errors"].as_u64().ok_or_else(||bad("event guard error missing"))?;
        if r["joint"]["total"]!=64{return Err(bad("event guard denominator"));}
        if before.saturating_sub(full)>=4{warning.push(name);}if before.saturating_sub(full)>=12||errors>=4{stop=Some("SEVERE_RETENTION".into());}
    }
    Ok((warning,stop))
}
fn event_read_panel(s:&Study,arm:&str,step:usize,model:&str,panel:&Panel,n:usize)->Result<(binary::Value,Vec<binary::Value>)>{
    let dir=if arm=="parent"{s.root.clone()}else{s.root.join(arm)};let label=format!("eval-{step}-{}",panel.0);
    let raw=binary::read_value_records(&dir.join(format!("{label}.r3rows")))?;
    let b=event_binding(s,arm,step,model,panel)?;
    if raw.len()<n+1||raw.len()>panel.1.len()+1||raw[0]!=b{return Err(bad("event raw panel incomplete/identity"));}
    for(i,(e,r))in panel.1.iter().zip(&raw[1..]).enumerate(){call_attempt(&dir,&label,"generation",&b,e,i,Some(r))?;}
    let rows=raw[1..n+1].to_vec();let mut r=event_score(&panel.0,&panel.1[..n],&panel.2[..n],&rows,&scoring_tokenizer(s)?)?;
    r["binding"]=b;r["rows_digest"]=binary::record!(digest(&rows)?);
    if read_confirmed::<binary::Value>(&dir.join(format!("{label}-score-{n}.r3b")))?!=r{return Err(bad("event independently recounted score mismatch"));}Ok((r,rows))
}
fn event_read_teachers(s:&Study,arm:usize,p:&Progress,mode:usize)->Result<(Vec<Episode>,Vec<binary::Value>)>{
    let es=event_teacher_cases(s,mode)?;let dir=s.root.join(s.arms()[arm]);let label=format!("teacher-{}-{}",p.local,if mode==0{"FULL"}else{"ID_ONLY"});
    let raw=binary::read_value_records(&dir.join(format!("{label}-teachers.r3rows")))?;
    let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
    let b=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"model":model,"arm":s.arms()[arm],"local":p.local,"mode":mode,"cases":digest(&es)?,"planned":64,"call_protocol":1});
    if raw.len()!=65||raw[0]!=b{return Err(bad("event teacher panel binding/count"));}
    for(i,(e,r))in es.iter().zip(&raw[1..]).enumerate(){call_attempt(&dir,&label,"teacher",&b,e,i,Some(r))?;}
    let tok=scoring_tokenizer(s)?;diagnosis::validate_teacher_forward(&s.runtime,s.config.seq_len,&es,&raw[1..],&tok)?;
    let expected=binary::record!({"binding":b,"train":diagnosis::teacher_scores(&es[..32],&raw[1..33],&tok)?,"dev":diagnosis::teacher_scores(&es[32..],&raw[33..],&tok)?,"rows_digest":digest(&&raw[1..])?});
    if read_confirmed::<binary::Value>(&dir.join(format!("{label}-score.r3b")))?!=expected{return Err(bad("event teacher recount mismatch"));}
    println!("EVENT_TEACHER arm={} mode={} train={} dev={}",s.arms()[arm],mode,expected["train"],expected["dev"]);Ok((es,raw[1..].to_vec()))
}
fn event_report(s:&Study)->Result<()> {
    let mut parent=BTreeMap::new();let mut endpoint=vec![];
    let baseline:binary::Value=read_confirmed(&s.root.join("baseline-finished.r3b"))?;
    if baseline["success"]!=true{return Err(bad("event baseline incomplete"));}
    for panel in event_panels(s,false,true)?{let(r,_)=event_read_panel(s,"parent",0,&s.parent_content,&panel,panel.1.len())?;
        println!("EVENT_PARENT {} {}",panel.0,r);parent.insert(panel.0,r);}
    let h=history(s)?;
    if baseline["extra"]["id_sufficient"]==true{if !h.is_empty()||!["ID_ONLY-word","ID_ONLY-renamed"].iter().all(|n|event_gate(&parent[*n])){return Err(bad("baseline skip branch mismatch"));}
        println!("BASELINE_ID_CONTRACT_SUFFICIENT updates0 {:?} Goal1=false",previous_usage(s,&h)?);return Ok(());}
    let end=h.last().ok_or_else(||bad("EVENT_TRAIN_NOT_RUN"))?;
    for arm in 0..2{
        let p=&end.arms[arm];let(m,_)=checkpoint::metadata(&p.native)?;let st=m.training.as_ref().ok_or_else(||bad("event endpoint training absent"))?;
        let op=m.optimizer_protocol.as_ref().ok_or_else(||bad("event endpoint optimizer absent"))?;op.validate(&m.architecture,st)?;
        if op.local_step!=s.clock(p.local)||st.step!=s.parent_step+p.local||st.sampler_state!=p.local as u64||st.config!=s.config
            ||st.resume_binding!=Some(bind(s,arm,st,&scoring_tokenizer(s)?)?){return Err(bad("event endpoint clock/objective"));}
        if !end.success||end.resume||!p.fit{return Err(bad("EVENT_ENDPOINT_INCOMPLETE; pure partial receipts preserved"));}
        let mut scores=BTreeMap::new();for panel in event_panels(s,true,false)?{let(r,_)=event_read_panel(s,s.arms()[arm],p.local,&m.model_content_digest,&panel,panel.1.len())?;
            println!("EVENT_FINAL arm={} panel={} score={}",s.arms()[arm],panel.0,r);scores.insert(panel.0,r);}
        for mode in 0..2{event_read_teachers(s,arm,p,mode)?;}
        for step in [64,128].into_iter().filter(|&step|step<=p.local){
            let saved=h.iter().flat_map(|v|v.arms.iter().enumerate()).find(|(a,x)|*a==arm&&x.local==step&&x.evaluated==step).map(|(_,x)|x);
            // A later saved endpoint can share the same segment; the immutable
            // native name and its manifest still bind the earlier decision.
            let native=if let Some(x)=saved{x.native.clone()}else{
                let paths=std::fs::read_dir(s.root.join(s.arms()[arm]))?.collect::<std::io::Result<Vec<_>>>()?;
                let mut matches=paths.into_iter().map(|e|e.path()).filter(|p|p.file_name().unwrap().to_string_lossy().ends_with(&format!("-step-{step}.r3m"))).collect::<Vec<_>>();
                if matches.len()!=1{return Err(bad("event guard native ambiguous"));}matches.remove(0)
            };
            let model=checkpoint::metadata(&native)?.0.model_content_digest;
            let mut middle=BTreeMap::new();for panel in event_panels(s,false,false)?{let(r,_)=event_read_panel(s,s.arms()[arm],step,&model,&panel,64)?;middle.insert(panel.0,r);}
            let (warnings,stop)=event_retention(s,&middle)?;
            let expected=binary::record!({"policy":digest(s)?,"local":step,"scores":middle,"warnings":warnings,"stop":stop});
            if read_confirmed::<binary::Value>(&s.root.join(s.arms()[arm]).join(format!("decision-{step}.r3b")))?!=expected
                ||step==p.local&&stop!=p.stop||step<p.local&&stop.is_some(){return Err(bad("event guard/raw/terminal disagreement"));}
        }
        let mut count=0;let(mut tokens,mut targets,mut padding)=(0u64,0u64,0u64);let mut exposures=vec![0usize;7680];
        let samples=samples_with_framing(&event_inputs(s,arm)?.train,&scoring_tokenizer(s)?,256,neural::Framing::QuestionEvidence)?;
        for index in 0..h.len(){let path=s.root.join(s.arms()[arm]).join(format!("updates-{index:03}.r3rows"));if !path.exists(){continue;}
            for r in binary::read_value_records(&path)?{let draw=s.tape.get(count).ok_or_else(||bad("event trace exceeds tape"))?;count+=1;
                let length=draw.iter().map(|&i|samples[i].tokens.len()-1).max().unwrap();let mut actual=[0usize;3];
                for &i in draw{exposures[i]+=1;actual[0]+=samples[i].tokens.len()-1;actual[1]+=samples[i].tokens.len()-samples[i].response_start;actual[2]+=length-(samples[i].tokens.len()-1);}
                if r["local"]!=count||r["model_step"]!=s.parent_step+count||r["optimizer_local"]!=s.clock(count)||r["rows"]!=binary::record!(draw)||r["lr"]!=s.config.lr
                    ||r["input"]!=actual[0]||r["target"]!=actual[1]||r["padding"]!=actual[2]||r["examples"]!=8{return Err(bad("event trace cost/tape/clock mismatch"));}
                tokens+=actual[0]as u64;targets+=actual[1]as u64;padding+=actual[2]as u64;
            }}if count!=p.local{return Err(bad("event committed trace count"));}
        let ids=(0..2).flat_map(|v|(0..16).flat_map(move|b|(0..4).map(move|q|6144+v*768+b*4+q))).collect::<Vec<_>>();
        println!("EVENT_TRACE arm={} updates={count} clock={} samples={} unique={} input={tokens} target={targets} padding={padding} fit_parent_exposure={:?} fit_new_exposure={:?} native={} hash={} stop={:?}",s.arms()[arm],op.local_step,count*8,exposures.iter().filter(|&&v|v>0).count(),ids.iter().map(|&i|s.event.as_ref().unwrap().parent_exposure[i]).collect::<Vec<_>>(),ids.iter().map(|&i|exposures[i]).collect::<Vec<_>>(),p.native.display(),p.physical,p.stop);
        endpoint.push(scores);
    }
    if end.arms[0].local!=end.arms[1].local{return Err(bad("event unequal endpoints"));}
    let mut signal=end.arms.iter().all(|p|p.stop.is_none());
    for name in ["ID_ONLY-word","ID_ONLY-renamed"]{
        let i=&endpoint[1][name];for r in [&parent[name],&endpoint[0][name]]{signal&=i["joint"]["full"].as_u64().unwrap()>=r["joint"]["full"].as_u64().unwrap()+16&&i["joint"]["query_both"].as_u64().unwrap()>=r["joint"]["query_both"].as_u64().unwrap()+8;}
        let f:Vec<bool>=binary::from_value(endpoint[0][name]["joint"]["exact"].clone())?;let i:Vec<bool>=binary::from_value(i["joint"]["exact"].clone())?;let mut paired=[0usize;4];
        for(f,i)in f.iter().zip(&i){paired[match(f,i){(true,true)=>0,(false,true)=>1,(true,false)=>2,_=>3}]+=1;}
        println!("EVENT_PAIRED {name} both/I_gain/I_loss/neither={paired:?} strong_F={} strong_I={}",event_gate(&endpoint[0][name]),event_gate(&endpoint[1][name]));
    }
    println!("EVENT_CLOSED NEXT_SIGNAL={} usage={:?} bytes={} execution_complete=true Goal1=false",if signal{"FOLLOWUP_SIGNAL"}else{"NO_CLEAR_SIGNAL"},previous_usage(s,&h)?,owned_bytes(&s.root)?);Ok(())
}
fn event_review(s:&Study,arm:usize)->Result<()> {
    let h=history(s)?;let end=h.last().ok_or_else(||bad("event endpoint absent"))?;
    if !end.success||end.resume||!end.arms.iter().all(|p|p.fit){return Err(bad("event B requires normally complete endpoints"));}
    let p=&end.arms[arm];let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
    let mut all=vec![];let mut raw=vec![];let mut normal=BTreeSet::new();
    for panel in event_panels(s,true,false)?{
        let take=match panel.0.as_str(){"value"|"citation"|"S1Q1"=>4,"FULL-word"|"ID_ONLY-word"=>8,"FULL-renamed"|"ID_ONLY-renamed"=>2,_=>0};if take==0{continue;}
        let(_,rs)=event_read_panel(s,s.arms()[arm],p.local,&model,&panel,panel.1.len())?;
        for(i,(e,r))in panel.1.into_iter().zip(rs).enumerate(){if i<take{normal.insert(all.len());}all.push(e);raw.push(r);}
    }
    let mut selected=normal.iter().copied().collect::<Vec<_>>();let mut seen=normal.clone();let mut failures=(0..raw.len()).filter(|&i|raw[i]["exact_match"]!=true).collect::<Vec<_>>();
    let mut categories:[Vec<usize>;3]=Default::default();
    for i in failures.drain(..){let r=&raw[i];let text=r["actual"].as_str();let id=all[i].request.input.ends_with(EVENT_TASK);
        let shape=text.is_some_and(|a|if id{a.len()==8&&a.bytes().all(|b|b.is_ascii_digit())}else{a.split_once("입니다. [event:").and_then(|(_,r)|r.strip_suffix(']')).is_some_and(|v|v.len()==8&&v.bytes().all(|b|b.is_ascii_digit()))});
        let rank=if r["error_class"]=="strict_utf8"{0}else if !shape||r["finish_reason"]!="stop"{1}else{2};categories[rank].push(i);}
    for at in 0..raw.len(){for group in &categories{if let Some(&i)=group.get(at){for j in [i,i^1]{if selected.len()<64&&j<raw.len()&&seen.insert(j){selected.push(j);}}}}if selected.len()>=64{break;}}
    let es=selected.iter().map(|&i|all[i].clone()).collect::<Vec<_>>();let expected=selected.iter().map(|&i|raw[i].clone()).collect::<Vec<_>>();
    let label=format!("review-{}",s.arms()[arm]);let(indices,mut ctl)=start_observation(s,&label,&binary::record!({"policy":digest(s)?,"native":p.physical,"cases":digest(&es)?,"selected":selected,"normal":normal,"teacher_indices":[0,1,16,17,32,33,60,61]}))?;
    let result=(||->Result<()>{let l=checkpoint::load(&p.native,Backend::Metal0.open()?,false)?;let binding=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"native":p.physical,"cases":digest(&es)?,"call_protocol":1});
        let actual=generated(&l,&s.root,&label,&es,&binding,&mut ctl)?;
        for(a,b)in actual.iter().zip(&expected){for field in ["raw_tokens","actual","finish_reason","error","generation_completed"]{if a[field]!=b[field]{return Err(bad("event B generation mismatch"));}}}
        for mode in 0..2{let(cases,expected)=event_read_teachers(s,arm,p,mode)?;let fixed=[0usize,1,16,17,32,33,60,61];let es=fixed.iter().map(|&i|cases[i].clone()).collect::<Vec<_>>();
            let b=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"native":p.physical,"cases":digest(&es)?,"mode":mode,"call_protocol":1});
            let rows=teacher_prefix(&s.root,&format!("{label}-teacher-{mode}"),&b,&l,&es,&mut ctl)?;diagnosis::teacher_scores(&es,&rows,&l.tokenizer)?;
            for(r,&i)in rows.iter().zip(&fixed){let a=&r["teacher"]["target_token_observation"];let b=&expected[i]["teacher"]["target_token_observation"];
                if a["gold"]!=b["gold"]||a["argmax"]!=b["argmax"]{return Err(bad("event B teacher IDs mismatch"));}
                let x:Vec<f64>=binary::from_value(a["nll"].clone())?;let y:Vec<f64>=binary::from_value(b["nll"].clone())?;
                if x.len()!=y.len()||x.iter().zip(y).any(|(a,b)|(a-b).abs()>1e-5){return Err(bad("event B teacher NLL mismatch"));}}
        }ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,&label,indices,&mut ctl,&result,binary::record!({"normal":32,"selected":es.len(),"teacher":16,"native":p.physical}))?;result
}

fn fit_rotation(source:&[[usize;8]],offset:usize)->Result<Vec<[usize;8]>>{
    if source.len()!=3072||offset>=source.len(){return Err(bad("FULL_FIT source tape bounds"));}
    Ok(source[offset..].iter().chain(&source[..offset]).copied().collect())
}
fn fit_train_order(c:&data::native::Corpus,tm:&[Meta])->Result<Vec<usize>>{
    let mut pairs=BTreeMap::<String,BTreeMap<String,Vec<usize>>>::new();
    for i in 6144..c.train.len(){
        let (pair,version)=tm[i].template.split_once("/id").ok_or_else(||bad("FULL_FIT word pair metadata"))?;
        if !["0","1"].contains(&version)||tm[i].view>3{return Err(bad("FULL_FIT view/version"));}
        pairs.entry(pair.into()).or_default().entry(c.train[i].binding.clone()).or_default().push(i);
    }
    if pairs.len()!=6{return Err(bad("FULL_FIT six word pairs required"));}
    let mut selected=vec![];
    for groups in pairs.values(){if groups.len()!=32{return Err(bad("FULL_FIT semantic group count"));}
        for rows in groups.values().take(4){
            let mut rows=rows.clone();rows.sort();
            if rows.len()!=8||rows.iter().enumerate().any(|(j,&i)|tm[i].view!=j%4){return Err(bad("FULL_FIT complete two-ID four-view group"));}
            selected.extend(rows);
        }
    }
    let set=selected.iter().copied().collect::<BTreeSet<_>>();
    if set.len()!=192{return Err(bad("FULL_FIT representative train192"));}
    selected.extend((6144..7680).filter(|i|!set.contains(i)));Ok(selected)
}
fn fit_exposure(tape:&[[usize;8]])->Result<Vec<usize>>{
    let mut out=vec![0;7680];for row in tape{
        if row[..4].iter().any(|&i|i>=6144)||row[4..].iter().any(|&i|!(6144..7680).contains(&i)){return Err(bad("FULL_FIT review4/word4 slots"));}
        for &i in row{out[i]+=1;}
    }Ok(out)
}
fn fit_costs(c:&data::native::Corpus,tok:&ByteBpe,tape:&[[usize;8]])->Result<binary::Value>{
    let ss=samples_with_framing(&c.train,tok,256,neural::Framing::QuestionEvidence)?;
    let(mut input,mut target,mut padding,mut max_batch)=(0usize,0usize,0usize,0usize);
    for row in tape{let n=row.iter().map(|&i|ss[i].tokens.len()-1).max().unwrap();max_batch=max_batch.max(n);
        for &i in row{input+=ss[i].tokens.len()-1;target+=ss[i].tokens.len()-ss[i].response_start;padding+=n-(ss[i].tokens.len()-1);}}
    if input>8_000_000||target>1_000_000{return Err(bad("FULL_FIT token budget"));}
    Ok(binary::record!({"input":input,"target":target,"padding":padding,"max_shifted_batch_length":max_batch,"exposure":fit_exposure(tape)?}))
}
fn fit_identity(e:&Episode,tok:&ByteBpe)->Result<String>{
    let p=tok.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"full-fit-input")?;
    let mut target=tok.encode(e.answer.as_bytes())?;target.push(EOS);
    if p.excluded.len()!=0||e.request.evidence.items.len()!=2||e.request.limits.max_tokens!=32
        ||p.token_ids.len()+target.len()>256||target.len()>32{return Err(bad("FULL_FIT complete input bounds"));}
    digest(&(tok.semantic_id(),neural::Framing::QuestionEvidence.digest(),digest(&p.token_ids)?,digest(&target)?))
}
fn fit_prepare(prior:&Path,review_b:&Path,output:&Path)->Result<()> {
    let old=load_study(prior,false)?;
    if old.full_fit.is_some()||old.contract!=EVENT_CONTRACT{return Err(bad("FULL_FIT requires closed F/I predecessor"));}
    let h=history(&old)?;let terminal=h.last().ok_or_else(||bad("FULL_FIT predecessor absent"))?;
    if !terminal.success||terminal.resume||!terminal.arms.iter().all(|a|a.fit)||terminal.arms[0].local!=64||terminal.arms[0].stop.is_some(){return Err(bad("FULL_FIT normal F64 required"));}
    let review:binary::Value=read(review_b)?;
    if review["phase"]!="B"||review["verdict"]!="PASS"||review["policy"]!=digest(&old)?||review["source"]!=old.source||review["runtime"]!=binary::record!(old.runtime){return Err(bad("FULL_FIT independent predecessor B binding"));}
    let p=&terminal.arms[0];let l=checkpoint::load(&p.native,Device::Cpu,true)?;
    let state=l.manifest.training.as_ref().ok_or_else(||bad("FULL_FIT parent training missing"))?;
    let op=l.manifest.optimizer_protocol.clone().ok_or_else(||bad("FULL_FIT inherited Adam missing"))?;
    let mut expected=old.event.as_ref().unwrap().optimizer.clone();expected.version=2;expected.family="ADAMW-INHERITED-V1".into();expected.study_start_step=Some(old.parent_step);expected.runtime_digest=digest(&old.runtime)?;expected.local_step=576;
    if state.step!=14912||state.sampler_state!=64||op!=expected||state.config!=old.config||state.resume_binding!=Some(bind(&old,0,state,&l.tokenizer)?)
        ||l.tokenizer.semantic_id()!=old.tokenizer||l.model.adapter().is_some(){return Err(bad("FULL_FIT parent weights/Adam/clock/objective"));}
    let c=event_inputs(&old,0)?;let (original,tm,dm)=inputs(&old)?;let tok=&l.tokenizer;
    let mut identities=vec![];
    for(i,e)in c.train.iter().chain(&c.validation).enumerate(){let (at,is_train)=if i<c.train.len(){(i,true)}else{(i-c.train.len(),false)};
        let source=if is_train{&original.train[at]}else{&original.validation[at]};
        let expected=if (is_train&&at>=6144)||(!is_train&&at>=3072){event_episode(source,false)?}else{source.clone()};
        if digest(e)?!=digest(&expected)?{return Err(bad("FULL_FIT frozen FULL/review content"));}
        let identity=fit_identity(e,tok)?;if is_train{identities.push(identity);}
    }
    let e=old.event.as_ref().unwrap();let muon:Study=read_confirmed(&e.original)?;
    let wp:Plan=read(&old.word_root.join("plan.r3b"))?;
    let rows=&wp.identifiable.as_ref().ok_or_else(||bad("FULL_FIT original native tape absent"))?.rows;
    let source=rows.get(wp.origin_step()..wp.origin_step()+3072).ok_or_else(||bad("FULL_FIT original3072 absent"))?;
    if source[..1024]!=muon.tape||old.tape!=source[512..640]||wp.origin_step()!=14336{return Err(bad("BLOCKED_INPUT_LINEAGE: FULL_FIT source tape"));}
    let mut prior_full=vec![0usize;7680];let mut seen=0usize;
    for index in 0..h.len(){let path=old.root.join("F").join(format!("updates-{index:03}.r3rows"));if !path.exists(){continue;}
        for row in binary::read_value_records(&path)?{if seen>=64||row["rows"]!=binary::record!(source[512+seen])||row["local"]!=seen+1||row["optimizer_local"]!=513+seen{return Err(bad("FULL_FIT prior actual tape/clock"));}
            for &i in &source[512+seen]{prior_full[i]+=1;}seen+=1;
        }}
    if seen!=64||prior_full[6144..].iter().filter(|&&n|n==1).count()!=256||prior_full[6144..].iter().any(|&n|n>1){return Err(bad("FULL_FIT prior exact exposure"));}
    for i in 6144..7680{if identities[i]==fit_identity(&original.train[i],tok)?{return Err(bad("FULL_FIT semantic-only parent has identical prompt"));}}
    let tape=fit_rotation(source,576)?;let costs=fit_costs(&c,tok,&tape)?;
    if fit_exposure(&tape)?[6144..].iter().any(|&n|n!=8){return Err(bad("FULL_FIT word cycle exposure"));}
    let order=fit_train_order(&c,&tm)?;
    let mut retained=BTreeMap::new();let mut refs=e.refs.clone();
    for panel in event_panels(&old,true,false)?.into_iter().filter(|p|!p.0.starts_with("ID_ONLY")){
        let(score,_)=event_read_panel(&old,"F",64,&l.model.weights_content_id()?,&panel,panel.1.len())?;
        retained.insert(panel.0.clone(),score);
        for suffix in [".r3rows".to_owned(),format!("-score-{}.r3b",panel.1.len())]{let path=old.root.join("F").join(format!("eval-64-{}{suffix}",panel.0));refs.insert(path.clone(),file_hash(&path)?);}
    }
    for name in ["value","citation","S1Q1"]{let j=&retained[name]["joint"];if j["full"]!=64||j["all4"]!=16||j["errors"]!=0{return Err(bad("FULL_FIT parent retention changed"));}}
    let terminal_path=old.root.join(format!("segment-{:03}-finished.r3b",h.len()-1));
    for path in [old.root.join("plan.r3b"),terminal_path.clone(),review_b.canonicalize()?,p.native.clone()]{refs.insert(path.clone(),file_hash(&path)?);}
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;let mut same=old.runtime.clone();same.binary=runtime.binary.clone();if runtime!=same{return Err(bad("FULL_FIT runtime changed"));}
    // Six scheduled natives plus two reserved time-split saves, without copying
    // the inherited native, corpus or vendor into this study.
    let planned_native_bytes=std::fs::metadata(&p.native)?.len().checked_mul(8).ok_or_else(||bad("FULL_FIT byte overflow"))?;
    if planned_native_bytes+64*1024*1024>1<<30{return Err(bad("BLOCKED_DISK: FULL_FIT immutable reservation"));}
    std::fs::create_dir(output)?;let root=output.canonicalize()?;std::fs::create_dir(root.join("F"))?;
    let mut s=old.clone();s.contract=FIT_CONTRACT.into();s.root=root.clone();s.parent=p.native.clone();s.parent_hash=p.physical.clone();s.parent_content=l.model.weights_content_id()?;s.parent_adam=optimizer_hash(&l.optimizer)?;s.parent_step=14912;s.source=sources()?;s.runtime=runtime;
    s.config.budget_start_step=14912;s.config.max_steps=17984;s.config.budget_start_tokens=state.consumed_tokens;s.config.max_tokens=state.consumed_tokens+costs["input"].as_u64().unwrap();s.tape=tape;s.costs=costs;
    s.max_updates=3072;s.generation_cap=8448;s.teacher_cap=144;s.active_cap=7200.;s.bytes_cap=1<<30;
    let event=s.event.as_mut().unwrap();event.optimizer=op;event.refs=refs;event.retention=retained;
    s.full_fit=Some(FullFit{predecessor:old.root.clone(),predecessor_hash:file_hash(&old.root.join("plan.r3b"))?,terminal_hash:file_hash(&terminal_path)?,source_tape_hash:digest(&source)?,source_offset:576,train_order:order,prior_full_exposure:prior_full,semantic_other_exposure:e.parent_exposure.clone(),exact_identities:identities,schedule:FIT_STEPS.to_vec()});
    fit_verify_inputs(&s)?;
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"contract":s.contract,"policy":digest(&s)?,"source":s.source,"parent":s.parent_hash,"weights":s.parent_content,"adam":s.parent_adam,"model_step":14912,"optimizer_step":576,"cursor":0,"data_counts":[c.train.len(),c.validation.len(),tm.len(),dm.len()],"full_fit":s.full_fit,"costs":s.costs,"planned_native_bytes":planned_native_bytes,"model_calls":0}))?;
    println!("FULL_FIT_PREPARED policy={} parent={} model14912 Adam576 cursor0 source576..3071/0..575 input={} target={} padding={} reserved_native_bytes={planned_native_bytes} calls0 A_PENDING",digest(&s)?,s.parent_hash,s.costs["input"],s.costs["target"],s.costs["padding"]);Ok(())
}
fn fit_verify_inputs(s:&Study)->Result<()> {
    let f=s.full_fit.as_ref().ok_or_else(||bad("FULL_FIT profile absent"))?;
    let old:Study=read_confirmed(&f.predecessor.join("plan.r3b"))?;
    if file_hash(&f.predecessor.join("plan.r3b"))?!=f.predecessor_hash||old.full_fit.is_some()||old.contract!=EVENT_CONTRACT||s.parent_step!=14912
        ||s.event.as_ref().unwrap().optimizer.local_step!=576||s.arms()!=["F"]||s.max_updates!=3072||f.source_offset!=576||f.schedule!=FIT_STEPS
        ||s.generation_cap!=8448||s.teacher_cap!=144||s.active_cap!=7200.||s.config.lr!=3e-5||s.config.warmup!=0{return Err(bad("FULL_FIT policy/parent bounds"));}
    let h=history(&old)?;let terminal=h.last().ok_or_else(||bad("FULL_FIT predecessor terminal missing"))?;
    if !terminal.success||terminal.resume||terminal.arms[0].local!=64||terminal.arms[0].physical!=s.parent_hash
        ||file_hash(&old.root.join(format!("segment-{:03}-finished.r3b",h.len()-1)))?!=f.terminal_hash{return Err(bad("FULL_FIT predecessor terminal changed"));}
    let p:Plan=read(&s.word_root.join("plan.r3b"))?;let rows=&p.identifiable.as_ref().ok_or_else(||bad("FULL_FIT source tape missing"))?.rows;
    let source=rows.get(p.origin_step()..p.origin_step()+3072).ok_or_else(||bad("FULL_FIT source tape extent"))?;
    if digest(&source)?!=f.source_tape_hash||s.tape!=fit_rotation(source,f.source_offset)?{return Err(bad("FULL_FIT frozen rotated tape changed"));}
    let c=event_inputs(s,0)?;let (_,tm,_)=inputs(s)?;let tok=scoring_tokenizer(s)?;
    if f.train_order!=fit_train_order(&c,&tm)?||f.exact_identities.len()!=c.train.len()||fit_exposure(&source[512..576])?!=f.prior_full_exposure{return Err(bad("FULL_FIT selection/exposure mismatch"));}
    for(e,id)in c.train.iter().zip(&f.exact_identities){if fit_identity(e,&tok)?!=*id{return Err(bad("FULL_FIT prompt-target identity changed"));}}
    if fit_costs(&c,&tok,&s.tape)?!=s.costs{return Err(bad("FULL_FIT token costs changed"));}Ok(())
}
fn first_roles(c:&data::native::Corpus,tm:&[Meta])->Result<Vec<u8>>{
    if c.train.len()!=7680||tm.len()!=c.train.len(){return Err(bad("first-decision train role cardinality"));}
    let mut roles=Vec::with_capacity(c.train.len());
    for (i,m) in tm.iter().enumerate(){
        let word=i>=6144;
        if word!=m.template.contains("/id") || (word&&m.template.split_once("/id").is_none()){
            return Err(bad("first-decision FULL word metadata role"));
        }
        roles.push(u8::from(word));
    }
    Ok(roles)
}
fn first_prepare(original:&Path,output:&Path)->Result<()> {
    if !output.file_name().is_some_and(|n|n.to_string_lossy().starts_with("first-decision-20260925-")){
        return Err(bad("first-decision output cost scope name"));
    }
    let old=load_study(original,false)?;
    if old.contract!=FIT_CONTRACT||old.first_decision.is_some(){return Err(bad("first-decision original FULL fit required"));}
    let h=history(&old)?;let end=h.last().ok_or_else(||bad("first-decision original terminal missing"))?;
    let p=&end.arms[0];let trace=fit_trace(&old,&h)?;
    let next=(576+3069)%3072;
    if next!=573||p.local!=3069||end.resume||end.success||trace["updates"]!=3069||trace["backward"]!=3072
        ||trace["source_last_index"]!=572||file_hash(&p.native)?!=p.physical{
        return Err(bad("first-decision original terminal/tape not verified"));
    }
    let l=checkpoint::load(&p.native,Device::Cpu,true)?;
    let st=l.manifest.training.as_ref().ok_or_else(||bad("first-decision parent state"))?;
    let op=l.manifest.optimizer_protocol.as_ref().ok_or_else(||bad("first-decision parent Adam"))?;
    if st.step!=17981||op.local_step!=3645||st.resume_binding!=Some(bind(&old,0,st,&l.tokenizer)?)
        ||p.physical!="f61db3b873859f2aac5083ace36385067f8cbb4d250cdd4d364d0eea289ea55c"
        ||l.model.adapter().is_some(){return Err(bad("first-decision exact parent native"));}
    let c=event_inputs(&old,0)?;let(_,tm,_)=inputs(&old)?;
    let registration:binary::Value=read_confirmed(&old.root.with_extension("full-fit-posthoc.r3b"))?;
    let posthoc=PathBuf::from(registration["output"].as_str().ok_or_else(||bad("first-decision posthoc registration"))?);
    let(d,view)=fit_posthoc_load(&posthoc,false)?;
    if d.original!=old.root||observation_history(&view,"posthoc")?.last().is_none_or(|r|r["success"]!=true){
        return Err(bad("first-decision completed posthoc required"));
    }
    let posthoc_plan_hash=file_hash(&posthoc.join("posthoc-plan.r3b"))?;
    let roles=first_roles(&c,&tm)?;
    let tape=(0..256).map(|i|old.tape[(3069+i)%3072]).collect::<Vec<_>>();
    if tape.first()!=Some(&old.tape[3069])||tape.len()!=256{ return Err(bad("first-decision tape suffix")); }
    let costs=fit_costs(&c,&l.tokenizer,&tape)?;
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let mut same=old.runtime.clone();same.binary=runtime.binary.clone();if runtime!=same{return Err(bad("first-decision runtime changed"));}
    let native_bytes=std::fs::metadata(&p.native)?.len();
    let planned_native_bytes=native_bytes.checked_mul(8).ok_or_else(||bad("first-decision byte reserve overflow"))?;
    let binary_bytes=std::fs::metadata(std::env::current_exe()?)?.len();
    let anticipated_scope_bytes=planned_native_bytes.checked_add(binary_bytes*2+64*1024*1024).ok_or_else(||bad("first-decision scoped byte reserve overflow"))?;
    if anticipated_scope_bytes>1<<30{return Err(bad("BLOCKED_DISK: first-decision reserve includes evidence and binaries"));}
    std::fs::create_dir(output)?;let root=output.canonicalize()?;
    std::fs::create_dir(root.join("C"))?;std::fs::create_dir(root.join("W"))?;
    let terminal_path=old.root.join(format!("segment-{:03}-finished.r3b",h.len()-1));
    let mut s=old.clone();s.contract=FIRST_CONTRACT.into();s.root=root.clone();s.parent=p.native.clone();s.parent_hash=p.physical.clone();
    s.parent_content=l.model.weights_content_id()?;s.parent_adam=optimizer_hash(&l.optimizer)?;s.parent_step=17981;
    s.source=sources()?;s.runtime=runtime;s.config.budget_start_step=17981;s.config.max_steps=18237;
    s.config.budget_start_tokens=st.consumed_tokens;s.config.max_tokens=st.consumed_tokens+costs["input"].as_u64().unwrap();
    s.tape=tape;s.costs=costs;s.max_updates=256;s.generation_cap=2560;s.teacher_cap=208;s.active_cap=7200.;s.segment_cap=900.;s.bytes_cap=1<<30;
    s.event.as_mut().unwrap().optimizer=op.clone();
    s.first_decision=Some(FirstDecision{original:old.root.clone(),plan_hash:file_hash(&old.root.join("plan.r3b"))?,
        terminal_hash:file_hash(&terminal_path)?,posthoc,posthoc_plan_hash,source_index:573,
        role_map:checkpoint::ResumeBinding::digest_bytes(&roles),backward_cap:264,trace_digest:digest(&trace)?});
    first_verify_inputs(&s)?;
    let teacher_train=first_teacher_cases(&s,true)?;let teacher_dev=first_teacher_cases(&s,false)?;
    first_review_teacher_positions(&teacher_train,&teacher_dev)?;
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"contract":s.contract,"policy":digest(&s)?,"source":s.source,
        "parent":s.parent_hash,"weights":s.parent_content,"adam":s.parent_adam,"model_step":17981,"optimizer_step":3645,
        "cursor":0,"source_tape_next":573,"roles":s.first_decision.as_ref().unwrap().role_map,"costs":s.costs,
        "planned_native_bytes":planned_native_bytes,"anticipated_scope_bytes":anticipated_scope_bytes,"planned_native_saves":8,
        "reserved_executables":2,"reserved_other_bytes":64*1024*1024,"max_updates_per_arm":256,"max_backward_per_arm":264,"model_calls":0}))?;
    println!("FIRST_PREPARED policy={} parent={} model17981 Adam3645 next573 input={} target={} padding={} planned_native_bytes={} anticipated_scope_bytes={} calls0 A_PENDING",digest(&s)?,s.parent_hash,s.costs["input"],s.costs["target"],s.costs["padding"],planned_native_bytes,anticipated_scope_bytes);Ok(())
}
fn first_verify_inputs(s:&Study)->Result<()> {
    let f=s.first_decision.as_ref().ok_or_else(||bad("first-decision policy missing"))?;
    if s.contract!=FIRST_CONTRACT||s.arms()!=["C","W"]||s.max_updates!=256||f.backward_cap!=264||f.source_index!=573
        ||!s.root.file_name().is_some_and(|n|n.to_string_lossy().starts_with("first-decision-20260925-"))
        ||s.parent_step!=17981||s.clock(0)!=3645||s.config.lr!=3e-5||s.config.warmup!=0
        ||s.generation_cap!=2560||s.teacher_cap!=208||s.active_cap!=7200.||s.segment_cap!=900.
        ||s.bytes_cap!=1<<30{return Err(bad("first-decision fixed policy"));}
    let old:Study=read_confirmed(&f.original.join("plan.r3b"))?;
    if file_hash(&f.original.join("plan.r3b"))?!=f.plan_hash||old.contract!=FIT_CONTRACT||old.first_decision.is_some()
        ||file_hash(&s.parent)?!=s.parent_hash{return Err(bad("first-decision original binding"));}
    let h=history(&old)?;let last=h.last().ok_or_else(||bad("first-decision original history"))?;
    if digest(&fit_trace(&old,&h)?)?!=f.trace_digest{return Err(bad("first-decision original trace changed"));}
    let terminal=f.original.join(format!("segment-{:03}-finished.r3b",h.len()-1));
    if file_hash(&terminal)?!=f.terminal_hash||last.arms[0].local!=3069||last.arms[0].physical!=s.parent_hash
        ||last.resume||last.success||s.tape!=(0..256).map(|i|old.tape[(3069+i)%3072]).collect::<Vec<_>>(){return Err(bad("first-decision original cursor/tape"));}
    let c=event_inputs(s,0)?;let(_,tm,_)=inputs(s)?;let tok=scoring_tokenizer(s)?;
    if checkpoint::ResumeBinding::digest_bytes(&first_roles(&c,&tm)?)!=f.role_map||fit_costs(&c,&tok,&s.tape)?!=s.costs{
        return Err(bad("first-decision role map/costs"));
    }
    if file_hash(&f.posthoc.join("posthoc-plan.r3b"))?!=f.posthoc_plan_hash{return Err(bad("first-decision posthoc plan changed"));}
    Ok(())
}
fn first_eval_trace(s:&Study,h:&[Segment])->Result<binary::Value>{
    let c=event_inputs(s,0)?;let tok=scoring_tokenizer(s)?;
    let samples=samples_with_framing(&c.train,&tok,s.config.seq_len,neural::Framing::QuestionEvidence)?;
    let mut arms=BTreeMap::new();
    for arm in 0..2{let mut count=0usize;let(mut input,mut target,mut padding,mut denominator)=(0usize,0usize,0usize,0usize);
        for index in 0..h.len(){let path=s.root.join(s.arms()[arm]).join(format!("updates-{index:03}.r3rows"));
            if !path.exists(){continue;}
            for row in binary::read_value_records(&path)?{let draw=s.tape.get(count).ok_or_else(||bad("first-eval trace overrun"))?;
                let length=draw.iter().map(|&i|samples[i].tokens.len()-1).max().unwrap();let mut cost=[0usize;3];
                for &i in draw{cost[0]+=samples[i].tokens.len()-1;cost[1]+=samples[i].tokens.len()-samples[i].response_start;
                    cost[2]+=length-(samples[i].tokens.len()-1);}
                count+=1;
                if row["local"]!=count||row["model_step"]!=17981+count||row["optimizer_local"]!=3645+count
                    ||row["source_tape_index"]!=(573+count-1)%3072||row["rows"]!=binary::record!(draw)
                    ||row["input"]!=cost[0]||row["target"]!=cost[1]||row["padding"]!=cost[2]
                    ||row["weighted_denominator"]!=cost[1]+if arm==1{4}else{0}
                    ||row["examples"]!=8||["plain_ce","first_nll","answer_ce"].iter().any(|key|row[*key].as_f64().is_none_or(|v|!v.is_finite())){
                    return Err(bad("first-eval trace/clock/cost/objective"));
                }
                input+=cost[0];target+=cost[1];padding+=cost[2];denominator+=cost[1]+if arm==1{4}else{0};
            }
        }
        if count!=256||first_discarded(s,arm)?!=[0;3]||input!=411360||target!=29952||padding!=65824
            ||denominator!=29952+if arm==1{1024}else{0}{return Err(bad("first-eval exact committed exposure/cost"));}
        arms.insert(s.arms()[arm].to_owned(),binary::record!({"committed":count,"backward":count,"input":input,"target":target,
            "padding":padding,"weighted_denominator":denominator,"first_source_index":573,"last_source_index":828}));
    }
    Ok(binary::record!(arms))
}
fn first_eval_origin(root:&Path)->Result<(Study,Vec<Progress>,binary::Value,(f64,usize,usize),BTreeMap<PathBuf,String>)>{
    let s=load_study(root,false)?;let h=history(&s)?;let end=h.last().ok_or_else(||bad("first-eval original terminal missing"))?;
    if s.root.file_name().is_none_or(|n|n!="first-decision-20260925-study-v7")||h.len()!=3||end.phase!=2
        ||end.success||end.resume||end.arms.len()!=2||end.arms.iter().any(|p|p.local!=256||p.fit||p.stop.is_some()||p.evaluated!=128)
        ||end.error.as_deref()!=Some("invalid input: fresh: first-decision teacher two-ID group")
        ||end.control["observed_conditions"]!=binary::record!(["INTEGRITY_FAIL"])
        ||end.arms[0].physical!="1f5656d5c033a5cba3d271c3939aed66aaa14f4a00b11cc30f8081af5db28a90"
        ||end.arms[1].physical!="c7d9e767646173815b8c8afd45299067723512ada87b3a61beea1ee25830c3bd"{
        return Err(bad("first-eval exact failed v7 endpoint only"));
    }
    let trace=first_eval_trace(&s,&h)?;let tok=scoring_tokenizer(&s)?;let mut refs=BTreeMap::new();
    for path in [s.root.join("plan.r3b"),s.root.join("segment-002-finished.r3b"),end.arms[0].native.clone(),end.arms[1].native.clone()]{refs.insert(path.clone(),file_hash(&path)?);}
    for arm in 0..2{let p=&end.arms[arm];let(m,_)=checkpoint::metadata(&p.native)?;
        let st=m.training.as_ref().ok_or_else(||bad("first-eval native training state"))?;
        let op=m.optimizer_protocol.as_ref().ok_or_else(||bad("first-eval native Adam"))?;op.validate(&m.architecture,st)?;
        if st.step!=18237||st.sampler_state!=256||st.config!=s.config||op.local_step!=3901
            ||st.resume_binding!=Some(bind(&s,arm,st,&tok)?)||file_hash(&p.native)?!=p.physical{
            return Err(bad("first-eval native clock/objective/source"));
        }
        let at128=h.iter().find_map(|seg|seg.arms.get(arm).filter(|q|q.local==128&&q.evaluated==128))
            .ok_or_else(||bad("first-eval +128 saved endpoint"))?;
        refs.insert(at128.native.clone(),file_hash(&at128.native)?);
        let model128=checkpoint::metadata(&at128.native)?.0.model_content_digest;
        for panel in fit_panels(&s)?.into_iter().take(5){event_read_panel(&s,s.arms()[arm],128,&model128,&panel,64)
            .map_err(|e|bad(&format!("first-eval old +128 {} {}: {e}",s.arms()[arm],panel.0)))?;
            for suffix in [".r3rows".to_owned(),"-score-64.r3b".into()]{let path=s.root.join(s.arms()[arm]).join(format!("eval-128-{}{suffix}",panel.0));refs.insert(path.clone(),file_hash(&path)?);}
        }
        let decision=s.root.join(s.arms()[arm]).join("decision-128.r3b");refs.insert(decision.clone(),file_hash(&decision)?);
        if arm==0{for panel in fit_panels(&s)?.into_iter().take(6){let n=if ["FULL-word","FULL-renamed","FULL-train"].contains(&panel.0.as_str()){192}else{64};
                event_read_panel(&s,"C",256,&m.model_content_digest,&panel,n)
                    .map_err(|e|bad(&format!("first-eval old C final {}: {e}",panel.0)))?;
                for suffix in [".r3rows".to_owned(),format!("-score-{n}.r3b")]{let path=s.root.join("C").join(format!("eval-256-{}{suffix}",panel.0));refs.insert(path.clone(),file_hash(&path)?);}
            }}
        else{for panel in fit_panels(&s)?.into_iter().take(6){let path=s.root.join("W").join(format!("eval-256-{}.r3rows",panel.0));
            if path.exists()||pending_path(&path).exists(){return Err(bad("first-eval original W final unexpectedly started"));}}}
        let teacher=s.root.join(s.arms()[arm]).join("teacher-256-FULL-teachers.r3rows");
        if teacher.exists()||pending_path(&teacher).exists(){return Err(bad("first-eval original teacher unexpectedly started"));}
    }
    let a:binary::Value=read_confirmed(&s.root.join("review-a.r3b"))?;let report=Path::new(a["report_path"].as_str().ok_or_else(||bad("first-eval original A report"))?);
    if a["accepted"]!=true||a["policy"]!=digest(&s)?||a["source"]!=s.source||a["report_hash"]!=file_hash(report)?{
        return Err(bad("first-eval original independent A identity"));
    }
    let mut usage=(a["active_seconds"].as_f64().filter(|n|n.is_finite()&&*n>=0.).ok_or_else(||bad("first-eval A time UNKNOWN"))?,0usize,0usize);
    let mut controls=observation_history(&s,"baseline")?.into_iter().map(|r|r["control"].clone()).collect::<Vec<_>>();
    controls.extend(h.iter().map(|r|r.control.clone()));
    for (i,c) in controls.iter().enumerate(){if c["observed_conditions"]!=if i==controls.len()-1{binary::record!(["INTEGRITY_FAIL"])}else{binary::record!([])}
            &&!(i!=controls.len()-1&&c["observed_conditions"]==binary::record!(["TIME_BUDGET"])){
            return Err(bad("first-eval mixed original error"));
        }
        usage.0+=c["elapsed_seconds"].as_f64().filter(|v|v.is_finite()&&*v>=0.).ok_or_else(||bad("first-eval original time UNKNOWN"))?;
        usage.1+=c["generation_calls"].as_u64().ok_or_else(||bad("first-eval original generation UNKNOWN"))?as usize;
        usage.2+=c["teacher_calls"].as_u64().ok_or_else(||bad("first-eval original teacher UNKNOWN"))?as usize;
    }
    if usage.1!=1408||usage.2!=0||usage.0>=s.active_cap{return Err(bad("first-eval original charged usage"));}
    let train=first_teacher_cases(&s,true)?;let dev=first_teacher_cases(&s,false)?;first_review_teacher_positions(&train,&dev)?;
    Ok((s,end.arms.clone(),trace,usage,refs))
}
fn first_eval_output(original:&Path,output:&Path)->Result<PathBuf>{
    let requested=output.parent().ok_or_else(||bad("first-eval output parent"))?.canonicalize()?.join(output.file_name().ok_or_else(||bad("first-eval output name"))?);
    let expected=original.parent().ok_or_else(||bad("first-eval original parent"))?.join("first-decision-20260925-eval-only-01");
    if requested!=expected||requested.try_exists()?||pending_path(&requested).exists(){return Err(bad("first-eval one new sibling scope only"));}
    Ok(requested)
}
fn first_eval_prepare(original:&Path,output:&Path)->Result<()> {
    let(s,endpoints,trace,usage,refs)=first_eval_origin(original)?;
    let requested=first_eval_output(&s.root,output)?;
    if usage.1+768+64>s.generation_cap||usage.2+192+16>s.teacher_cap||usage.0>=s.active_cap{return Err(bad("first-eval aggregate budget insufficient"));}
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let mut old=s.runtime.clone();old.binary=runtime.binary.clone();if old!=runtime{return Err(bad("first-eval runtime changed beyond executable"));}
    let original_bytes=owned_bytes(&s.root)?;guard_bytes(&s,32*1024*1024)?;
    let registration=s.root.with_extension("first-eval-only.r3b");
    if registration.exists()||pending_path(&registration).exists(){return Err(bad("first-eval scope already registered"));}
    publish_confirmed(&registration,&binary::record!({"contract":FIRST_EVAL_ONLY,"original_plan":file_hash(&s.root.join("plan.r3b"))?,
        "terminal":file_hash(&s.root.join("segment-002-finished.r3b"))?,"output":requested}))?;
    std::fs::create_dir(output)?;let root=output.canonicalize()?;for arm in ["C","W"]{std::fs::create_dir(root.join(arm))?;}
    let d=FirstEvalOnly{contract:FIRST_EVAL_ONLY.into(),mode:"EVAL_ONLY".into(),root:root.clone(),original:s.root.clone(),
        plan_hash:file_hash(&s.root.join("plan.r3b"))?,terminal_hash:file_hash(&s.root.join("segment-002-finished.r3b"))?,
        endpoints,trace_digest:digest(&trace)?,prior_usage:usage,refs,original_bytes,source:sources()?,runtime};
    publish_confirmed(&root.join("eval-only-plan.r3b"),&d)?;
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"contract":FIRST_EVAL_ONLY,"mode":"EVAL_ONLY","policy":digest(&d)?,"source":d.source,
        "original":d.original,"original_terminal":d.terminal_hash,"native":[d.endpoints[0].physical,d.endpoints[1].physical],
        "model_step":18237,"Adam":3901,"committed_training":512,"new_optimizer":0,"new_backward":0,
        "prior_usage":d.prior_usage,"remaining_main_generation":768,"remaining_main_teacher":192,
        "reserved_B_generation":64,"reserved_B_teacher":16,"new_native_copies":0,"model_calls":0}))?;
    println!("FIRST_EVAL_PREPARED policy={} source={} original_failed=true nativeC={} nativeW={} prior={:?} new_calls0",digest(&d)?,d.source,d.endpoints[0].physical,d.endpoints[1].physical,d.prior_usage);Ok(())
}
fn first_eval_load(root:&Path,execute:bool)->Result<(FirstEvalOnly,Study,Study)>{
    let d:FirstEvalOnly=read_confirmed(&root.join("eval-only-plan.r3b"))?;
    if d.contract!=FIRST_EVAL_ONLY||d.mode!="EVAL_ONLY"||d.root!=root.canonicalize()?||d.root==d.original{return Err(bad("first-eval root/contract/mode"));}
    let(s,endpoints,trace,usage,refs)=first_eval_origin(&d.original)?;
    let registration=s.root.with_extension("first-eval-only.r3b");
    let registered:binary::Value=read_confirmed(&registration)?;
    if registered!=binary::record!({"contract":FIRST_EVAL_ONLY,"original_plan":d.plan_hash,"terminal":d.terminal_hash,"output":d.root})
        ||d.plan_hash!=file_hash(&s.root.join("plan.r3b"))?||d.terminal_hash!=file_hash(&s.root.join("segment-002-finished.r3b"))?
        ||digest(&d.endpoints)?!=digest(&endpoints)?||d.trace_digest!=digest(&trace)?||d.prior_usage!=usage||d.refs!=refs
        ||d.original_bytes!=owned_bytes(&s.root)?{return Err(bad("first-eval original evidence changed"));}
    for(path,hash)in &d.refs{if file_hash(path)?!=*hash||pending_path(path).exists(){return Err(bad("first-eval original raw/native changed"));}}
    let (executor_source,executor_runtime,executor_hash)=first_eval_executor(&d)?;
    let mut view=s.clone();view.root=d.root.clone();view.source=executor_source;view.runtime=executor_runtime;
    if execute{
        let a:binary::Value=read_confirmed(&root.join("eval-only-admission.r3b"))?;
        let path=Path::new(a["report"].as_str().ok_or_else(||bad("first-eval original admission absent"))?);
        if a["policy"]!=digest(&d)?||a["executor_hash"]!=binary::record!(executor_hash)
            ||a["report_hash"]!=file_hash(path)?||a["accepted"]!=true{return Err(bad("first-eval original admission changed"));}
        let(source,runtime,post_hash)=first_eval_post_main(&d,&s,&view)?;
        if source!=sources()?{return Err(bad("first-eval post-main source changed"));}
        runtime.verify(&Backend::Metal0.open()?)?;
        let admitted:binary::Value=read_confirmed(&root.join("eval-only-post-main-admission.r3b"))?;
        let receipt=Path::new(admitted["report"].as_str().ok_or_else(||bad("first-eval post-main independent B absent"))?);
        if admitted["policy"]!=digest(&d)?||admitted["post_main_executor_hash"]!=post_hash
            ||admitted["report_hash"]!=file_hash(receipt)?||admitted["accepted"]!=true{
            return Err(bad("first-eval post-main B admission changed"));
        }
    }
    Ok((d,s,view))
}
fn first_eval_executor(d:&FirstEvalOnly)->Result<(String,RuntimeProfile,Option<String>)>{
    let path=d.root.join("eval-only-executor.r3b");
    if !path.exists(){return Ok((d.source.clone(),d.runtime.clone(),None));}
    let e:binary::Value=read_confirmed(&path)?;
    let source=e["source"].as_str().ok_or_else(||bad("first-eval executor source"))?.to_owned();
    let runtime:RuntimeProfile=binary::from_value(e["runtime"].clone())?;
    if e["contract"]!=FIRST_EVAL_ONLY||e["mode"]!="EVAL_ONLY"||e["policy"]!=digest(d)?
        ||e["prepared_source"]!=d.source||e["prepared_runtime"]!=binary::record!(d.runtime)
        ||e["reason"]!="main-report-usage-sticky"||e["model_calls"]!=0||source==d.source{
        return Err(bad("first-eval executor revision identity"));
    }
    let mut old=d.runtime.clone();old.binary=runtime.binary.clone();
    if old!=runtime{return Err(bad("first-eval executor runtime changed beyond binary"));}
    Ok((source,runtime,Some(file_hash(&path)?)))
}
fn first_eval_revise(root:&Path)->Result<()> {
    let(d,_,_)=first_eval_load(root,false)?;
    if d.root.join("eval-only-executor.r3b").exists()||d.root.join("eval-only-admission.r3b").exists()
        ||sources()?==d.source{return Err(bad("first-eval revision unavailable"));}
    for item in std::fs::read_dir(&d.root)?{let item=item?;let name=item.file_name();let name=name.to_string_lossy();
        if name=="C"||name=="W"{if std::fs::read_dir(item.path())?.next().is_some(){return Err(bad("first-eval calls before revision"));}}
        else if name!="eval-only-plan.r3b"&&name!="preparation.r3b"{
            return Err(bad("first-eval revision only before observations/admission"));
        }
    }
    let runtime=RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?;
    let mut old=d.runtime.clone();old.binary=runtime.binary.clone();if old!=runtime{return Err(bad("first-eval revision runtime"));}
    publish_confirmed(&d.root.join("eval-only-executor.r3b"),&binary::record!({"contract":FIRST_EVAL_ONLY,"mode":"EVAL_ONLY",
        "policy":digest(&d)?,"prepared_source":d.source,"prepared_runtime":d.runtime,"source":sources()?,"runtime":runtime,
        "reason":"main-report-usage-sticky","model_calls":0}))?;
    println!("FIRST_EVAL_REVISED policy={} executor={} calls0",digest(&d)?,file_hash(&d.root.join("eval-only-executor.r3b"))?);Ok(())
}
fn first_eval_admit(root:&Path,review:&Path)->Result<()> {
    if review!=root.join("eval-only-boundary-review.r3b")||pending_path(review).exists(){
        return Err(bad("first-eval independent B immutable phase receipt path"));
    }
    let(d,_,view)=first_eval_load(root,false)?;let r:binary::Value=read(review)?;
    let executor_hash=first_eval_executor(&d)?.2.ok_or_else(||bad("first-eval revised executor missing"))?;
    if r["contract"]!=FIRST_EVAL_ONLY||r["policy"]!=digest(&d)?||r["source"]!=view.source||r["runtime"]!=binary::record!(view.runtime)
        ||r["executor_hash"]!=executor_hash
        ||r["verdict"]!="PASS"||r["boundaries"]!=binary::record!(["immutable_failed_origin","zero_optimizer","split_raw","aggregate_budget","returned_resume","dedup_B"]){
        return Err(bad("first-eval independent B boundary incomplete"));
    }
    publish_confirmed(&root.join("eval-only-admission.r3b"),&binary::record!({"policy":digest(&d)?,"executor_hash":executor_hash,"report":review.canonicalize()?,
        "report_hash":file_hash(review)?,"accepted":true}))?;Ok(())
}
fn first_eval_usage_labels(d:&FirstEvalOnly,s:&Study,labels:&[&str])->Result<(f64,usize,usize)>{
    let mut usage=d.prior_usage;
    for &label in labels{for r in observation_history(s,label)?{let c=&r["control"];
        if c["observed_conditions"].as_array().is_none_or(|v|v.iter().any(|x|x!="TIME_BUDGET")){return Err(bad("first-eval sticky successor error"));}
        usage.0+=c["elapsed_seconds"].as_f64().filter(|v|v.is_finite()&&*v>=0.).ok_or_else(||bad("first-eval time UNKNOWN"))?;
        usage.1+=c["generation_calls"].as_u64().ok_or_else(||bad("first-eval generation UNKNOWN"))?as usize;
        usage.2+=c["teacher_calls"].as_u64().ok_or_else(||bad("first-eval teacher UNKNOWN"))?as usize;
    }}
    if usage.0>s.active_cap||usage.1>s.generation_cap||usage.2>s.teacher_cap{return Err(bad("first-eval aggregate cap"));}Ok(usage)
}
fn first_eval_usage(d:&FirstEvalOnly,s:&Study)->Result<(f64,usize,usize)>{
    first_eval_usage_labels(d,s,&["eval-only","review-C","review-W"])
}
fn first_eval_split_usage(d:&FirstEvalOnly,main:&Study,review:&Study)->Result<(f64,usize,usize)>{
    let m=first_eval_usage_labels(d,main,&["eval-only"])?;
    let b=first_eval_usage_labels(d,review,&["review-C","review-W"])?;
    let usage=(m.0+(b.0-d.prior_usage.0),m.1+b.1-d.prior_usage.1,m.2+b.2-d.prior_usage.2);
    if usage.0>main.active_cap||usage.1>main.generation_cap||usage.2>main.teacher_cap{
        return Err(bad("first-eval aggregate post-main cap"));
    }
    Ok(usage)
}
fn first_eval_start(d:&FirstEvalOnly,main:&Study,s:&Study,label:&str,b:&binary::Value)->Result<(usize,RunControl)>{
    let(time,g,t)=first_eval_split_usage(d,main,s)?;guard_bytes(s,8*1024*1024)?;
    let cancel=std::sync::Arc::new(AtomicBool::new(false));let signal=cancel.clone();
    ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut ctl=RunControl::new(cancel,std::time::Duration::from_secs_f64((s.active_cap-time).max(0.).min(s.segment_cap)),12*1024*1024)?;
    ctl.set_call_limits(s.generation_cap-g,s.teacher_cap-t);
    start_observation_control(s,label,b,ctl)
}
fn first_eval_teacher_read(s:&Study,arm:usize,p:&Progress)->Result<Vec<binary::Value>>{
    let train=first_teacher_cases(s,true)?;let dev=first_teacher_cases(s,false)?;
    first_review_teacher_positions(&train,&dev)?;
    let es=train.iter().chain(&dev).cloned().collect::<Vec<_>>();
    let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
    let label=format!("teacher-{}-FULL",p.local);let dir=s.root.join(s.arms()[arm]);
    let b=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"model":model,
        "arm":s.arms()[arm],"local":p.local,"cases":digest(&es)?,"planned":96,"train":48,"dev":48,"call_protocol":1});
    let raw=binary::read_value_records(&dir.join(format!("{label}-teachers.r3rows")))?;
    if raw.len()!=97||raw[0]!=b{return Err(bad("first-eval teacher identity/count"));}
    for(i,(e,r))in es.iter().zip(&raw[1..]).enumerate(){call_attempt(&dir,&label,"teacher",&b,e,i,Some(r))?;}
    let tok=scoring_tokenizer(s)?;diagnosis::validate_teacher_forward(&s.runtime,s.config.seq_len,&es,&raw[1..],&tok)?;
    let score=binary::record!({"binding":b,"train":diagnosis::teacher_scores(&train,&raw[1..49],&tok)?,
        "dev":diagnosis::teacher_scores(&dev,&raw[49..],&tok)?,"rows_digest":digest(&&raw[1..])?});
    if read_confirmed::<binary::Value>(&dir.join(format!("{label}-score.r3b")))?!=score{
        return Err(bad("first-eval teacher independently recounted score"));
    }
    Ok(raw[1..].to_vec())
}
fn first_eval_main_refs(d:&FirstEvalOnly,original:&Study,main:&Study)->Result<BTreeMap<PathBuf,String>>{
    let h=observation_history(main,"eval-only")?;
    if h.len()!=1||h[0]["success"]!=true||h[0]["resume"]!=false
        ||h[0]["control"]["observed_conditions"]!=binary::record!([])
        ||h[0]["control"]["generation_calls"]!=768||h[0]["control"]["teacher_calls"]!=192{
        return Err(bad("first-eval exact completed main observation"));
    }
    let mut refs=BTreeMap::new();
    for name in ["eval-only-started.r3b","eval-only-segment-000-started.r3b","eval-only-segment-000-finished.r3b","eval-only-finished.r3b"]{
        let path=main.root.join(name);refs.insert(path.clone(),file_hash(&path)?);
    }
    for arm in 0..2{let src=if arm==0{original}else{main};let p=&d.endpoints[arm];
        let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
        for panel in fit_panels(main)?.into_iter().take(6){let full=["FULL-word","FULL-renamed","FULL-train"].contains(&panel.0.as_str());
            event_read_panel(src,src.arms()[arm],256,&model,&panel,if full{192}else{64})?;
            if full{event_read_panel(src,src.arms()[arm],256,&model,&panel,64)?;}
            let dir=src.root.join(src.arms()[arm]);let stem=format!("eval-256-{}",panel.0);
            for suffix in if full{vec![".r3rows","-score-64.r3b","-score-192.r3b"]}else{vec![".r3rows","-score-64.r3b"]}{
                let path=dir.join(format!("{stem}{suffix}"));refs.insert(path.clone(),file_hash(&path)?);
            }
        }
        first_eval_teacher_read(main,arm,p)?;
        for suffix in ["-teachers.r3rows","-score.r3b"]{
            let path=main.root.join(main.arms()[arm]).join(format!("teacher-256-FULL{suffix}"));
            refs.insert(path.clone(),file_hash(&path)?);
        }
    }
    if main.root.join("W/eval-256-OLD_FULL.r3rows").exists(){return Err(bad("first-eval unscheduled OLD_FULL"));}
    Ok(refs)
}
fn first_eval_post_main(d:&FirstEvalOnly,original:&Study,main:&Study)->Result<(String,RuntimeProfile,String)>{
    let path=d.root.join("eval-only-post-main-executor.r3b");let e:binary::Value=read_confirmed(&path)?;
    let source=e["source"].as_str().ok_or_else(||bad("first-eval post-main source"))?.to_owned();
    let runtime:RuntimeProfile=binary::from_value(e["runtime"].clone())?;
    let old_hash=first_eval_executor(d)?.2.ok_or_else(||bad("first-eval main executor absent"))?;
    let refs=first_eval_main_refs(d,original,main)?;
    let mut old=main.runtime.clone();old.binary=runtime.binary.clone();
    if e["contract"]!=FIRST_EVAL_ONLY||e["mode"]!="EVAL_ONLY"||e["policy"]!=digest(d)?
        ||e["main_executor_hash"]!=old_hash||e["main_source"]!=main.source||e["main_runtime"]!=binary::record!(main.runtime)
        ||e["main_finished_hash"]!=file_hash(&main.root.join("eval-only-finished.r3b"))?
        ||e["main_refs"]!=binary::record!(refs)||e["source"]!=source||e["runtime"]!=binary::record!(runtime)
        ||e["reason"]!="six-scheduled-panels-and-nested-teacher-parity"||e["new_optimizer"]!=0
        ||e["new_model_calls"]!=0||source==main.source||runtime!=old{
        return Err(bad("first-eval post-main revision identity/raw/runtime"));
    }
    Ok((source,runtime,file_hash(&path)?))
}
fn first_eval_post_main_revise(root:&Path)->Result<()> {
    let(d,original,main)=first_eval_load(root,false)?;
    if d.root.join("eval-only-post-main-executor.r3b").exists()||d.root.join("eval-only-post-main-admission.r3b").exists()
        ||d.root.join("review-C-started.r3b").exists()||d.root.join("review-W-started.r3b").exists()
        ||sources()?==main.source{return Err(bad("first-eval post-main revision unavailable"));}
    let old:binary::Value=read_confirmed(&root.join("eval-only-admission.r3b"))?;
    let old_hash=first_eval_executor(&d)?.2.ok_or_else(||bad("first-eval main executor absent"))?;
    let old_report=Path::new(old["report"].as_str().ok_or_else(||bad("first-eval old B receipt path"))?);
    if old["policy"]!=digest(&d)?||old["executor_hash"]!=old_hash||old["report_hash"]!=file_hash(old_report)?
        ||old["accepted"]!=true{return Err(bad("first-eval original admission changed"));}
    let refs=first_eval_main_refs(&d,&original,&main)?;
    let runtime=RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?;
    let mut expected=main.runtime.clone();expected.binary=runtime.binary.clone();
    if runtime!=expected{return Err(bad("first-eval post-main runtime changed beyond executable"));}
    publish_confirmed(&root.join("eval-only-post-main-executor.r3b"),&binary::record!({"contract":FIRST_EVAL_ONLY,
        "mode":"EVAL_ONLY","policy":digest(&d)?,"main_executor_hash":old_hash,"main_source":main.source,
        "main_runtime":main.runtime,"main_finished_hash":file_hash(&root.join("eval-only-finished.r3b"))?,
        "main_refs":refs,"source":sources()?,"runtime":runtime,
        "reason":"six-scheduled-panels-and-nested-teacher-parity","new_optimizer":0,"new_model_calls":0}))?;
    println!("FIRST_EVAL_POST_MAIN_REVISED policy={} executor={} calls0",digest(&d)?,file_hash(&root.join("eval-only-post-main-executor.r3b"))?);Ok(())
}
fn first_eval_post_main_admit(root:&Path,review:&Path)->Result<()> {
    if review!=root.join("eval-only-post-main-boundary-review.r3b")||pending_path(review).exists(){
        return Err(bad("first-eval post-main independent B immutable receipt path"));
    }
    let(d,original,main)=first_eval_load(root,false)?;
    let(source,runtime,post_hash)=first_eval_post_main(&d,&original,&main)?;
    let r:binary::Value=read(review)?;
    if r["contract"]!=FIRST_EVAL_ONLY||r["policy"]!=digest(&d)?||r["source"]!=source
        ||r["runtime"]!=binary::record!(runtime)||r["post_main_executor_hash"]!=post_hash
        ||r["verdict"]!="PASS"||r["boundaries"]!=binary::record!(["immutable_failed_origin","zero_optimizer",
            "split_raw","aggregate_budget","returned_resume","dedup_B","completed_main_raw","nested_teacher_parity"]){
        return Err(bad("first-eval post-main independent B incomplete"));
    }
    publish_confirmed(&root.join("eval-only-post-main-admission.r3b"),&binary::record!({"policy":digest(&d)?,
        "post_main_executor_hash":post_hash,"report":review.canonicalize()?,"report_hash":file_hash(review)?,"accepted":true}))?;Ok(())
}
fn first_eval_run(root:&Path,phase:&str)->Result<()> {
    let(d,original,view)=first_eval_load(root,true)?;
    let(source,runtime,_post_hash)=first_eval_post_main(&d,&original,&view)?;
    let mut review=view.clone();review.source=source;review.runtime=runtime;
    match phase {
        "evaluate"=>Err(bad("first-eval completed main observation cannot restart")),
        "report"=>first_eval_report(&d,&original,&view,&review),
        "review-c"=>first_eval_review(&d,&original,&view,&review,0),
        "review-w"=>first_eval_review(&d,&original,&view,&review,1),
        _=>Err(bad("first-eval phase")),
    }
}
fn first_eval_report(d:&FirstEvalOnly,original:&Study,view:&Study,review:&Study)->Result<()> {
    let h=observation_history(view,"eval-only")?;
    if h.last().is_none_or(|r|r["success"]!=true){return Err(bad("first-eval main observation incomplete"));}
    let mut scores=vec![];let panels=fit_panels(view)?;
    for arm in 0..2{let src=if arm==0{original}else{view};let p=&d.endpoints[arm];
        let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
        let mut scored=BTreeMap::new();
        for panel in panels.iter().take(6){let n=if ["FULL-word","FULL-renamed","FULL-train"].contains(&panel.0.as_str()){192}else{64};
            let(r,_)=event_read_panel(src,src.arms()[arm],p.local,&model,panel,n)?;scored.insert(panel.0.clone(),r);
        }
        first_eval_teacher_read(view,arm,p)?;scores.push(scored);
    }
    let parent=first_parent_scores(original)?;let mut quality=vec![];
    for arm in 0..2{let prior:binary::Value=read_confirmed(&original.root.join(original.arms()[arm]).join("decision-128.r3b"))?;
        if prior["policy"]!=digest(original)?||!prior["stop"].is_null(){return Err(bad("first-eval prior regular guard"));}
        let mut severe=false;let mut persistent=false;
        for name in ["value","citation","S1Q1","FULL-word","FULL-renamed"]{
            let panel=panels.iter().find(|p|p.0==name).ok_or_else(||bad("first-eval guard panel"))?;
            let src=if arm==0{original}else{view};let model=checkpoint::metadata(&d.endpoints[arm].native)?.0.model_content_digest;
            let short=event_read_panel(src,src.arms()[arm],256,&model,panel,64)?.0;
            let cur=short["joint"]["full"].as_u64().ok_or_else(||bad("first-eval guard FULL"))?;
            let base=parent[name]["joint"]["full"].as_u64().ok_or_else(||bad("first-eval parent FULL"))?;
            let all4=short["joint"]["all4"].as_u64().ok_or_else(||bad("first-eval guard ALL4"))?;
            let base4=parent[name]["joint"]["all4"].as_u64().ok_or_else(||bad("first-eval parent ALL4"))?;
            let errors=short["joint"]["errors"].as_u64().ok_or_else(||bad("first-eval guard errors"))?;
            severe|=base.saturating_sub(cur)>=16||errors>=4;
            persistent|=base.saturating_sub(cur)>=5||base4.saturating_sub(all4)>=3;
        }
        let stop=if severe{Some("SEVERE_RETENTION")}else if prior["persistent"]==true&&persistent{Some("PERSISTENT_RETENTION")}else{None};
        quality.push(binary::record!({"arm":original.arms()[arm],"quality_stop":stop,"severe":severe,
            "persistent":persistent,"prior_decision_hash":file_hash(&original.root.join(original.arms()[arm]).join("decision-128.r3b"))?}));
    }
    first_eval_split_usage(d,view,review)?;
    let usage=first_eval_usage_labels(d,view,&["eval-only"])?;
    let report=binary::record!({"contract":FIRST_EVAL_ONLY,"policy":digest(d)?,"source":review.source,
        "main_observation_source":view.source,"post_main_executor_hash":first_eval_post_main(d,original,view)?.2,"original_failed":true,
        "original_terminal":d.terminal_hash,"original_success":false,"training_commits":[256,256],"new_optimizer":0,
        "native":[d.endpoints[0].physical,d.endpoints[1].physical],"C_generation_source":original.root,
        "W_generation_source":view.root,"teacher_source":view.root,"scores":scores,"quality":quality,
        "retention_guard_pass":quality.iter().all(|q|q["quality_stop"].is_null()),"Goal1_accepted":false,"usage":usage,
        "main_observation":file_hash(&view.root.join("eval-only-finished.r3b"))?});
    let path=view.root.join("eval-only-report.r3b");
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=report{return Err(bad("first-eval report changed"));}}
    else{publish_confirmed(&path,&report)?;}
    println!("FIRST_EVAL_REPORT retention_guard_pass={} usage={usage:?} original_failed=true Goal1=false",report["retention_guard_pass"]);Ok(())
}
fn first_eval_review(d:&FirstEvalOnly,original:&Study,view:&Study,review:&Study,arm:usize)->Result<()> {
    first_eval_report(d,original,view,review)?;
    let src=if arm==0{original}else{view};let p=&d.endpoints[arm];
    let(cases,expected,selection)=first_review_cases(src,arm,p)?;
    let train=first_teacher_cases(view,true)?;let dev=first_teacher_cases(view,false)?;
    let positions=first_review_teacher_positions(&train,&dev)?;
    let es=train.into_iter().chain(dev).collect::<Vec<_>>();
    let teacher=positions.iter().map(|&i|es[i].clone()).collect::<Vec<_>>();
    let origin=first_eval_teacher_read(view,arm,p)?;
    let selected=positions.iter().map(|&i|origin[i].clone()).collect::<Vec<_>>();
    let label=format!("review-{}",review.arms()[arm]);
    let b=binary::record!({"contract":FIRST_EVAL_ONLY,"policy":digest(d)?,"source":review.source,
        "post_main_executor_hash":first_eval_post_main(d,original,view)?.2,"native":p.physical,
        "original_raw":arm==0,"selection":selection,"expected":digest(&expected)?,"teacher_cases":digest(&teacher)?,
        "teacher_expected":digest(&selected)?,"planned_generation":cases.len(),"planned_teacher":8});
    let(index,mut ctl)=first_eval_start(d,view,review,&label,&b)?;
    let result=(||->Result<()>{let l=checkpoint::load(&p.native,Backend::Metal0.open()?,false)?;
        let model=l.model.weights_content_id()?;
        let binding=binary::record!({"policy":digest(review)?,"model":model,"cases":digest(&cases)?,"planned":cases.len(),"call_protocol":1});
        let rows=generated(&l,&review.root,&label,&cases,&binding,&mut ctl)?;
        for(a,b)in rows.iter().zip(&expected){for field in ["raw_tokens","actual","finish_reason","error","generation_completed"]{
            if a[field]!=b[field]{return Err(bad("first-eval B generation mismatch"));}
        }}
        let tbind=binary::record!({"policy":digest(review)?,"model":model,"cases":digest(&teacher)?,"planned":8,"call_protocol":1});
        let replay=teacher_prefix(&review.root,&label,&tbind,&l,&teacher,&mut ctl)?;
        diagnosis::validate_teacher_forward(&review.runtime,review.config.seq_len,&teacher,&replay,&l.tokenizer)?;
        first_eval_teacher_parity(&teacher,&replay,&selected,&l.tokenizer)?;
        ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(review,&label,index,&mut ctl,&result,binary::record!({"normal":16,"generation":cases.len(),
        "teacher":8,"native":p.physical,"selection":selection}))?;
    result
}
fn first_eval_teacher_parity(es:&[Episode],replay:&[binary::Value],main:&[binary::Value],tok:&ByteBpe)->Result<()> {
    if replay.len()!=8||main.len()!=8||es.len()!=8{return Err(bad("first-eval B teacher8 count"));}
    diagnosis::teacher_scores(es,replay,tok)?;diagnosis::teacher_scores(es,main,tok)?;
    for(a,b)in replay.iter().zip(main){let a=&a["teacher"]["target_token_observation"];
        let b=&b["teacher"]["target_token_observation"];
        if a["gold"]!=b["gold"]||a["argmax"]!=b["argmax"]{return Err(bad("first-eval B teacher IDs"));}
        let x:Vec<f64>=binary::from_value(a["nll"].clone())?;let y:Vec<f64>=binary::from_value(b["nll"].clone())?;
        if x.len()!=y.len()||x.iter().zip(y).any(|(x,y)|(x-y).abs()>1e-5){return Err(bad("first-eval B teacher NLL"));}
    }
    Ok(())
}
fn first_parent_scores(s:&Study)->Result<BTreeMap<String,binary::Value>>{
    if first_fixture(s){return Ok(["value","citation","S1Q1","FULL-word","FULL-renamed"].into_iter()
        .map(|name|(name.into(),binary::record!({"joint":{"full":4,"all4":1,"errors":0,"total":4}}))).collect());}
    let f=s.first_decision.as_ref().ok_or_else(||bad("first-decision parent profile"))?;
    let(d,view)=fit_posthoc_load(&f.posthoc,false)?;
    if d.endpoint.physical!=s.parent_hash{return Err(bad("first-decision parent posthoc native"));}
    let model=checkpoint::metadata(&s.parent)?.0.model_content_digest;
    let mut out=BTreeMap::new();
    for panel in fit_panels(&view)?.into_iter().take(5){
        let(score,_)=event_read_panel(&view,"F",3069,&model,&panel,64)?;
        out.insert(panel.0,score);
    }
    Ok(out)
}
fn first_baseline(s:&Study)->Result<()> {
    if !history(s)?.is_empty(){return Err(bad("first-decision baseline after training"));}
    let binding=binary::record!({"policy":digest(s)?,"parent":s.parent_hash,"source":"completed-posthoc-parent-64","model_calls":0});
    let(index,mut ctl)=start_observation(s,"baseline",&binding)?;
    let result=(||->Result<()>{let scores=first_parent_scores(s)?;
        if scores.len()!=5||scores.values().any(|r|r["joint"]["total"]!=64){return Err(bad("first-decision parent64 evidence"));}
        ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,"baseline",index,&mut ctl,&result,binary::record!({"parent64":result.is_ok(),"new_generation":0,"new_teacher":0}))?;
    result
}
fn first_teacher_cases(s:&Study,train:bool)->Result<Vec<Episode>>{
    let c=event_inputs(s,0)?;let(_,tm,dm)=inputs(s)?;
    let(episodes,meta)=if train{(&c.train[6144..7680],&tm[6144..7680])}else{(&c.validation[3072..3264],&dm[3072..3264])};
    Ok(first_teacher_indices(episodes,meta,train)?.into_iter().map(|i|episodes[i].clone()).collect())
}
fn first_teacher_indices(episodes:&[Episode],meta:&[Meta],train:bool)->Result<Vec<usize>>{
    if episodes.len()!=if train{1536}else{192}||meta.len()!=episodes.len(){return Err(bad("first-decision teacher source count"));}
    let pairs=["words0-1","words0-2","words0-3","words1-2","words1-3","words2-3"];
    let mut groups=BTreeMap::<String,BTreeMap<usize,BTreeMap<String,[Option<usize>;4]>>>::new();let mut ids=BTreeSet::new();
    for (i,(e,m)) in episodes.iter().zip(meta).enumerate(){
        let(pair,id)=m.template.split_once("/id").ok_or_else(||bad("first-decision teacher word role"))?;
        let version=match id{"0"=>0,"1" if train=>1,_=>return Err(bad("first-decision teacher ID role"))};
        if !pairs.contains(&pair)||m.view>3||m.split!=if train{"train"}else{"dev"}
            ||e.id!=m.id||e.family!=m.base||e.sequence!=m.base||e.id!=format!("{}/{}",m.base,m.view)
            ||!e.binding.ends_with(&format!("/{pair}"))||!ids.insert(e.id.clone()){
            return Err(bad("first-decision teacher metadata/duplicate ID/view"));
        }
        let slots=groups.entry(pair.into()).or_default().entry(version).or_default().entry(e.binding.clone()).or_insert([None;4]);
        if slots[m.view].replace(i).is_some(){return Err(bad("first-decision teacher duplicate view"));}
    }
    if groups.len()!=6{return Err(bad("first-decision teacher six-pair coverage"));}
    let mut selected=vec![];
    for pair in pairs{
        let versions=&groups[pair];
        if versions.len()!=if train{2}else{1}{return Err(bad("first-decision teacher ID-version coverage"));}
        for (version,bindings) in versions{
            if *version>usize::from(train)||bindings.len()!=if train{32}else{8}
                ||bindings.values().any(|slots|slots.iter().any(Option::is_none)){
                return Err(bad("first-decision teacher binding four-view coverage"));
            }
        }
        let mut chosen=BTreeSet::new();
        for version in if train{vec![0,1]}else{vec![0,0]}{
            let(binding,slots)=versions[&version].iter().find(|(binding,_)|!chosen.contains(*binding))
                .ok_or_else(||bad("first-decision teacher two distinct bindings"))?;
            chosen.insert(binding.clone());selected.extend(slots.iter().map(|v|v.unwrap()));
        }
    }
    if selected.len()!=48||selected.iter().collect::<BTreeSet<_>>().len()!=48{return Err(bad("first-decision teacher48 unique"));}
    let mut words=BTreeSet::new();let mut versions=[0usize;2];
    for &i in &selected{let(value,id)=event_label(&episodes[i].request,false)?;
        if episodes[i].answer!=format!("{value}입니다. [event:{id}]"){return Err(bad("first-decision teacher answer binding"));}
        words.insert(value);versions[usize::from(meta[i].template.ends_with("/id1"))]+=1;
    }
    if words!=["왼쪽","오른쪽","직진","대기"].into_iter().map(str::to_owned).collect()
        ||versions!=if train{[24,24]}else{[48,0]}{return Err(bad("first-decision teacher four-word/ID balance"));}
    Ok(selected)
}
fn first_review_teacher_positions(train:&[Episode],dev:&[Episode])->Result<Vec<usize>>{
    if train.len()!=48||dev.len()!=48{return Err(bad("first-decision B teacher source48"));}
    let mut positions=vec![];
    for (is_train,pair,version,view) in [(true,"words0-1",0,0),(true,"words0-1",0,1),
        (true,"words2-3",1,0),(true,"words2-3",1,1),(false,"words0-1",0,2),
        (false,"words0-1",0,3),(false,"words2-3",0,2),(false,"words2-3",0,3)]{
        let cases=if is_train{train}else{dev};
        let i=cases.iter().position(|e|e.binding.ends_with(&format!("/{pair}"))
            &&e.family.ends_with(&format!("/id{version}"))&&e.id.ends_with(&format!("/{view}")))
            .ok_or_else(||bad("first-decision B all-word/ID/view selection"))?;
        positions.push(i+if is_train{0}else{48});
    }
    let main=train.iter().chain(dev).collect::<Vec<_>>();let mut words=BTreeSet::new();let mut ids=BTreeSet::new();
    for &i in &positions{let e=main[i];let(value,id)=event_label(&e.request,false)?;
        if e.answer!=format!("{value}입니다. [event:{id}]")||!ids.insert(&e.id){return Err(bad("first-decision B teacher duplicate/answer"));}
        words.insert(value);
    }
    if positions.len()!=8||words!=["왼쪽","오른쪽","직진","대기"].into_iter().map(str::to_owned).collect(){
        return Err(bad("first-decision B four-word coverage"));
    }
    Ok(positions)
}
fn first_evaluate(s:&Study,arm:usize,l:&mut checkpoint::Loaded,p:&Progress,final_eval:bool,ctl:&mut RunControl)->Result<BTreeMap<String,binary::Value>>{
    if p.local>256{return Err(bad("first-decision endpoint range"));}
    let final_eval=final_eval||p.local>128;
    let teachers=if final_eval&&!first_fixture(s){let train=first_teacher_cases(s,true)?;let dev=first_teacher_cases(s,false)?;
        first_review_teacher_positions(&train,&dev)?;Some((train,dev))}else{None};
    l.model.refresh_identity()?;let mut out=BTreeMap::new();
    for panel in fit_panels(s)?.into_iter().take(if final_eval{6}else{5}){
        guard_bytes(s,8*1024*1024)?;
        let name=panel.0.clone();
        let short=event_panel_score(s,s.arms()[arm],p.local,l,&panel,first_panel_count(s,false),ctl)?;
        let score=if final_eval&&["FULL-word","FULL-renamed","FULL-train"].contains(&name.as_str()){
            event_panel_score(s,s.arms()[arm],p.local,l,&panel,first_panel_count(s,true),ctl)?
        }else{short};
        out.insert(name,score);
        guard_bytes(s,0)?;
    }
    if let Some((train,dev))=teachers{first_teacher_collect(s,arm,l,p,&train,&dev,ctl)?;}
    Ok(out)
}
fn first_teacher_collect(s:&Study,arm:usize,l:&mut checkpoint::Loaded,p:&Progress,train:&[Episode],dev:&[Episode],ctl:&mut RunControl)->Result<()> {
        guard_bytes(s,8*1024*1024)?;
        let es=train.iter().chain(dev).cloned().collect::<Vec<_>>();
        let label=format!("teacher-{}-FULL",p.local);
        let binding=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"model":l.model.weights_content_id()?,
            "arm":s.arms()[arm],"local":p.local,"cases":digest(&es)?,"planned":96,"train":48,"dev":48,"call_protocol":1});
        let rows=teacher_prefix(&s.root.join(s.arms()[arm]),&label,&binding,l,&es,ctl)?;
        diagnosis::validate_teacher_forward(&s.runtime,s.config.seq_len,&es,&rows,&l.tokenizer)?;
        let score=binary::record!({"binding":binding,"train":diagnosis::teacher_scores(train,&rows[..48],&l.tokenizer)?,
            "dev":diagnosis::teacher_scores(dev,&rows[48..],&l.tokenizer)?,"rows_digest":digest(&rows)?});
        let path=s.root.join(s.arms()[arm]).join(format!("{label}-score.r3b"));
        if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("first-decision teacher score changed"));}}
        else{publish_confirmed(&path,&score)?;}
        guard_bytes(s,0)?;
    Ok(())
}
fn first_final_observation(s:&Study,arm:usize,l:&mut checkpoint::Loaded,p:&Progress,ctl:&mut RunControl)->Result<()> {
    first_evaluate(s,arm,l,p,true,ctl)?;Ok(())
}
fn first_decision(s:&Study,arm:usize,p:&Progress,scores:&BTreeMap<String,binary::Value>)->Result<Option<String>>{
    let parent=first_parent_scores(s)?;
    let mut persistent=false;let mut severe=false;
    for name in ["value","citation","S1Q1","FULL-word","FULL-renamed"]{
        let short;
        let current=if scores[name]["joint"]["total"]!=first_panel_count(s,false){
            let panel=fit_panels(s)?.into_iter().find(|p|p.0==name).ok_or_else(||bad("first-decision guard panel"))?;
            let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
            short=event_read_panel(s,s.arms()[arm],p.local,&model,&panel,first_panel_count(s,false))?.0;&short
        }else{&scores[name]};
        let before=&parent[name];
        let cur=current["joint"]["full"].as_u64().ok_or_else(||bad("first-decision score FULL"))?;
        let base=before["joint"]["full"].as_u64().ok_or_else(||bad("first-decision parent FULL"))?;
        let all4=current["joint"]["all4"].as_u64().ok_or_else(||bad("first-decision score ALL4"))?;
        let base4=before["joint"]["all4"].as_u64().ok_or_else(||bad("first-decision parent ALL4"))?;
        let errors=current["joint"]["errors"].as_u64().ok_or_else(||bad("first-decision score errors"))?;
        severe|=base.saturating_sub(cur)>=16||errors>=4;
        persistent|=base.saturating_sub(cur)>=5||base4.saturating_sub(all4)>=3;
    }
    let prior=if p.local==256{
        let old:binary::Value=read_confirmed(&s.root.join(s.arms()[arm]).join("decision-128.r3b"))?;
        if old["policy"]!=digest(s)?||!old["stop"].is_null(){return Err(bad("first-decision prior guard changed"));}
        old["persistent"].as_bool().ok_or_else(||bad("first-decision prior guard flag"))?
    }else{false};
    let quality_stop=if severe{Some("SEVERE_RETENTION".to_owned())}else if prior&&persistent{Some("PERSISTENT_RETENTION".to_owned())}else{None};
    let cost_stop=p.stop.as_ref().filter(|v|first_cost_reason(v)).cloned();
    let stop=cost_stop.clone().or(quality_stop.clone());
    let record=binary::record!({"policy":digest(s)?,"local":p.local,"scores":scores,"parent":parent,"persistent":persistent,"quality_stop":quality_stop,"cost_stop":cost_stop,"stop":stop});
    let path=s.root.join(s.arms()[arm]).join(format!("decision-{}.r3b",p.local));
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=record{return Err(bad("first-decision guard changed"));}}
    else{publish_confirmed(&path,&record)?;}
    Ok(quality_stop)
}
fn first_cost_reason(v:&str)->bool{["BUDGET_EXHAUSTED","MATCHED_COST_ENDPOINT","UNMATCHED_COST_ENDPOINT"].contains(&v)}
fn first_cost_decision(s:&Study,arm:usize,p:&Progress,scores:&BTreeMap<String,binary::Value>)->Result<()> {
    let regular=s.root.join(s.arms()[arm]).join(format!("decision-{}.r3b",p.local));
    if !regular.exists(){first_decision(s,arm,p,scores)?;return Ok(());}
    if p.local!=128||!p.stop.as_deref().is_some_and(first_cost_reason){return Err(bad("first-decision cost final identity"));}
    let old:binary::Value=read_confirmed(&regular)?;
    if old["policy"]!=digest(s)?||old["local"]!=p.local||!old["stop"].is_null()
        ||!old["cost_stop"].is_null()||!old["quality_stop"].is_null()
        ||["value","citation","S1Q1","FULL-word","FULL-renamed"].iter()
            .any(|name|old["scores"][*name]["joint"]["total"]!=first_panel_count(s,false)){
        return Err(bad("first-decision prior +128 guard changed"));
    }
    let record=binary::record!({"policy":digest(s)?,"local":p.local,"scores":scores,"prior_decision_hash":file_hash(&regular)?,
        "quality_stop":null,"cost_stop":p.stop,"stop":p.stop});
    let path=s.root.join(s.arms()[arm]).join(format!("cost-decision-{}.r3b",p.local));
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=record{return Err(bad("first-decision cost final changed"));}}
    else{publish_confirmed(&path,&record)?;}
    Ok(())
}
fn first_panel_count(s:&Study,full:bool)->usize{
    if first_fixture(s){return 4;}
    if full{192}else{64}
}
fn first_fixture(s:&Study)->bool{
    #[cfg(test)]
    {s.first_decision.as_ref().is_some_and(|f|f.terminal_hash=="0".repeat(64))}
    #[cfg(not(test))]
    {let _=s;false}
}
fn fit_panels(s:&Study)->Result<Vec<Panel>>{
    let f=s.full_fit.as_ref().ok_or_else(||bad("FULL_FIT panels/profile"))?;let(old,tm,dm)=inputs(s)?;let c=event_inputs(s,0)?;
    let mut out=vec![];for(name,at,n)in [("value",0,512),("citation",512,512),("S1Q1",2560,512),("FULL-word",3072,192),("FULL-renamed",3264,192)]{
        out.push((name.into(),c.validation[at..at+n].to_vec(),dm[at..at+n].to_vec()));}
    out.push(("FULL-train".into(),f.train_order.iter().map(|&i|c.train[i].clone()).collect(),f.train_order.iter().map(|&i|tm[i].clone()).collect()));
    out.push(("OLD_FULL".into(),old.validation[3072..3136].to_vec(),dm[3072..3136].to_vec()));Ok(out)
}
fn fit_baseline(s:&Study)->Result<()> {
    if !history(s)?.is_empty(){return Err(bad("FULL_FIT parent parity after training"));}
    let f=s.full_fit.as_ref().unwrap();let old=load_study(&f.predecessor,false)?;let mut es=vec![];let mut expected=vec![];
    for panel in event_panels(&old,true,false)?.into_iter().filter(|p|p.0=="FULL-word"||p.0=="citation"){
        let(_,rows)=event_read_panel(&old,"F",64,&s.parent_content,&panel,panel.1.len())?;es.extend_from_slice(&panel.1[..8]);expected.extend_from_slice(&rows[..8]);}
    if es.len()!=16{return Err(bad("FULL_FIT fixed parent16"));}
    let b=binary::record!({"policy":digest(s)?,"native":s.parent_hash,"runtime":s.runtime,"cases":digest(&es)?,"planned":16,"call_protocol":1});
    let(index,mut ctl)=start_observation(s,"baseline",&b)?;
    let result=(||->Result<()>{let l=checkpoint::load(&s.parent,Backend::Metal0.open()?,false)?;
        let rows=generated(&l,&s.root,"baseline",&es,&b,&mut ctl)?;
        for(a,b)in rows.iter().zip(&expected){for field in ["raw_tokens","actual","finish_reason","error","generation_completed"]{if a[field]!=b[field]{return Err(bad("FULL_FIT parent16 parity mismatch"));}}}
        ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,"baseline",index,&mut ctl,&result,binary::record!({"parity":result.is_ok(),"planned":16}))?;result
}
fn fit_quality(r:&binary::Value,kind:&str)->bool{
    let(total,full,pair,all4,component)=match kind{"value"|"citation"|"S1Q1"=>(512,488,232,116,508),"FULL-train"=>(1536,1524,756,372,0),_=>(192,183,88,44,190)};
    let j=&r["joint"];let at_least=|k:&str,n:u64|j[k].as_u64().is_some_and(|v|v>=n);
    j["total"]==total&&at_least("full",full)&&at_least("query_both",pair)&&at_least("swap_both",pair)&&at_least("all4",all4)&&j["errors"]==0
        &&(kind=="value"||(r["valid_outside_id"]==0&&r["parse_failure_rows"]==0&&(component==0||r["value_correct"].as_u64().is_some_and(|v|v>=component)&&r["citation_support_correct"].as_u64().is_some_and(|v|v>=component))))
}
fn fit_development(scores:&BTreeMap<String,binary::Value>)->bool{
    ["value","citation","S1Q1","FULL-word","FULL-renamed"].iter().all(|n|scores.get(*n).is_some_and(|r|fit_quality(r,n)))
}
fn fit_teacher(s:&Study,l:&checkpoint::Loaded,p:&Progress,ctl:&mut RunControl)->Result<()> {
    let es=event_teacher_cases(s,0)?;let label=format!("teacher-{}-FULL",p.local);
    let b=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"model":l.model.weights_content_id()?,"arm":"F","local":p.local,"mode":0,"cases":digest(&es)?,"planned":64,"call_protocol":1});
    let rows=teacher_prefix(&s.root.join("F"),&label,&b,l,&es,ctl)?;
    diagnosis::validate_teacher_forward(&s.runtime,s.config.seq_len,&es,&rows,&l.tokenizer)?;
    let score=binary::record!({"binding":b,"train":diagnosis::teacher_scores(&es[..32],&rows[..32],&l.tokenizer)?,"dev":diagnosis::teacher_scores(&es[32..],&rows[32..],&l.tokenizer)?,"rows_digest":digest(&rows)?});
    let path=s.root.join("F").join(format!("{label}-score.r3b"));if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("FULL_FIT teacher score mismatch"));}}else{publish_confirmed(&path,&score)?;}Ok(())
}
fn fit_evaluate(s:&Study,l:&mut checkpoint::Loaded,p:&Progress,ending:bool,ctl:&mut RunControl)->Result<BTreeMap<String,binary::Value>>{
    l.model.refresh_identity()?;let panels=fit_panels(s)?;let full=[1536,3072].contains(&p.local);let mut scores=BTreeMap::new();
    for panel in &panels[..5]{let count=if full{panel.1.len()}else{64};
        let r=event_panel_score(s,"F",p.local,l,panel,count,ctl)?;
        if count>64&&["value","citation","S1Q1"].contains(&panel.0.as_str()){event_panel_score(s,"F",p.local,l,panel,64,ctl)?;}
        scores.insert(panel.0.clone(),r);
    }
    let all_train=p.local==3072||p.local==1536&&fit_development(&scores);
    if all_train||p.local==768||p.local==1536||ending{
        let n=if all_train{1536}else{192};scores.insert("FULL-train".into(),event_panel_score(s,"F",p.local,l,&panels[5],n,ctl)?);
    }
    if full{fit_teacher(s,l,p,ctl)?;}
    if ending{scores.insert("OLD_FULL".into(),event_panel_score(s,"F",p.local,l,&panels[6],64,ctl)?);}
    Ok(scores)
}
fn fit_guard_value(s:&Study,step:usize,model:&str)->Result<binary::Value>{
    let mut streak=[0usize;3];let mut previous=None;let dir=s.root.join("F");
    for &at in s.full_fit.as_ref().unwrap().schedule.iter().filter(|&&at|at<step){
        let path=dir.join(format!("decision-{at}.r3b"));let d:binary::Value=read_confirmed(&path)?;
        if d["policy"]!=digest(s)?||d["local"]!=at||!d["stop"].is_null()||d["previous"]!=binary::record!(previous){return Err(bad("FULL_FIT previous guard invalid"));}
        streak=binary::from_value(d["streak"].clone())?;previous=Some(file_hash(&path)?);
    }
    let before=streak;let mut stop=None;let mut panel_digests=BTreeMap::new();
    for(i,panel)in fit_panels(s)?.into_iter().take(3).enumerate(){let(r,_)=event_read_panel(s,"F",step,model,&panel,64)?;let j=&r["joint"];
        let (next,stopped)=fit_guard_panel(streak[i],j)?;streak[i]=next;
        if stopped{stop=Some("RETENTION_STOP".to_owned());}
        panel_digests.insert(panel.0,digest(&r)?);
    }
    Ok(binary::record!({"policy":digest(s)?,"local":step,"model":model,"previous":previous,"guard_before":before,"streak":streak,"panels":panel_digests,"stop":stop}))
}
fn fit_guard_panel(before:usize,j:&binary::Value)->Result<(usize,bool)>{
    let full=j["full"].as_u64().ok_or_else(||bad("FULL_FIT guard full"))?;let all4=j["all4"].as_u64().ok_or_else(||bad("FULL_FIT guard all4"))?;let errors=j["errors"].as_u64().ok_or_else(||bad("FULL_FIT guard errors"))?;
    if j["total"]!=64||full>64||all4>16||errors>64{return Err(bad("FULL_FIT guard denominator"));}
    let next=if full<=56||all4<=12{before+1}else{0};Ok((next,full<=48||errors>=4||next>=2))
}
fn fit_decision(s:&Study,p:&Progress,model:&str)->Result<Option<String>>{
    let d=fit_guard_value(s,p.local,model)?;let path=s.root.join("F").join(format!("decision-{}.r3b",p.local));
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=d{return Err(bad("FULL_FIT guard decision changed"));}}else{publish_confirmed(&path,&d)?;}
    Ok(binary::from_value(d["stop"].clone())?)
}
fn fit_train(s:&Study)->Result<()> {
    if read_confirmed::<binary::Value>(&s.root.join("baseline-finished.r3b"))?["success"]!=true{return Err(bad("FULL_FIT parent parity incomplete"));}
    let h=history(s)?;if h.last().is_some_and(|r|!r.resume){return Err(bad("FULL_FIT closed; no resume"));}
    let index=h.len();let mut phase=h.last().map_or(0,|r|r.phase);
    let mut p=h.last().map(|r|r.arms[0].clone()).unwrap_or(Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false});
    let saved=p.local;let previous=if index==0{None}else{Some(file_hash(&s.root.join(format!("segment-{:03}-finished.r3b",index-1)))?)};
    let mut ctl=control(s,&h)?;
    publish_confirmed(&s.root.join(format!("segment-{index:03}-started.r3b")),&binary::record!({"policy":digest(s)?,"previous":previous,"phase":phase,"arms":[p.clone()]}))?;
    let device=Backend::Metal0.open()?;let(mut l,mut o)=load_arm(s,0,&p,&device)?;let mut entered=false;
    let discarded=fit_discarded(s)?;
    let result=(||->Result<()>{
        if index==0&&(l.model.weights_content_id()?!=s.parent_content||optimizer_hash(&o.tensors)?!=s.parent_adam||o.protocol.local_step!=576){return Err(bad("FULL_FIT initial tensors/clock"));}
        let samples=samples_with_framing(&event_inputs(s,0)?.train,&l.tokenizer,s.config.seq_len,neural::Framing::QuestionEvidence)?;
        loop{
            let schedule=&s.full_fit.as_ref().unwrap().schedule;
            if p.stop.is_some()||phase>=schedule.len(){
                if !p.fit{fit_evaluate(s,&mut l,&p,true,&mut ctl)?;p.fit=true;}break;
            }
            let end=if index==0{1}else{schedule[phase]};
            if p.local>end{return Err(bad("FULL_FIT cursor past scheduled endpoint"));}
            let mut trace=std::fs::OpenOptions::new().append(true).create(true).open(s.root.join("F").join(format!("updates-{index:03}.r3rows")))?;
            while p.local<end{
                ctl.check("full_fit_before_microbatch")?;guard_bytes(s,0)?;
                if p.local+discarded[0]>=s.max_updates{return Err(bad("FULL_FIT backward budget exhausted"));}
                // Do not begin a new update when the command is already at its
                // cleanup boundary. A returned discarded backward still counts.
                let b=batch(&samples,&s.tape[p.local],&device)?;device.synchronize()?;let start=Instant::now();
                let entry=s.root.join("F").join(format!("step-{}-segment-{index:03}-entered.r3b",p.local+1));
                publish_confirmed(&entry,&binary::record!({"policy":digest(s)?,"local":p.local+1,"rows":s.tape[p.local],"status":"MAY_ENTER"}))?;
                let mut row=train_update(s,0,&mut l,&mut o,&mut p,&b,&mut ctl,&mut entered,&entry,start)?;
                row["source_tape_index"]=binary::record!((s.full_fit.as_ref().unwrap().source_offset+p.local-1)%3072);append_row(&mut trace,&row)?;
                if p.local==1||p.local%16==0{println!("FULL_FIT_UPDATE local={} model={} Adam={} source={} input={} target={} loss={} remaining={}",p.local,row["model_step"],row["optimizer_local"],row["source_tape_index"],row["input"],row["target"],row["answer_ce"],s.max_updates-p.local);}
            }
            if p.native==s.parent||!fit_saved_cursor(&p){save_arm(s,index,0,&mut l,&o,&mut p)?;}
            if index==0{break;}
            let scores=fit_evaluate(s,&mut l,&p,false,&mut ctl)?;
            p.stop=fit_decision(s,&p,&l.model.weights_content_id()?)?;p.evaluated=p.local;phase+=1;
            if p.stop.is_none()&&p.local==1536&&fit_development(&scores)&&scores.get("FULL-train").is_some_and(|r|fit_quality(r,"FULL-train")){phase=schedule.len();}
        }
        ctl.seal_completed_no_call()?;Ok(())
    })();
    if let Err(e)=&result{ctl.classify_error(e);}if entered{return result;}
    if p.local>saved&&!fit_saved_cursor(&p){save_arm(s,index,0,&mut l,&o,&mut p)?;}
    let pure_time=ctl.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]);
    let seg=Segment{policy:digest(s)?,previous,phase,arms:vec![p.clone()],success:result.is_ok(),resume:!p.fit&&(result.is_ok()||pure_time),control:ctl.receipt(),error:result.as_ref().err().map(ToString::to_string)};
    publish_confirmed(&s.root.join(format!("segment-{index:03}-finished.r3b")),&seg)?;
    println!("FULL_FIT_SEGMENT index={index} cursor={} model={} Adam={} evaluated={} fit={} resume={} stop={:?} native={} hash={} active={} bytes={}",p.local,s.parent_step+p.local,s.clock(p.local),p.evaluated,p.fit,seg.resume,p.stop,p.native.display(),p.physical,seg.control["elapsed_seconds"],owned_bytes(&s.root)?);
    if pure_time{Ok(())}else{result}
}
fn fit_saved_cursor(p:&Progress)->bool{p.native.file_name().is_some_and(|v|v.to_string_lossy().ends_with(&format!("-step-{}.r3m",p.local)))}
fn fit_discarded(s:&Study)->Result<[usize;3]>{
    let mut total=[0usize;3];for item in std::fs::read_dir(s.root.join("F"))?{let path=item?.path();
        if path.file_name().is_some_and(|v|v.to_string_lossy().ends_with("-discarded.r3b")){
            let r:binary::Value=read_confirmed(&path)?;
            if r["forward_backward"]!=1||r["optimizer"]!=0{return Err(bad("FULL_FIT discarded usage UNKNOWN"));}
            total[0]+=1;total[1]+=r["input"].as_u64().ok_or_else(||bad("discarded input UNKNOWN"))?as usize;total[2]+=r["target"].as_u64().ok_or_else(||bad("discarded target UNKNOWN"))?as usize;
        }
    }Ok(total)
}
fn first_discarded(s:&Study,arm:usize)->Result<[usize;3]>{
    let mut total=[0usize;3];
    for item in std::fs::read_dir(s.root.join(s.arms()[arm]))?{let path=item?.path();
        if path.file_name().is_some_and(|v|v.to_string_lossy().ends_with("-discarded.r3b")){
            let r:binary::Value=read_confirmed(&path)?;
            if r["forward_backward"]!=1||r["optimizer"]!=0{return Err(bad("first-decision discarded UNKNOWN"));}
            total[0]+=1;
            total[1]+=r["input"].as_u64().ok_or_else(||bad("first-decision discarded input UNKNOWN"))?as usize;
            total[2]+=r["target"].as_u64().ok_or_else(||bad("first-decision discarded target UNKNOWN"))?as usize;
        }
    }
    if total[0]>8{return Err(bad("first-decision discarded reserve exceeded"));}
    Ok(total)
}
#[derive(Debug,PartialEq,Eq)]
enum FirstDispatch { Train, EvaluateOnly, CostStop }
fn first_dispatch(local:usize,endpoint:usize,discarded:usize,committed_cap:usize,backward_cap:usize)->Result<FirstDispatch>{
    if local>endpoint||endpoint>committed_cap||local>committed_cap||discarded>8||local+discarded>backward_cap{
        return Err(bad("first-decision dispatcher cursor/budget UNKNOWN"));
    }
    if local==endpoint{return Ok(FirstDispatch::EvaluateOnly);}
    if discarded==8||local+discarded==backward_cap{return Ok(FirstDispatch::CostStop);}
    Ok(FirstDispatch::Train)
}
fn first_finish_cost(s:&Study,models:&mut [(checkpoint::Loaded,Optimizer)],ps:&mut [Progress],ctl:&mut RunControl)->Result<()> {
    let peer=if ps[0].local!=ps[1].local{"UNMATCHED_COST_ENDPOINT"}else{"MATCHED_COST_ENDPOINT"};
    for p in ps.iter_mut(){if p.stop.is_none(){p.stop=Some(peer.into());}}
    for arm in 0..2{
        let p=&mut ps[arm];if p.fit{continue;}
        let scores=first_evaluate(s,arm,&mut models[arm].0,p,true,ctl)?;
        first_cost_decision(s,arm,p,&scores)?;
        p.evaluated=p.local;p.fit=true;
    }
    Ok(())
}
fn first_train(s:&Study)->Result<()> {
    if read_confirmed::<binary::Value>(&s.root.join("baseline-finished.r3b"))?["success"]!=true{return Err(bad("first-decision baseline missing"));}
    let h=history(s)?;if h.last().is_some_and(|r|!r.resume){return Err(bad("first-decision closed"));}
    let index=h.len();let mut phase=h.last().map_or(0,|r|r.phase);
    let initial=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
    let mut ps=h.last().map(|r|r.arms.clone()).unwrap_or(vec![initial.clone(),initial]);
    let saved=[ps[0].local,ps[1].local];
    let previous=if index==0{None}else{Some(file_hash(&s.root.join(format!("segment-{:03}-finished.r3b",index-1)))?)};
    let mut ctl=control(s,&h)?;
    publish_confirmed(&s.root.join(format!("segment-{index:03}-started.r3b")),&binary::record!({"policy":digest(s)?,"previous":previous,"phase":phase,"arms":ps}))?;
    let device=Backend::Metal0.open()?;
    let mut models=(0..2).map(|arm|load_arm(s,arm,&ps[arm],&device)).collect::<Result<Vec<_>>>()?;
    let samples=samples_with_framing(&event_inputs(s,0)?.train,&models[0].0.tokenizer,s.config.seq_len,neural::Framing::QuestionEvidence)?;
    let mut entered=false;
    let result=(||->Result<()>{
        if ps.iter().any(|p|p.stop.as_deref().is_some_and(first_cost_reason)){
            first_finish_cost(s,&mut models,&mut ps,&mut ctl)?;ctl.seal_completed_no_call()?;return Ok(());
        }
        let schedule=[1,128,256];
        if phase>=schedule.len(){return Err(bad("first-decision phase past end"));}
        let endpoint=schedule[phase];
        let mut cost_stop=false;
        'arms: for arm in 0..2{
            let(l,o)=&mut models[arm];let p=&mut ps[arm];
            if p.local>endpoint{return Err(bad("first-decision cursor past endpoint"));}
            let mut trace=std::fs::OpenOptions::new().append(true).create(true).open(s.root.join(s.arms()[arm]).join(format!("updates-{index:03}.r3rows")))?;
            while p.local<endpoint{
                let discarded=first_discarded(s,arm)?;
                if first_dispatch(p.local,endpoint,discarded[0],s.max_updates,s.first_decision.as_ref().unwrap().backward_cap)?==FirstDispatch::CostStop{
                    p.stop=Some("BUDGET_EXHAUSTED".into());cost_stop=true;break;
                }
                ctl.check("first_before_microbatch")?;guard_bytes(s,0)?;
                let b=batch(&samples,&s.tape[p.local],&device)?;device.synchronize()?;let start=Instant::now();
                let entry=s.root.join(s.arms()[arm]).join(format!("step-{}-segment-{index:03}-entered.r3b",p.local+1));
                publish_confirmed(&entry,&binary::record!({"policy":digest(s)?,"local":p.local+1,"rows":s.tape[p.local],"status":"MAY_ENTER"}))?;
                let mut row=train_update(s,arm,l,o,p,&b,&mut ctl,&mut entered,&entry,start)?;
                row["source_tape_index"]=binary::record!((573+p.local-1)%3072);append_row(&mut trace,&row)?;
                if p.local==1||p.local%32==0{println!("FIRST_UPDATE arm={} local={} model={} Adam={} input={} target={} objective={}",s.arms()[arm],p.local,row["model_step"],row["optimizer_local"],row["input"],row["target"],row["answer_ce"]);}
            }
            if p.local>saved[arm]&&!fit_saved_cursor(p){save_arm(s,index,arm,l,o,p)?;}
            if cost_stop{break 'arms;}
        }
        if ps.iter().any(|p|p.stop.as_deref()==Some("BUDGET_EXHAUSTED")){
            first_finish_cost(s,&mut models,&mut ps,&mut ctl)?;ctl.seal_completed_no_call()?;return Ok(());
        }
        if ps[0].local!=ps[1].local{return Err(bad("first-decision unmatched without cost stop"));}
        if phase==0{phase=1;ctl.seal_completed_no_call()?;return Ok(());}
        if ps[0].local<endpoint{return Err(bad("first-decision endpoint cursor"));}
        for arm in 0..2{
            let(l,_)=&mut models[arm];let p=&mut ps[arm];
            let scores=first_evaluate(s,arm,l,p,false,&mut ctl)?;
            let quality=first_decision(s,arm,p,&scores)?;if p.stop.is_none(){p.stop=quality;}p.evaluated=p.local;
        }
        let stopping=ps.iter().any(|p|p.stop.is_some());
        if stopping&&phase==1{
            // A normal +128 quality stop makes this same native the final endpoint.
            for arm in 0..2{let(l,_)=&mut models[arm];first_final_observation(s,arm,l,&ps[arm],&mut ctl)?;}
        }
        if phase==2||stopping{for p in &mut ps{p.fit=true;}}else{phase=2;}
        ctl.seal_completed_no_call()?;Ok(())
    })();
    if let Err(e)=&result{ctl.classify_error(e);}if entered{return result;}
    for arm in 0..2{if ps[arm].local>saved[arm]&&!fit_saved_cursor(&ps[arm]){
        let(l,o)=&mut models[arm];save_arm(s,index,arm,l,o,&mut ps[arm])?;
    }}
    let pure_time=ctl.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]);
    let terminal=ps.iter().all(|p|p.fit);
    let seg=Segment{policy:digest(s)?,previous,phase,arms:ps,success:result.is_ok(),resume:!terminal&&(result.is_ok()||pure_time),control:ctl.receipt(),error:result.as_ref().err().map(ToString::to_string)};
    publish_confirmed(&s.root.join(format!("segment-{index:03}-finished.r3b")),&seg)?;
    println!("FIRST_SEGMENT index={index} phase={phase} local={:?} fit={:?} resume={} active={} bytes={}",seg.arms.iter().map(|p|p.local).collect::<Vec<_>>(),seg.arms.iter().map(|p|p.fit).collect::<Vec<_>>(),seg.resume,seg.control["elapsed_seconds"],first_owned_bytes(s)?);
    if pure_time{Ok(())}else{result}
}
fn fit_counts(step:usize,ending:bool,dev_pass:bool)->[usize;7]{
    let full=[1536,3072].contains(&step);
    let train=if step==3072||step==1536&&dev_pass{1536}else if ending||step==768||step==1536{192}else{0};
    [if full{512}else{64},if full{512}else{64},if full{512}else{64},if full{192}else{64},if full{192}else{64},train,if ending{64}else{0}]
}
fn fit_read_endpoint(s:&Study,p:&Progress,ending:bool)->Result<BTreeMap<String,binary::Value>>{
    let(m,_)=checkpoint::metadata(&p.native)?;let st=m.training.as_ref().ok_or_else(||bad("FULL_FIT endpoint state missing"))?;
    let op=m.optimizer_protocol.as_ref().ok_or_else(||bad("FULL_FIT endpoint optimizer missing"))?;
    op.validate(&m.architecture,st)?;
    if st.step!=s.parent_step+p.local||st.sampler_state!=p.local as u64||op.local_step!=s.clock(p.local)||st.config!=s.config
        ||st.resume_binding!=Some(bind(s,0,st,&scoring_tokenizer(s)?)?)||file_hash(&p.native)?!=p.physical{return Err(bad("FULL_FIT endpoint policy/clock/native"));}
    let panels=fit_panels(s)?;let counts=fit_counts(p.local,ending,false);let mut scores=BTreeMap::new();
    for (panel,&n) in panels[..5].iter().zip(&counts[..5]){let(r,_)=event_read_panel(s,"F",p.local,&m.model_content_digest,panel,n)?;scores.insert(panel.0.clone(),r);}
    let counts=fit_counts(p.local,ending,fit_development(&scores));
    for (panel,&n)in panels[5..].iter().zip(&counts[5..]){if n>0{let(r,_)=event_read_panel(s,"F",p.local,&m.model_content_digest,panel,n)?;scores.insert(panel.0.clone(),r);}}
    Ok(scores)
}
fn fit_trace(s:&Study,h:&[Segment])->Result<binary::Value>{
    let c=event_inputs(s,0)?;let tok=scoring_tokenizer(s)?;let ss=samples_with_framing(&c.train,&tok,256,neural::Framing::QuestionEvidence)?;
    let(mut input,mut target,mut padding,mut count)=(0usize,0usize,0usize,0usize);let mut seen=vec![0usize;7680];
    for index in 0..h.len(){let path=s.root.join("F").join(format!("updates-{index:03}.r3rows"));if !path.exists(){continue;}
        for row in binary::read_value_records(&path)?{
            let draw=s.tape.get(count).ok_or_else(||bad("FULL_FIT trace beyond budget"))?;count+=1;
            let length=draw.iter().map(|&i|ss[i].tokens.len()-1).max().unwrap();let mut cost=[0usize;3];
            for &i in draw{cost[0]+=ss[i].tokens.len()-1;cost[1]+=ss[i].tokens.len()-ss[i].response_start;cost[2]+=length-(ss[i].tokens.len()-1);seen[i]+=1;}
            if row["local"]!=count||row["model_step"]!=s.parent_step+count||row["optimizer_local"]!=s.clock(count)||row["source_tape_index"]!=(576+count-1)%3072
                ||row["rows"]!=binary::record!(draw)||row["lr"]!=s.config.lr||row["examples"]!=8||row["input"]!=cost[0]||row["target"]!=cost[1]||row["padding"]!=cost[2]
                ||row["answer_ce"].as_f64().is_none_or(|v|!v.is_finite()){return Err(bad("FULL_FIT trace/cost/clock/input mismatch"));}
            input+=cost[0];target+=cost[1];padding+=cost[2];
        }
    }
    let discarded=fit_discarded(s)?;
    if count!=h.last().ok_or_else(||bad("FULL_FIT trace absent"))?.arms[0].local||count+discarded[0]>3072||input+discarded[1]>8_000_000||target+discarded[2]>1_000_000{return Err(bad("FULL_FIT trace count/budget"));}
    let f=s.full_fit.as_ref().unwrap();let mut histogram=BTreeMap::<String,usize>::new();
    for i in 6144..7680{*histogram.entry(format!("new{}/prior-full{}/semantic-other{}",seen[i],f.prior_full_exposure[i],f.semantic_other_exposure[i])).or_default()+=1;}
    Ok(binary::record!({"updates":count,"backward":count+discarded[0],"discarded":discarded,"examples":count*8,"input":input+discarded[1],"target":target+discarded[2],"padding":padding,"new_exact_exposure":seen,"word_exposure_histogram":histogram,"optimizer_clock":s.clock(count),"source_last_index":if count>0{Some((576+count-1)%3072)}else{None}}))
}
fn fit_report(s:&Study)->Result<()> {
    let h=history(s)?;let end=h.last().ok_or_else(||bad("FULL_FIT NOT_RUN"))?;let p=&end.arms[0];
    if !end.success||end.resume||!p.fit{return Err(bad("FULL_FIT incomplete; original partial preserved"));}
    let mut final_scores=None;
    for &step in s.full_fit.as_ref().unwrap().schedule.iter().filter(|&&n|n<=p.local){
        let mut natives=std::fs::read_dir(s.root.join("F"))?.collect::<std::io::Result<Vec<_>>>()?.into_iter().map(|e|e.path()).filter(|path|path.file_name().is_some_and(|v|v.to_string_lossy().ends_with(&format!("-step-{step}.r3m")))).collect::<Vec<_>>();
        if natives.len()!=1{return Err(bad("FULL_FIT scheduled checkpoint ambiguous/absent"));}let native=natives.remove(0);let model=checkpoint::metadata(&native)?.0.model_content_digest;
        let mut probe=p.clone();probe.local=step;probe.native=native;probe.physical=file_hash(&probe.native)?;
        let scores=fit_read_endpoint(s,&probe,step==p.local)?;
        let d=fit_guard_value(s,step,&model)?;
        if read_confirmed::<binary::Value>(&s.root.join("F").join(format!("decision-{step}.r3b")))?!=d||step<p.local&&!d["stop"].is_null()
            ||step==p.local&&d["stop"]!=binary::record!(p.stop){return Err(bad("FULL_FIT raw/guard/terminal disagreement"));}
        for(name,r)in &scores{println!("FULL_FIT_RESULT local={step} model={} {name} full={}/{} QB={} SB={} ALL4={} value={} support={} other={} outside={} malformed={} EOS={} errors={}",s.parent_step+step,r["joint"]["full"],r["joint"]["total"],r["joint"]["query_both"],r["joint"]["swap_both"],r["joint"]["all4"],r["value_correct"],r["citation_support_correct"],r["other_provided_id"],r["valid_outside_id"],r["parse_failure_rows"],r["joint"]["eos"],r["joint"]["errors"]);}
        if [1536,3072].contains(&step){event_read_teachers(s,0,&probe,0)?;}
        if step==p.local{final_scores=Some(scores);}
    }
    let scores=final_scores.ok_or_else(||bad("FULL_FIT final scheduled evaluation absent"))?;
    let dev=fit_development(&scores);let fit=scores.get("FULL-train").is_some_and(|r|fit_quality(r,"FULL-train"));
    let class=if p.stop.is_some(){"RETENTION_STOP"}else if dev&&fit{"FIT_AND_HELDOUT_DEVELOPMENT_PASS"}else if fit{"FIT_GAIN_NO_TRANSFER"}else{"TRAIN_FIT_INCOMPLETE"};
    let trace=fit_trace(s,&h)?;let mut summary=trace.clone();summary["new_exact_exposure"]=binary::Value::Null;println!("FULL_FIT_TRACE {summary}");
    // Compare only requests that were actually observed at the original F64.
    let old=load_study(&s.full_fit.as_ref().unwrap().predecessor,false)?;let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
    for panel in event_panels(&old,true,false)?.into_iter().filter(|x|["value","citation","S1Q1","FULL-word","FULL-renamed","OLD_FULL"].contains(&x.0.as_str())){
        let n=panel.1.len();let full_panel=fit_panels(s)?.into_iter().find(|x|x.0==panel.0).unwrap();
        let Some(score)=scores.get(&panel.0)else{continue};let measured=score["joint"]["total"].as_u64().unwrap()as usize;let n=n.min(measured);
        let(_,before)=event_read_panel(&old,"F",64,&s.parent_content,&panel,panel.1.len())?;
        let(_,after)=event_read_panel(s,"F",p.local,&model,&full_panel,measured)?;
        let mut pair=[0usize;4];for(a,b)in before.iter().zip(&after).take(n){pair[match(a["exact_match"]==true,b["exact_match"]==true){(true,true)=>0,(false,true)=>1,(true,false)=>2,_=>3}]+=1;}
        println!("FULL_FIT_PARENT_PAIRED {} n={n} both/gain/loss/neither={pair:?}",panel.0);
    }
    let f=s.full_fit.as_ref().unwrap();let c=event_inputs(s,0)?;let(_,tm,_)=inputs(s)?;let train=scores.get("FULL-train").unwrap();
    let raw=binary::read_value_records(&s.root.join("F").join(format!("eval-{}-FULL-train.r3rows",p.local)))?;let n=train["joint"]["total"].as_u64().unwrap()as usize;
    let seen:Vec<usize>=binary::from_value(trace["new_exact_exposure"].clone())?;let mut groups=BTreeMap::<String,[usize;3]>::new();
    for(&i,row)in f.train_order[..n].iter().zip(&raw[1..]){let value=c.train[i].answer.split_once("입니다.").ok_or_else(||bad("FULL_FIT train value"))?.0;
        for key in [format!("word/{value}"),format!("pair-id/{}",tm[i].template),format!("exact-exposure/{}/{}",f.prior_full_exposure[i],seen[i])]{let x=groups.entry(key).or_default();x[0]+=1;x[1]+=usize::from(row["exact_match"]==true);x[2]+=usize::from(row["finish_reason"]=="stop"&&row["error"].is_null());}}
    println!("FULL_FIT_TRAIN_GROUPS {groups:?}");
    println!("FULL_FIT_CLOSED class={class} development={dev} full_train_fit={fit} step={} Adam={} cursor={} stop={:?} native={} hash={} usage={:?} bytes={} Goal1=false",s.parent_step+p.local,s.clock(p.local),p.local,p.stop,p.native.display(),p.physical,previous_usage(s,&h)?,owned_bytes(&s.root)?);Ok(())
}
fn fit_posthoc_origin(root:&Path)->Result<(Study,Progress,binary::Value,(f64,usize,usize))>{
    let s=load_study(root,false)?;let h=history(&s)?;
    let end=h.last().ok_or_else(||bad("posthoc original terminal missing"))?;
    let trace=fit_trace(&s,&h)?;
    fit_posthoc_stop(&s,end,&trace)?;
    let p=end.arms[0].clone();let (m,_)=checkpoint::metadata(&p.native)?;
    let st=m.training.as_ref().ok_or_else(||bad("posthoc native state missing"))?;
    let op=m.optimizer_protocol.as_ref().ok_or_else(||bad("posthoc Adam identity missing"))?;
    op.validate(&m.architecture,st)?;
    if st.step!=s.parent_step+p.local||st.sampler_state!=p.local as u64||st.config!=s.config
        ||op.local_step!=s.clock(p.local)||st.resume_binding!=Some(bind(&s,0,st,&scoring_tokenizer(&s)?)?){return Err(bad("posthoc native/clock/policy mismatch"));}
    let a:binary::Value=read_confirmed(&s.root.join("review-a.r3b"))?;
    let mut usage=(a["active_seconds"].as_f64().filter(|n|n.is_finite()&&*n>=0.).ok_or_else(||bad("posthoc A usage UNKNOWN"))?,0usize,0usize);
    let mut controls=h.iter().map(|r|r.control.clone()).collect::<Vec<_>>();
    for label in ["baseline","review-F"]{controls.extend(observation_history(&s,label)?.into_iter().map(|r|r["control"].clone()));}
    for (i,c) in controls.iter().enumerate(){
        if i!=h.len()-1&&c["observed_conditions"].as_array().is_none_or(|v|v.iter().any(|x|x!="TIME_BUDGET")){return Err(bad("posthoc mixed original failure"));}
        usage.0+=c["elapsed_seconds"].as_f64().filter(|n|n.is_finite()&&*n>=0.).ok_or_else(||bad("posthoc elapsed UNKNOWN"))?;
        usage.1+=c["generation_calls"].as_u64().ok_or_else(||bad("posthoc generation UNKNOWN"))?as usize;
        usage.2+=c["teacher_calls"].as_u64().ok_or_else(||bad("posthoc teacher UNKNOWN"))?as usize;
    }
    if usage.1>s.generation_cap||usage.2>s.teacher_cap{return Err(bad("posthoc original call cap"));}
    Ok((s,p,trace,usage))
}
fn fit_posthoc_stop(s:&Study,end:&Segment,trace:&binary::Value)->Result<()> {
    if s.full_fit.is_none()||end.arms.len()!=1||end.success||end.resume||end.arms[0].fit||end.arms[0].stop.is_some()
        ||end.error.as_deref()!=Some(bad("FULL_FIT backward budget exhausted").to_string().as_str())
        ||end.control["observed_conditions"]!=binary::record!(["INTEGRITY_FAIL"])
        ||trace["backward"]!=s.max_updates||trace["updates"]!=end.arms[0].local||end.arms[0].local>=s.max_updates {
        return Err(bad("posthoc scope requires only the verified backward-cap endpoint"));
    }Ok(())
}
fn fit_posthoc_output(original:&Path,output:&Path)->Result<PathBuf>{
    let requested=output.parent().ok_or_else(||bad("posthoc output parent"))?.canonicalize()?.join(output.file_name().ok_or_else(||bad("posthoc output name"))?);
    if requested.starts_with(original)||requested.try_exists()?{return Err(bad("posthoc output must be new and outside the original"));}Ok(requested)
}
fn fit_posthoc_register(original:&Path,value:&binary::Value)->Result<()> {
    let path=original.with_extension("full-fit-posthoc.r3b");
    if path.try_exists()?||pending_path(&path).try_exists()?{return Err(bad("posthoc scope already registered or pending"));}
    publish_confirmed(&path,value)
}
fn fit_posthoc_prepare(original:&Path,output:&Path)->Result<()> {
    let(s,p,trace,usage)=fit_posthoc_origin(original)?;
    let requested=fit_posthoc_output(&s.root,output)?;
    if usage.1+3584>s.generation_cap||usage.2+72>s.teacher_cap||usage.0>=s.active_cap{return Err(bad("posthoc remaining original budget insufficient"));}
    // Scoped model/raw cap; retained build executables are reported separately.
    let original_bytes=owned_bytes(&s.root)?;
    if original_bytes.saturating_add(3584*7*1024+1024*1024)>s.bytes_cap{return Err(bad("posthoc model/raw headroom insufficient"));}
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let mut old=s.runtime.clone();old.binary=runtime.binary.clone();if old!=runtime{return Err(bad("posthoc runtime changed beyond executable"));}
    // One explicit observation scope per predecessor, outside the read-only run.
    // A second output directory cannot reset its inherited call allowance.
    fit_posthoc_register(&s.root,&binary::record!({"contract":FIT_POSTHOC,"original_plan":file_hash(&s.root.join("plan.r3b"))?,"output":requested}))?;
    std::fs::create_dir(output)?;let root=output.canonicalize()?;std::fs::create_dir(root.join("F"))?;
    let d=FitPosthoc{contract:FIT_POSTHOC.into(),root:root.clone(),original:s.root.clone(),plan_hash:file_hash(&s.root.join("plan.r3b"))?,
        terminal_hash:file_hash(&s.root.join(format!("segment-{:03}-finished.r3b",history(&s)?.len()-1)))?,endpoint:p,
        trace_hash:digest(&trace)?,prior_usage:usage,original_bytes,source:sources()?,runtime};
    publish_confirmed(&root.join("posthoc-plan.r3b"),&d)?;
    println!("FULL_FIT_POSTHOC_PREPARED policy={} original={} cursor={} model={} Adam={} prior={:?} original_bytes={} new_model_copies=0 calls0 A_PENDING",digest(&d)?,d.plan_hash,d.endpoint.local,s.parent_step+d.endpoint.local,s.clock(d.endpoint.local),usage,original_bytes);Ok(())
}
fn fit_posthoc_load(root:&Path,execute:bool)->Result<(FitPosthoc,Study)>{
    let d:FitPosthoc=read_confirmed(&root.join("posthoc-plan.r3b"))?;
    if d.contract!=FIT_POSTHOC||d.root!=root.canonicalize()?||d.root==d.original{return Err(bad("posthoc root/contract"));}
    let(s,p,trace,usage)=fit_posthoc_origin(&d.original)?;
    let registration=d.original.with_extension("full-fit-posthoc.r3b");
    let registered:binary::Value=read_confirmed(&registration)?;
    if registered!=binary::record!({"contract":FIT_POSTHOC,"original_plan":d.plan_hash,"output":d.root}){return Err(bad("posthoc scope registration mismatch"));}
    let terminal=s.root.join(format!("segment-{:03}-finished.r3b",history(&s)?.len()-1));
    if d.plan_hash!=file_hash(&s.root.join("plan.r3b"))?||d.terminal_hash!=file_hash(&terminal)?||digest(&p)?!=digest(&d.endpoint)?
        ||d.trace_hash!=digest(&trace)?||d.prior_usage!=usage||d.original_bytes!=owned_bytes(&s.root)?{return Err(bad("posthoc original changed"));}
    if execute{
        if d.source!=sources()?{return Err(bad("posthoc frozen source changed"));}d.runtime.verify(&Backend::Metal0.open()?)?;
        let a:binary::Value=read_confirmed(&root.join("posthoc-admission.r3b"))?;
        let path=Path::new(a["report"].as_str().ok_or_else(||bad("posthoc independent A absent"))?);
        if a["policy"]!=digest(&d)?||a["report_hash"]!=file_hash(path)?||a["accepted"]!=true{return Err(bad("posthoc independent A changed"));}
    }
    // Collector view only; never serialize this as a Study or alter native state.
    let mut view=s;view.root=d.root.clone();view.source=d.source.clone();view.runtime=d.runtime.clone();
    view.costs["posthoc_policy"]=binary::record!(digest(&d)?);
    view.bytes_cap=view.bytes_cap.checked_sub(d.original_bytes+std::fs::metadata(&registration)?.len()).ok_or_else(||bad("posthoc byte budget exhausted"))?;
    Ok((d,view))
}
fn fit_posthoc_admit(root:&Path,review:&Path)->Result<()> {
    let(d,_)=fit_posthoc_load(root,false)?;let r:binary::Value=read(review)?;
    if r["contract"]!=FIT_POSTHOC||r["policy"]!=digest(&d)?||r["source"]!=d.source||r["runtime"]!=binary::record!(d.runtime)
        ||r["verdict"]!="PASS"||r["boundaries"]!=binary::record!(["immutable_origin","zero_update","aggregate_budget","returned_resume"]){return Err(bad("posthoc independent A incomplete"));}
    publish_confirmed(&root.join("posthoc-admission.r3b"),&binary::record!({"policy":digest(&d)?,"report":review.canonicalize()?,"report_hash":file_hash(review)?,"accepted":true}))?;Ok(())
}
fn fit_posthoc_usage(d:&FitPosthoc,s:&Study)->Result<(f64,usize,usize)>{
    let mut usage=d.prior_usage;
    for label in ["posthoc","review-F"]{for r in observation_history(s,label)?{let c=&r["control"];
        if c["observed_conditions"].as_array().is_none_or(|v|v.iter().any(|x|x!="TIME_BUDGET")){return Err(bad("posthoc sticky failure"));}
        usage.0+=c["elapsed_seconds"].as_f64().filter(|n|n.is_finite()&&*n>=0.).ok_or_else(||bad("posthoc time UNKNOWN"))?;
        usage.1+=c["generation_calls"].as_u64().ok_or_else(||bad("posthoc calls UNKNOWN"))?as usize;
        usage.2+=c["teacher_calls"].as_u64().ok_or_else(||bad("posthoc teacher UNKNOWN"))?as usize;
    }}
    if usage.1>s.generation_cap||usage.2>s.teacher_cap{return Err(bad("posthoc aggregate call cap"));}Ok(usage)
}
fn fit_posthoc_start(d:&FitPosthoc,s:&Study,label:&str,b:&binary::Value)->Result<(usize,RunControl)>{
    let(time,g,t)=fit_posthoc_usage(d,s)?;guard_bytes(s,1024*1024)?;
    let cancel=std::sync::Arc::new(AtomicBool::new(false));let signal=cancel.clone();
    ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut ctl=RunControl::new(cancel,std::time::Duration::from_secs_f64((s.active_cap-time).max(0.).min(s.segment_cap)),12*1024*1024)?;
    ctl.set_call_limits(s.generation_cap-g,s.teacher_cap-t);
    start_observation_control(s,label,b,ctl)
}
fn fit_posthoc_scores(s:&Study,p:&Progress)->Result<BTreeMap<String,binary::Value>>{
    let model=checkpoint::metadata(&p.native)?.0.model_content_digest;let mut scores=BTreeMap::new();
    for panel in fit_panels(s)?{let(r,_)=event_read_panel(s,"F",p.local,&model,&panel,panel.1.len())?;scores.insert(panel.0,r);}
    event_read_teachers(s,0,p,0)?;Ok(scores)
}
fn fit_posthoc_collect(s:&Study,p:&Progress,l:&checkpoint::Loaded,panel:&Panel,ctl:&mut RunControl)->Result<()> {
    for begin in (0..panel.1.len()).step_by(64){
        guard_bytes(s,1024*1024)?;
        event_panel_score(s,"F",p.local,l,panel,(begin+64).min(panel.1.len()),ctl)?;
    }Ok(())
}
fn fit_posthoc_run(root:&Path,phase:&str)->Result<()> {
    let(d,s)=fit_posthoc_load(root,phase!="report")?;let p=&d.endpoint;
    match phase{
        "evaluate"=>{
            let b=binary::record!({"posthoc":digest(&d)?,"native":p.physical,"panels":[512,512,512,192,192,1536,64],"teacher":64,"optimizer":0,"backward":0});
            let(i,mut ctl)=fit_posthoc_start(&d,&s,"posthoc",&b)?;
            let result=(||->Result<()>{let l=checkpoint::load(&p.native,Backend::Metal0.open()?,false)?;
                for panel in fit_panels(&s)?{fit_posthoc_collect(&s,p,&l,&panel,&mut ctl)?;}
                guard_bytes(&s,1024*1024)?;fit_teacher(&s,&l,p,&mut ctl)?;
                fit_posthoc_scores(&s,p)?;ctl.seal_completed_no_call()?;Ok(())})();
            finish_observation(&s,"posthoc",i,&mut ctl,&result,binary::record!({"scope":"POST_HOC_DIAGNOSTIC","model_step":s.parent_step+p.local,"cursor":p.local,"planned_3072_completed":false,"candidate_eligible":false,"optimizer":0,"backward":0}))?;
            println!("POSTHOC_SEGMENT index={i} cursor={} control={} result={:?}",p.local,ctl.receipt(),result.as_ref().err().map(ToString::to_string));result
        },
        "report"|"review"=>{
            let h=observation_history(&s,"posthoc")?;if h.last().is_none_or(|r|r["success"]!=true){return Err(bad("posthoc final incomplete"));}
            let scores=fit_posthoc_scores(&s,p)?;let usage=fit_posthoc_usage(&d,&s)?;
            for(name,r)in &scores{println!("POSTHOC_RESULT cursor={} {name} {}",p.local,r);}
            println!("POSTHOC_ONLY model={} Adam={} cursor={} dev_gate={} train_gate={} usage={usage:?} original_resume=false candidate_eligible=false Goal1=false bytes={}",s.parent_step+p.local,s.clock(p.local),p.local,fit_development(&scores),fit_quality(&scores["FULL-train"],"FULL-train"),owned_bytes(&s.root)?);
            if phase=="review"{fit_replay(&s,p,scores,Some(&d))?;}Ok(())
        },_=>Err(bad("unknown posthoc phase"))
    }
}
fn fit_review(s:&Study)->Result<()> {
    fit_report(s)?;
    let h=history(s)?;let end=h.last().ok_or_else(||bad("FULL_FIT endpoint absent"))?;
    if !end.success||end.resume||!end.arms[0].fit{return Err(bad("FULL_FIT B requires normal endpoint"));}let p=&end.arms[0];
    let scores=fit_read_endpoint(s,p,true)?;fit_replay(s,p,scores,None)
}
fn fit_replay(s:&Study,p:&Progress,scores:BTreeMap<String,binary::Value>,posthoc:Option<&FitPosthoc>)->Result<()> {
    let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
    let mut all=vec![];let mut raw=vec![];let mut normal=BTreeSet::new();let mut ranks:[Vec<usize>;5]=Default::default();
    for (panel,take)in fit_panels(s)?.into_iter().take(5).zip([8,6,6,6,6]){
        let n=scores[&panel.0]["joint"]["total"].as_u64().unwrap()as usize;let(_,rows)=event_read_panel(s,"F",p.local,&model,&panel,n)?;
        for(i,(e,r))in panel.1.into_iter().zip(rows).enumerate(){let at=all.len();if i<take{normal.insert(at);}
            if r["exact_match"]!=true{let found=identifiable::binding::citation::individually_valid_ids(r["actual"].as_str().unwrap_or(""));
                let gold=identifiable::binding::citation::individually_valid_ids(&e.answer);
                let rank=if r["error_class"]=="strict_utf8"{0}else if r["finish_reason"]!="stop"{1}else if found.iter().any(|id|e.request.evidence.items.iter().all(|v|v.event_id!=*id)){2}else if found.iter().any(|id|!gold.contains(id)){3}else{4};ranks[rank].push(at);}
            all.push(e);raw.push(r);
        }
    }
    let mut selected=normal.iter().copied().collect::<Vec<_>>();let mut seen=normal.clone();
    for rank in &ranks{for &i in rank{for at in [i,i^1]{if selected.len()<64&&at<all.len()&&seen.insert(at){selected.push(at);}}}}
    if normal.len()!=32||selected.iter().any(|i|!seen.contains(&(i^1))){return Err(bad("FULL_FIT B normal/mate coverage"));}
    let es=selected.iter().map(|&i|all[i].clone()).collect::<Vec<_>>();let expected=selected.iter().map(|&i|raw[i].clone()).collect::<Vec<_>>();
    let teacher=posthoc.is_some()||[1536,3072].contains(&p.local);let manifest=binary::record!({"policy":digest(s)?,"native":p.physical,"cases":digest(&es)?,"normal":normal,"selected":selected,"teacher_indices":if teacher{vec![0,1,16,17,32,33,60,61]}else{vec![]}});
    let(index,mut ctl)=if let Some(d)=posthoc{fit_posthoc_start(d,s,"review-F",&manifest)?}else{start_observation(s,"review-F",&manifest)?};
    let result=(||->Result<()>{let l=checkpoint::load(&p.native,Backend::Metal0.open()?,false)?;
        let binding=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"native":p.physical,"cases":digest(&es)?,"call_protocol":1});
        let actual=generated(&l,&s.root,"review-F",&es,&binding,&mut ctl)?;
        for(a,b)in actual.iter().zip(&expected){for field in ["raw_tokens","actual","finish_reason","error","generation_completed"]{if a[field]!=b[field]{return Err(bad("FULL_FIT B raw mismatch"));}}}
        if teacher{let(cases,expected)=event_read_teachers(s,0,p,0)?;let fixed=[0usize,1,16,17,32,33,60,61];let es=fixed.iter().map(|&i|cases[i].clone()).collect::<Vec<_>>();
            let binding=binary::record!({"policy":digest(s)?,"source":s.source,"runtime":s.runtime,"native":p.physical,"cases":digest(&es)?,"call_protocol":1});
            let rows=teacher_prefix(&s.root,"review-F-teacher-0",&binding,&l,&es,&mut ctl)?;
            diagnosis::validate_teacher_forward(&s.runtime,s.config.seq_len,&es,&rows,&l.tokenizer)?;
            for(r,&i)in rows.iter().zip(&fixed){let a=&r["teacher"]["target_token_observation"];let b=&expected[i]["teacher"]["target_token_observation"];
                if a["gold"]!=b["gold"]||a["argmax"]!=b["argmax"]{return Err(bad("FULL_FIT B teacher IDs mismatch"));}
                let x:Vec<f64>=binary::from_value(a["nll"].clone())?;let y:Vec<f64>=binary::from_value(b["nll"].clone())?;
                if x.len()!=y.len()||x.iter().zip(y).any(|(a,b)|(a-b).abs()>1e-5){return Err(bad("FULL_FIT B teacher NLL mismatch"));}}
        }ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,"review-F",index,&mut ctl,&result,binary::record!({"normal":32,"selected":es.len(),"teacher":if teacher{8}else{0},"teacher_scope":if teacher{"FINAL_SCHEDULED"}else{"NOT_RUN_NOT_SCHEDULED"}}))?;result
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
        baseline_hash:file_hash(&baseline_raw)?,baseline_raw,max_updates:1024,generation_cap:6400,teacher_cap:6400,active_cap:7200.,segment_cap:900.,bytes_cap:1610612736,event:None,full_fit:None,first_decision:None};
    for arm in ARMS {std::fs::create_dir(root.join(arm))?;}
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    let roles=[OptimizerProtocol::new(&l.model.config,false,digest(&s.runtime)?,st.step)?,OptimizerProtocol::new(&l.model.config,true,digest(&s.runtime)?,st.step)?];
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"policy":digest(&s)?,"parent":s.parent_hash,"content":s.parent_content,"parent_adam_preserved":s.parent_adam,"roles":roles,"fresh_state":"all zeros on allocation; no inherited moments","costs":s.costs,"optimizer_calls":0,"generation_calls":0,"phase_order":[["A",32],["M",32],["M",128],["A",128],["A",512],["M",512],["M",1024],["A",1024]]}))?;
    println!("MUON_PREPARED policy={} parent={} input={input} target={target} padding={padding} new_optimizer0 A_PENDING",digest(&s)?,s.parent_hash);Ok(())
}

fn load_study(root:&Path,execute:bool)->Result<Study>{
    let s:Study=read_confirmed(&root.join("plan.r3b"))?;
    if s.contract!=if s.first_decision.is_some(){FIRST_CONTRACT}else if s.full_fit.is_some(){FIT_CONTRACT}else if s.event.is_some(){EVENT_CONTRACT}else{CONTRACT}||s.root!=root.canonicalize()?{return Err(bad("study root/contract binding"));}
    inputs(&s)?;
    if s.event.is_some(){for arm in 0..s.arms().len(){event_inputs(&s,arm)?;}}
    if s.first_decision.is_some(){first_verify_inputs(&s)?;}else if s.full_fit.is_some(){fit_verify_inputs(&s)?;}
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
    if r["contract"]!=s.contract||r["source"]!=s.source||r["policy"]!=digest(&s)?||r["runtime"]!=binary::record!(s.runtime)
        ||r["verdict"]!="PASS"||(s.event.is_none()&&r["tests_passed"].as_u64().is_none_or(|n|n<6))
        ||(s.first_decision.is_some()&&r["boundaries"]!=binary::record!(["first_objective_gradient","inherited_native_process","pair_dispatch_budget"]))
        ||(s.first_decision.is_none()&&s.full_fit.is_some()&&r["boundaries"]!=binary::record!(["full_input_lineage","rotated_tape_exposure","inherited_native_process","single_arm_guard","large_count_reader","final_no_call"]))
        ||(s.full_fit.is_none()&&s.event.is_some()&&r["boundaries"]!=binary::record!(["request_labels","strict_scorer_gate","tape_cost","inherited_native_process","actual_teacher_modes","final_no_call"]))
        ||r["active_seconds"].as_f64().is_none_or(|v|!v.is_finite()||v<0.||v>=s.active_cap) {
        return Err(bad("independent A incomplete/mismatch"));
    }
    publish_confirmed(&root.join("review-a.r3b"),&binary::record!({"accepted":true,"policy":digest(&s)?,"source":s.source,"report_path":review.canonicalize()?,"report_hash":file_hash(review)?,"active_seconds":r["active_seconds"]}))
}

#[derive(Clone,Serialize,Deserialize)]
struct Progress { local:usize,native:PathBuf,physical:String,stop:Option<String>,evaluated:usize,fit:bool }
#[derive(Clone,Serialize,Deserialize)]
struct Segment { policy:String,previous:Option<String>,phase:usize,arms:Vec<Progress>,success:bool,resume:bool,control:binary::Value,error:Option<String> }
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
            ||seg.arms.len()!=s.arms().len()||seg.arms.iter().any(|a|a.local>s.max_updates||a.evaluated>a.local) {return Err(bad("segment chain/policy/cursor"));}
        for a in &seg.arms {if file_hash(&a.native)?!=a.physical{return Err(bad("durable native changed"));}}
        out.push(seg);
    }Ok(out)
}
fn previous_usage(s:&Study,h:&[Segment])->Result<(f64,usize,usize)> {
    let a:binary::Value=read_confirmed(&s.root.join("review-a.r3b"))?;
    let mut time=a["active_seconds"].as_f64().ok_or_else(||bad("A active usage unknown"))?;let(mut generations,mut teacher)=(0,0);
    let labels=std::iter::once("baseline".to_owned()).chain(s.arms().iter().map(|a|format!("review-{a}")));
    let observations=labels.map(|label|observation_history(s,&label)).collect::<Result<Vec<_>>>()?;
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
    let h=history(s)?;let c=control(s,&h)?;start_observation_control(s,label,binding,c)
}
fn start_observation_control(s:&Study,label:&str,binding:&binary::Value,c:RunControl)->Result<(usize,RunControl)>{
    let previous=observation_history(s,label)?;
    if previous.last().is_some_and(|r|r["success"]==true){return Err(bad("observation already complete; pure report only"));}
    let i=previous.len();
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
fn first_owned_bytes(s:&Study)->Result<u64>{
    let parent=s.root.parent().ok_or_else(||bad("first-decision cost parent"))?;
    let mut bytes=0u64;
    for e in std::fs::read_dir(parent)?{let e=e?;let name=e.file_name();
        if name.to_string_lossy().starts_with("first-decision-20260925-"){
            let m=e.metadata()?;bytes+=if m.is_dir(){owned_bytes(&e.path())?}else{m.len()};
        }
    }
    let terminal=s.first_decision.as_ref().and_then(|f|std::fs::read_dir(&f.original).ok()).and_then(|items|
        items.filter_map(|e|e.ok()).filter(|e|{let n=e.file_name();let n=n.to_string_lossy();n.starts_with("segment-")&&n.ends_with("-finished.r3b")})
            .max_by_key(|e|e.file_name()));
    let cutoff=if let Some(entry)=terminal{entry.metadata()?.modified()?}
        else if first_fixture(s){std::time::UNIX_EPOCH}
        else{return Err(bad("first-decision original terminal cost boundary"));};
    let release=parent.parent().ok_or_else(||bad("first-decision project root"))?.join("target/release");
    let deps=release.join("deps");let mut counted=BTreeSet::new();
    for dir in [&release,&deps]{if !dir.exists(){continue;}
        for item in std::fs::read_dir(dir)?{let item=item?;let name=item.file_name();let name=name.to_string_lossy();
            if item.path().extension().is_none()&&item.file_type()?.is_file()
                &&(name.starts_with("replica-")||name.starts_with("replica_"))&&item.metadata()?.modified()?>=cutoff{
                counted.insert(item.path());bytes+=item.metadata()?.len();
            }
        }
    }
    let executable=std::env::current_exe()?;
    if !executable.starts_with(parent)&&!counted.contains(&executable){
        bytes+=std::fs::metadata(executable)?.len();
    }
    Ok(bytes)
}
fn guard_bytes(s:&Study,reserve:u64)->Result<()> {
    let actual=if s.first_decision.is_some(){first_owned_bytes(s)?}else{owned_bytes(&s.root)?};
    if actual.saturating_add(reserve)>s.bytes_cap{return Err(bad("BLOCKED_DISK: study immutable byte cap"));}Ok(())
}
fn bind(s:&Study,arm:usize,state:&TrainingState,tok:&ByteBpe)->Result<checkpoint::ResumeBinding>{
    let mut b=checkpoint::ResumeBinding::default_for(state,tok);b.family=checkpoint::ANSWER_MEAN_FAMILY;b.normalizer=2;b.execution=1;
    if let Some(first)=&s.first_decision {if arm==1{b.family=7;b.span_alpha_bits=Some(2f64.to_bits());b.annotation=Some(first.role_map);}}
    b.policy=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&(s,s.arms()[arm]))?);b.provenance=b.policy;
    b.train_order=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&s.tape)?);b.framing=neural::Framing::QuestionEvidence.digest();Ok(b)
}
fn load_arm(s:&Study,arm:usize,p:&Progress,device:&Device)->Result<(checkpoint::Loaded,Optimizer)> {
    let mut l=checkpoint::load(&p.native,device.clone(),true)?;let mut state=l.manifest.training.clone().ok_or_else(||bad("missing native training state"))?;
    let corpus=if s.event.is_some(){event_inputs(s,arm)?}else{verified_corpus(&s.word_root.join("corpus.r3cor"),&s.corpus_hash)?};
    let opt=if p.local==0 {
        if p.physical!=s.parent_hash||state.step!=s.parent_step||l.model.weights_content_id()?!=s.parent_content||optimizer_hash(&l.optimizer)?!=s.parent_adam{return Err(bad("fresh parent weights/Adam provenance"));}
        if let Some(first)=&s.first_decision {
            let original:Study=read_confirmed(&first.original.join("plan.r3b"))?;
            if state.sampler_state!=3069||state.resume_binding!=Some(bind(&original,0,&state,&l.tokenizer)?)
                ||l.manifest.optimizer_protocol.as_ref()!=Some(&s.event.as_ref().unwrap().optimizer){return Err(bad("first-decision explicit ANSWER parent fork"));}
        }
        state.parent_checkpoint_hash=Some(s.parent_hash.clone());state.config=s.config.clone();state.corpus_hash=corpus.manifest.train.sha256.clone();
        state.validation_hash=corpus.manifest.validation.sha256.clone();
        if !state.previous_corpora.contains(&l.tokenizer.train_hash){state.previous_corpora.push(l.tokenizer.train_hash.clone());}
        state.sampler_state=0;state.resume_binding=Some(bind(s,arm,&state,&l.tokenizer)?);
        if let Some(e)=&s.event {
            let original=l.manifest.optimizer_protocol.as_ref().ok_or_else(||bad("inherited native optimizer missing"))?;
            if original!=&e.optimizer{return Err(bad("inherited parent optimizer identity"));}
            let mut protocol=original.clone();protocol.version=2;protocol.family="ADAMW-INHERITED-V1".into();protocol.study_start_step=Some(s.parent_step);protocol.runtime_digest=digest(&s.runtime)?;
            Optimizer{protocol,tensors:std::mem::take(&mut l.optimizer)}
        }else{Optimizer::fresh(&l.model.config,&l.model.vars,arm==1,digest(&s.runtime)?,s.parent_step)?}
    } else {
        let protocol=l.manifest.optimizer_protocol.clone().ok_or_else(||bad("optimizer identity absent"))?;
        protocol.validate(&l.model.config,&state)?;
        let mut expected=if let Some(e)=&s.event{let mut v=e.optimizer.clone();v.version=2;v.family="ADAMW-INHERITED-V1".into();v.study_start_step=Some(s.parent_step);v.runtime_digest=digest(&s.runtime)?;v}else{OptimizerProtocol::new(&l.model.config,arm==1,digest(&s.runtime)?,s.parent_step)?};expected.local_step=s.clock(p.local);
        if protocol!=expected||state.config!=s.config||state.sampler_state!=p.local as u64||state.corpus_hash!=corpus.manifest.train.sha256
            ||state.validation_hash!=corpus.manifest.validation.sha256||state.parent_checkpoint_hash.as_deref()!=Some(s.parent_hash.as_str())
            ||state.resume_binding!=Some(bind(s,arm,&state,&l.tokenizer)?){return Err(bad("wrong optimizer policy/runtime/local clock"));}
        Optimizer{protocol,tensors:std::mem::take(&mut l.optimizer)}
    };
    if file_hash(&p.native)?!=p.physical||l.tokenizer.semantic_id()!=s.tokenizer{return Err(bad("native physical/tokenizer binding"));}
    opt.protocol.validate(&l.model.config,&state)?;opt.validate(&l.model.vars)?;l.manifest.training=Some(state);l.manifest.optimizer_protocol=Some(opt.protocol.clone());l.optimizer.clear();Ok((l,opt))
}
fn save_arm(s:&Study,index:usize,arm:usize,l:&mut checkpoint::Loaded,o:&Optimizer,p:&mut Progress)->Result<()> {
    l.model.device.synchronize()?;l.manifest.optimizer_protocol=Some(o.protocol.clone());l.manifest.status="DIAGNOSTIC_COMPLETE".into();
    let path=s.root.join(s.arms()[arm]).join(format!("segment-{index:03}-step-{}.r3m",p.local));
    let bytes=(l.model.vars.values().map(|v|v.elem_count()).sum::<usize>()+o.tensors.values().map(Tensor::elem_count).sum::<usize>())as u64*4+1024*1024;
    guard_bytes(s,bytes)?;
    checkpoint::save(&path,&l.model,&l.tokenizer,l.manifest.clone(),&o.tensors)?;
    p.physical=file_hash(&path)?;p.native=path;println!("DURABLE arm={} local={} absolute={} bytes={}",s.arms()[arm],p.local,s.parent_step+p.local,std::fs::metadata(&p.native)?.len());Ok(())
}
fn verify_output_result(row:&binary::Value,tok:&ByteBpe)->Result<()> {
    verify_generated(row,tok)?;
    // Preserve a proven command-capped RETURNED timeout at the same cursor.
    // Its immutable call resolution is checked by the collector, not inferred
    // from an absent result; no request timeout/cancel/UNKNOWN is admitted here.
    if row["row_version"]==2&&row["generation_started"]==true&&row["generation_completed"]==false
        &&row["generation"].is_null()&&row["actual"].is_null()&&row["decode_error"].is_null()
        &&row["error"]=="model: native generation timeout"&&row["generation_error"]==row["error"]
        &&row["error_class"]=="timeout"&&row["finish_reason"]=="timeout"
        &&row["timeout_cap_source"]=="command"&&row["command_stop"]=="TIME_BUDGET"&&row["interruption"]=="TIME_BUDGET"
        &&row["effective_timeout_ms"].as_u64().zip(row["original_timeout_ms"].as_u64()).is_some_and(|(a,b)|a>0&&a<b)
        &&row["raw_generated_count"].as_u64()==row["raw_tokens"].as_array().map(|v|v.len() as u64)
        &&(row["diagnostic_stop_error"].is_null()||row["diagnostic_stop_error"]=="model: TIME_BUDGET") {return Ok(());}
    if !row["generation_error"].is_null()
        || !row["command_stop"].is_null()&&row["command_stop"]!="TIME_BUDGET"
        || !row["diagnostic_stop_error"].is_null()&&row["command_stop"]!="TIME_BUDGET" {
        return Err(bad("native execution/command failure"));
    }
    if !row["error"].is_null(){
        // Only a verified normal RETURNED output can be a quality error.
        // Decode stays strict: invalid text remains null and never scores exact.
        if row["row_version"]!=2||row["generation_completed"]!=true
            ||row["decode_error"]!=row["error"]||row["error_class"]!="strict_utf8" {
            return Err(bad(&format!("native generation error {}",row["error"])));
        }
        let ids:Vec<u32>=binary::from_value(row["generation"]["tokens"].clone())?;
        let bytes=tok.decode_bytes(&ids)?;
        if std::str::from_utf8(&bytes).is_ok(){return Err(bad("unverified UTF-8 output error"));}
    }
    Ok(())
}
fn generated(l:&checkpoint::Loaded,root:&Path,label:&str,es:&[Episode],binding:&binary::Value,c:&mut RunControl)->Result<Vec<binary::Value>>{
    generated_until(l,root,label,es,binding,c,es.len())
}
fn generated_until(l:&checkpoint::Loaded,root:&Path,label:&str,es:&[Episode],binding:&binary::Value,c:&mut RunControl,until:usize)->Result<Vec<binary::Value>>{
    if until>es.len(){return Err(bad("generation prefix exceeds manifest"));}
    let path=root.join(format!("{label}.r3rows"));let mut entries=if path.exists(){binary::read_value_records(&path)?}else{vec![]};
    if entries.first().is_some_and(|v|v!=binding){return Err(bad("evaluation binding changed"));}
    let mut rows=if entries.is_empty(){vec![]}else{entries.drain(1..).collect()};
    if rows.len()>es.len(){return Err(bad("extra RETURNED rows"));}
    for(i,row)in rows.iter().enumerate(){call_attempt(root,label,"generation",binding,&es[i],i,Some(row))?;verify_output_result(row,&l.tokenizer)?;if row["id"]!=es[i].id||row["expected"]!=es[i].answer{return Err(bad("RETURNED case changed"));}}
    let mut f=std::fs::OpenOptions::new().append(true).create_new(!path.exists()).open(&path)?;
    if entries.is_empty(){append_row(&mut f,binding)?;std::fs::File::open(root)?.sync_all()?;}
    for(i,e)in es.iter().enumerate().take(until).skip(rows.len()){
        c.check("muon_next_generation")?;let attempt=prepare_call(root,label,"generation",binding,e,i)?;
        let mut row=match recovery::observe_generation(l,e,&e.request,c,false){ObservedCall::Returned(r)=>r,ObservedCall::NotInvoked(_)=>{resolve_call(&attempt,None,c)?;c.stop_result()?;return Err(bad("not invoked without stop"));}};
        row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),c)?;
        let verified=verify_output_result(&row,&l.tokenizer);rows.push(row);
        if verified.is_err(){c.stop_result()?;verified?;}
        if i%64==63{println!("PANEL {label} returned={}/{}",i+1,es.len());}
    }Ok(rows)
}
fn baseline(s:&Study)->Result<()> {
    if s.full_fit.is_some(){return fit_baseline(s);}
    if s.event.is_some(){return event_baseline(s);}
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
    if s.event.is_some(){return event_evaluate(s,arm,l,p,train,ctl);}
    l.model.refresh_identity()?;let model=l.model.weights_content_id()?;let dir=s.root.join(s.arms()[arm]);let mut out=BTreeMap::new();
    for(name,es,ms)in panels(s,p.local,train)?{
        let label=format!("eval-{}-{name}",p.local);
        let binding=binary::record!({"policy":digest(s)?,"arm":s.arms()[arm],"model":model,"local":p.local,"absolute":s.parent_step+p.local,"runtime":s.runtime,"cases":digest(&es)?,"metadata":digest(&ms)?,"tokenizer":s.tokenizer,"planned":es.len(),"call_protocol":1});
        let rows=generated(l,&dir,&label,&es,&binding,ctl)?;
        let mut score=scored(&name,&es,&ms,&rows,&l.tokenizer)?;score["raw_hash"]=binary::record!(file_hash(&dir.join(format!("{label}.r3rows")))?);score["model"]=binary::record!(model);
        if train{
            let teachers=teacher_prefix(&dir,&label,&binding,l,&es,ctl)?;score["teacher_nll"]=binary::record!(teacher_ce(&teachers)?);
            let(c,_,_)=inputs(s)?;let seen=s.tape[..p.local].iter().flatten().fold(BTreeMap::<usize,usize>::new(),|mut a,&i|{*a.entry(i).or_default()+=1;a});
            score["actual_train_exposures"]=binary::record!(es.iter().map(|e|c.train.iter().position(|x|x.id==e.id).map(|i|seen.get(&i).copied().unwrap_or(0))).collect::<Vec<_>>());
        }
        let path=dir.join(format!("{label}-score.r3b"));if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("score disagreement"));}}else{publish_confirmed(&path,&score)?;}
        println!("SCORE arm={} local={} {name} {}",s.arms()[arm],p.local,score["joint"]);out.insert(name,score);
    }Ok(out)
}

fn decision(s:&Study,arm:usize,p:&Progress,scores:&BTreeMap<String,binary::Value>)->Result<Option<String>>{
    if s.event.is_some(){return event_decision(s,arm,p,scores);}
    let dir=s.root.join(s.arms()[arm]);let mut streak=[0usize;3];let mut stop=None;
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
        let (full,all4,errors)=if total==64{(full,all4,j["errors"].as_u64().ok_or_else(||bad("guard missing error count"))?)}else{
            let panel=panels(s,p.local,false)?.into_iter().find(|v|v.0==*name).ok_or_else(||bad("guard panel"))?;
            let raw=binary::read_value_records(&dir.join(format!("eval-{}-{name}.r3rows",p.local)))?;
            let tok=scoring_tokenizer(s)?;let r=scored(name,&panel.1[..64],&panel.2[..64],&raw[1..65],&tok)?;
            (r["joint"]["full"].as_u64().unwrap(),r["joint"]["all4"].as_u64().unwrap(),r["joint"]["errors"].as_u64().ok_or_else(||bad("guard missing error count"))?)
        };
        streak[i]=if full<60||all4<12{streak[i]+1}else{0};
        if full<=48||errors>=4{stop=Some("SEVERE_RETENTION".to_owned());}
        else if streak[i]>=2&&stop.is_none(){stop=Some("PERSISTENT_RETENTION".to_owned());}
    }
    let d=binary::record!({"policy":digest(s)?,"local":p.local,"scores":scores,"streak":streak,"stop":stop});let path=dir.join(format!("decision-{}.r3b",p.local));
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=d{return Err(bad("guard decision mismatch"));}}else{publish_confirmed(&path,&d)?;}Ok(stop)
}
fn train_update(s:&Study,arm:usize,l:&mut checkpoint::Loaded,o:&mut Optimizer,p:&mut Progress,b:&Batch,ctl:&mut RunControl,entered:&mut bool,entry:&Path,start:Instant)->Result<binary::Value>{
    let logits=l.model.forward(&b.input,Some(&b.valid))?;
    let (plain,first_nll,loss,targets,weighted_denominator)=if s.first_decision.is_some(){
        let row=&s.tape[p.local];
        let roles=row.map(|i|i>=6144);
        if row[..4].iter().any(|&i|i>=6144)||row[4..].iter().any(|&i|!(6144..7680).contains(&i)){
            return Err(bad("first-decision row role policy"));
        }
        if arm==1{first_decision_objective(&logits,b,&roles)?}
        else {let(ce,answer,n,_)=response_objective(&logits,b,1.,true)?;
            let first=masked_loss(&logits,&b.target,&b.first_target_mask)?.0;(ce,first,answer,n,n)}
    }else{let(ce,answer,n,_)=response_objective(&logits,b,1.,true)?;
        let first=masked_loss(&logits,&b.target,&b.first_target_mask)?.0;(ce,first,answer,n,n)};
    let examples=b.mask.dim(0)?;
    let value=loss.to_scalar::<f32>()?;if !value.is_finite(){return Err(bad("nonfinite study loss"));}
    let graph=loss.backward()?;let mut grads=BTreeMap::new();
    for(n,v)in &l.model.vars{grads.insert(n.clone(),graph.get(v).ok_or_else(||bad("missing gradient"))?.detach());}
    l.model.device.synchronize()?;let fb=start.elapsed().as_secs_f64();
    if let Err(e)=ctl.check("before_study_optimizer"){
        publish_confirmed(&entry.with_file_name(format!("{}-discarded.r3b",entry.file_stem().unwrap().to_string_lossy().trim_end_matches("-entered"))),&binary::record!({"input":b.tokens,"target":targets,"forward_backward":1,"optimizer":0,"reason":e.to_string()}))?;
        return Err(e);
    }
    *entered=true;let began=Instant::now();
    let stats=o.step(&l.model.vars,&grads,&s.config,[1,32,128,512,1024].contains(&(p.local+1))||s.event.is_some()&&p.local+1==64)?;
    l.model.device.synchronize()?;let opt=began.elapsed().as_secs_f64();*entered=false;p.local+=1;
    let st=l.manifest.training.as_mut().unwrap();st.step=s.parent_step+p.local;st.sampler_state=p.local as u64;st.consumed_tokens+=b.tokens as u64;st.target_tokens+=targets as u64;st.train_loss=Some(value as f64);
    l.manifest.optimizer_protocol=Some(o.protocol.clone());
    Ok(binary::record!({"local":p.local,"model_step":st.step,"optimizer_local":o.protocol.local_step,"rows":s.tape[p.local-1],"input":b.tokens,"target":targets,"padding":b.input.elem_count()-b.tokens,"examples":examples,"answer_ce":value,"plain_ce":plain.to_scalar::<f32>()?,"first_nll":first_nll.to_scalar::<f32>()?,"weighted_denominator":weighted_denominator,"lr":s.config.lr,"stats":stats,"forward_backward_seconds":fb,"optimizer_seconds":opt,"step_seconds":start.elapsed().as_secs_f64(),"arm":s.arms()[arm]}))
}
fn train(s:&Study)->Result<()> {
    if s.full_fit.is_some(){return fit_train(s);}
    let baseline:binary::Value=read_confirmed(&s.root.join("baseline-finished.r3b"))?;if baseline["success"]!=true{return Err(bad("baseline not accepted"));}
    let h=history(s)?;if h.last().is_some_and(|s|!s.resume){return Err(bad("closed pair; no resume"));}
    if s.event.is_some()&&baseline["extra"]["id_sufficient"]==true{return Err(bad("BASELINE_ID_CONTRACT_SUFFICIENT; no learning"));}
    let mut ctl=control(s,&h)?;let index=h.len();
    let initial=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
    let mut ps=h.last().map(|h|h.arms.clone()).unwrap_or(vec![initial.clone(),initial]);let mut phase=h.last().map_or(0,|h|h.phase);
    let previous=if index==0{None}else{Some(file_hash(&s.root.join(format!("segment-{:03}-finished.r3b",index-1)))?)};
    publish_confirmed(&s.root.join(format!("segment-{index:03}-started.r3b")),&binary::record!({"policy":digest(s)?,"previous":previous,"phase":phase,"arms":ps}))?;
    let device=Backend::Metal0.open()?;let mut models=vec![];let saved_local=ps.iter().map(|p|p.local).collect::<Vec<_>>();
    let mut optimizer_entered=false;
    let result=(||->Result<()> {
        // Native load is not a model call. Complete RETURNED evaluation can be
        // sealed at zero remaining time; every new forward is checked below.
        for arm in 0..2 {models.push(load_arm(s,arm,&ps[arm],&device)?);}
        if models[0].0.tokenizer.semantic_id()!=s.tokenizer||models[1].0.tokenizer.semantic_id()!=s.tokenizer{return Err(bad("tokenizer changed"));}
        if index==0 && s.event.is_none() {
            if models[0].0.model.weights_content_id()?!=models[1].0.model.weights_content_id()?||models.iter().any(|(_,o)|o.protocol.local_step!=0){return Err(bad("fresh pair initial state"));}
            for(_,o)in &models {for t in o.tensors.values(){if norm(t)?!=0.{return Err(bad("nonzero fresh optimizer"));}}}
            publish_confirmed(&s.root.join("fresh-state.r3b"),&binary::record!({"parent":s.parent_hash,"content":s.parent_content,"both_identical":true,"state0":true,"optimizer_reset":"all states only; weights retained"}))?;
        }
        let(c,_,_)=inputs(s)?;
        let samples=if phase<8{samples_with_framing(&c.train,&models[0].0.tokenizer,s.config.seq_len,neural::Framing::QuestionEvidence)?}else{vec![]};
        let event_samples=if s.event.is_some(){(0..2).map(|a|samples_with_framing(&event_inputs(s,a)?.train,&models[a].0.tokenizer,s.config.seq_len,neural::Framing::QuestionEvidence)).collect::<Result<Vec<_>>>()?}else{vec![]};
        if index==0&&s.event.is_some(){if models.iter().any(|(l,o)|l.model.weights_content_id().ok().as_ref()!=Some(&s.parent_content)||optimizer_hash(&o.tensors).ok().as_ref()!=Some(&s.parent_adam)||o.protocol.local_step!=s.clock(0)){return Err(bad("inherited initial tensors/clock"));}}
        let phases:Vec<(usize,usize)>=if s.event.is_some(){vec![(0,64),(1,64),(1,128),(0,128)]}else{vec![(0,32),(1,32),(1,128),(0,128),(0,512),(1,512),(1,1024),(0,1024)]};
        loop {
            let stopping=ps.iter().filter(|p|p.stop.is_some()).map(|p|p.local).min();
            if phase>=phases.len()||stopping.is_some_and(|n|ps.iter().all(|p|p.local==n&&p.evaluated==n)){break;}
            let (arm,endpoint)=if index==0 {if ps[0].local==0{(0,1)}else if ps[1].local==0{(1,1)}else{break;}}else{phases[phase]};
            if stopping.is_some_and(|n|endpoint>n){return Err(bad("peer endpoint exceeds quality stop"));}
            let(l,o)=&mut models[arm];let p=&mut ps[arm];
            let path=s.root.join(s.arms()[arm]).join(format!("updates-{index:03}.r3rows"));
            let mut trace=std::fs::OpenOptions::new().append(true).create(true).open(&path)?;
            while p.local<endpoint {
                ctl.check("before_study_microbatch")?;guard_bytes(s,0)?;
                let b=batch(if s.event.is_some(){&event_samples[arm]}else{&samples},&s.tape[p.local],&device)?;device.synchronize()?;let start=Instant::now();
                let entry=s.root.join(s.arms()[arm]).join(format!("step-{}-segment-{index:03}-entered.r3b",p.local+1));
                publish_confirmed(&entry,&binary::record!({"policy":digest(s)?,"local":p.local+1,"rows":s.tape[p.local],"status":"MAY_ENTER"}))?;
                let row=train_update(s,arm,l,o,p,&b,&mut ctl,&mut optimizer_entered,&entry,start)?;
                append_row(&mut trace,&row)?;
                if p.local==1||p.local%8==0{println!("STUDY_UPDATE arm={} local={} model={} clock={} input={} target={} loss={} delta={} remaining={}",s.arms()[arm],p.local,row["model_step"],row["optimizer_local"],row["input"],row["target"],row["answer_ce"],row["stats"]["delta_norm"],s.max_updates-p.local);}

            }
            if index==0 {continue;}
            let scores=evaluate(s,arm,l,p,false,&mut ctl)?;p.stop=decision(s,arm,p,&scores)?;p.evaluated=p.local;
            if p.local==512||s.event.is_some(){save_arm(s,index,arm,l,o,p)?;}
            phase+=1;
        }
        let end=phase>=phases.len()||ps.iter().any(|p|p.stop.is_some())&&ps[0].local==ps[1].local&&ps.iter().all(|p|p.evaluated==p.local);
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
    println!("MUON_SEGMENT index={index} phase={phase} local={:?} resume={} stop={:?} active={} new_bytes={}",seg.arms.iter().map(|p|p.local).collect::<Vec<_>>(),seg.resume,seg.arms.iter().map(|p|&p.stop).collect::<Vec<_>>(),seg.control["elapsed_seconds"],owned_bytes(&s.root)?);
    if pure_time{Ok(())}else{result}
}

fn first_report(s:&Study)->Result<()> {
    let h=history(s)?;let end=h.last().ok_or_else(||bad("first-decision NOT_RUN"))?;
    let samples=samples_with_framing(&event_inputs(s,0)?.train,&scoring_tokenizer(s)?,s.config.seq_len,neural::Framing::QuestionEvidence)?;
    for arm in 0..2{
        let p=&end.arms[arm];let mut count=0usize;let(mut input,mut target,mut padding)=(0usize,0usize,0usize);
        for index in 0..h.len(){let path=s.root.join(s.arms()[arm]).join(format!("updates-{index:03}.r3rows"));
            if !path.exists(){continue;}
            for row in binary::read_value_records(&path)?{
                let draw=s.tape.get(count).ok_or_else(||bad("first-decision trace overrun"))?;count+=1;
                let length=draw.iter().map(|&i|samples[i].tokens.len()-1).max().unwrap();let mut cost=[0usize;3];
                for &i in draw{cost[0]+=samples[i].tokens.len()-1;cost[1]+=samples[i].tokens.len()-samples[i].response_start;cost[2]+=length-(samples[i].tokens.len()-1);}
                if row["local"]!=count||row["model_step"]!=17981+count||row["optimizer_local"]!=3645+count
                    ||row["source_tape_index"]!=(573+count-1)%3072||row["rows"]!=binary::record!(draw)
                    ||row["input"]!=cost[0]||row["target"]!=cost[1]||row["padding"]!=cost[2]
                    ||row["examples"]!=8||row["weighted_denominator"]!=cost[1]+if arm==1{4}else{0}
                    ||row["plain_ce"].as_f64().is_none_or(|x|!x.is_finite())
                    ||row["first_nll"].as_f64().is_none_or(|x|!x.is_finite())
                    ||row["answer_ce"].as_f64().is_none_or(|x|!x.is_finite()){
                    return Err(bad("first-decision trace/objective/cost/clock"));
                }
                input+=cost[0];target+=cost[1];padding+=cost[2];
            }
        }
        let discarded=first_discarded(s,arm)?;
        if count!=p.local||count>256||count+discarded[0]>264{return Err(bad("first-decision committed/backward cap"));}
        let(m,_)=checkpoint::metadata(&p.native)?;let st=m.training.as_ref().ok_or_else(||bad("first-decision endpoint state"))?;
        let op=m.optimizer_protocol.as_ref().ok_or_else(||bad("first-decision endpoint Adam"))?;
        let binding=if count==0{
            let first=s.first_decision.as_ref().ok_or_else(||bad("first-decision parent profile"))?;
            let original:Study=read_confirmed(&first.original.join("plan.r3b"))?;
            st.sampler_state==3069&&p.physical==s.parent_hash
                &&st.resume_binding==Some(bind(&original,0,st,&scoring_tokenizer(s)?)?)
        }else{st.sampler_state==count as u64&&st.resume_binding==Some(bind(s,arm,st,&scoring_tokenizer(s)?)?)};
        if st.step!=17981+count||op.local_step!=3645+count||!binding||file_hash(&p.native)?!=p.physical{
            return Err(bad("first-decision endpoint native/binding"));
        }
        if p.fit||p.evaluated>0{
            let model=m.model_content_digest;
            let mut recounted=BTreeMap::new();
            for panel in fit_panels(s)?.into_iter().take(if p.fit{6}else{5}){
                let n=first_panel_count(s,p.fit&&["FULL-word","FULL-renamed","FULL-train"].contains(&panel.0.as_str()));
                let(score,_)=event_read_panel(s,s.arms()[arm],p.evaluated,&model,&panel,n)?;
                recounted.insert(panel.0,score);
            }
            let regular=s.root.join(s.arms()[arm]).join(format!("decision-{}.r3b",p.evaluated));
            let cost=s.root.join(s.arms()[arm]).join(format!("cost-decision-{}.r3b",p.evaluated));
            let guard:binary::Value=read_confirmed(if cost.exists(){&cost}else{&regular})?;
            let expected_cost=p.stop.as_ref().filter(|v|first_cost_reason(v));
            let quality=guard["quality_stop"].as_str();
            if guard["stop"]!=binary::record!(p.stop)||guard["cost_stop"]!=binary::record!(expected_cost)
                ||quality.is_some_and(|v|!["SEVERE_RETENTION","PERSISTENT_RETENTION"].contains(&v))
                ||(expected_cost.is_none()&&guard["quality_stop"]!=binary::record!(p.stop))
                ||guard["policy"]!=digest(s)?{return Err(bad("first-decision guard/endpoint"));}
            if cost.exists(){
                let old:binary::Value=read_confirmed(&regular)?;
                if p.evaluated!=128||expected_cost.is_none()||old["policy"]!=digest(s)?||!old["stop"].is_null()
                    ||!old["cost_stop"].is_null()||!old["quality_stop"].is_null()
                    ||guard["prior_decision_hash"]!=file_hash(&regular)?{return Err(bad("first-decision cost/prior guard identity"));}
            }
            if expected_cost.is_some()&&guard["scores"]!=binary::record!(recounted){return Err(bad("first-decision cost final scores"));}
        }
        println!("FIRST_TRACE arm={} committed={} backward={} discarded={} model={} Adam={} input={} target={} padding={} evaluated={} fit={} stop={:?}",s.arms()[arm],count,count+discarded[0],discarded[0],st.step,op.local_step,input+discarded[1],target+discarded[2],padding,p.evaluated,p.fit,p.stop);
    }
    println!("FIRST_USAGE active={} generation={} teacher={} scoped_bytes={} comparison={} Goal1=false",previous_usage(s,&h)?.0,previous_usage(s,&h)?.1,previous_usage(s,&h)?.2,first_owned_bytes(s)?,if end.arms[0].local==end.arms[1].local{"MATCHED"}else{"UNMATCHED_COST_ENDPOINT"});
    Ok(())
}
fn first_review_cases(s:&Study,arm:usize,p:&Progress)->Result<(Vec<Episode>,Vec<binary::Value>,binary::Value)> {
    let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
    let mut cases=vec![];let mut expected=vec![];let mut groups=[vec![],vec![]];let mut selected=BTreeSet::new();
    for(panel,take)in fit_panels(s)?.into_iter().take(5).zip([4,3,3,3,3]){
        let n=if ["FULL-word","FULL-renamed"].contains(&panel.0.as_str()){192}else{64};
        let(_,rows)=event_read_panel(s,s.arms()[arm],p.local,&model,&panel,n)?;
        for i in 0..take{selected.insert(panel.1[i].id.clone());cases.push(panel.1[i].clone());expected.push(rows[i].clone());}
        if let Some(which)=["FULL-word","FULL-renamed"].iter().position(|&v|v==panel.0){
            for base in (0..n).step_by(4){let range=base..base+4;
                if range.clone().any(|i|selected.contains(&panel.1[i].id))||!range.clone().any(|i|rows[i]["exact_match"]!=true){continue;}
                groups[which].push(range.map(|i|(panel.1[i].clone(),rows[i].clone())).collect::<Vec<_>>());
            }
        }
    }
    for round in 0..2{for panel in 0..2{if let Some(group)=groups[panel].get(round){
        for(e,r)in group{if !selected.insert(e.id.clone()){return Err(bad("first-decision B duplicate failure ID"));}
            cases.push(e.clone());expected.push(r.clone());}
    }}}
    if cases.len()>32||cases.len()!=selected.len(){return Err(bad("first-decision B unique case cap"));}
    let manifest=binary::record!({"normal":16,"failure_groups":(cases.len()-16)/4,"cases":digest(&cases)?,
        "ids":cases.iter().map(|e|e.id.clone()).collect::<Vec<_>>(),"requests":cases.iter().map(|e|digest(&e.request)).collect::<Result<Vec<_>>>()?});
    Ok((cases,expected,manifest))
}
fn first_review(s:&Study,arm:usize)->Result<()> {
    first_report(s)?;
    let h=history(s)?;let end=h.last().ok_or_else(||bad("first-decision B endpoint"))?;
    if end.resume||!end.success||!end.arms.iter().all(|p|p.fit){
        return Err(bad("first-decision B requires completed actual endpoints"));
    }
    if end.arms[0].local!=end.arms[1].local
        && !end.arms.iter().any(|p|p.stop.as_deref()==Some("BUDGET_EXHAUSTED")){
        return Err(bad("first-decision B unmatched without cost stop"));
    }
    let p=&end.arms[arm];let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
    let(cases,expected,manifest)=first_review_cases(s,arm,p)?;
    let train=first_teacher_cases(s,true)?;let dev=first_teacher_cases(s,false)?;
    let positions=first_review_teacher_positions(&train,&dev)?;
    let main=train.into_iter().chain(dev).collect::<Vec<_>>();
    let teacher=positions.iter().map(|&i|main[i].clone()).collect::<Vec<_>>();
    let label=format!("review-{}",s.arms()[arm]);
    let(index,mut ctl)=start_observation(s,&label,&binary::record!({"policy":digest(s)?,"native":p.physical,"selection":manifest,"teacher":8}))?;
    let result=(||->Result<()>{let l=checkpoint::load(&p.native,Backend::Metal0.open()?,false)?;
        let binding=binary::record!({"policy":digest(s)?,"model":model,"cases":digest(&cases)?,"planned":cases.len(),"call_protocol":1});
        let rows=generated(&l,&s.root,&label,&cases,&binding,&mut ctl)?;
        for(a,b)in rows.iter().zip(&expected){for field in ["raw_tokens","actual","finish_reason","error","generation_completed"]{
            if a[field]!=b[field]{return Err(bad("first-decision B generation mismatch"));}
        }}
        let raw=binary::read_value_records(&s.root.join(s.arms()[arm]).join(format!("teacher-{}-FULL-teachers.r3rows",p.local)))?;
        let mut origin=vec![];for &at in &positions{origin.push(raw.get(at+1).ok_or_else(||bad("first-decision B teacher raw"))?.clone());}
        let tbind=binary::record!({"policy":digest(s)?,"model":model,"cases":digest(&teacher)?,"planned":8,"call_protocol":1});
        let replay=teacher_prefix(&s.root,&label,&tbind,&l,&teacher,&mut ctl)?;
        for(a,b)in replay.iter().zip(&origin){if a["gold"]!=b["gold"]||a["argmax"]!=b["argmax"]{return Err(bad("first-decision B teacher IDs"));}
            let x:Vec<f64>=binary::from_value(a["nll"].clone())?;let y:Vec<f64>=binary::from_value(b["nll"].clone())?;
            if x.len()!=y.len()||x.iter().zip(y).any(|(x,y)|(x-y).abs()>1e-5){return Err(bad("first-decision B teacher NLL"));}}
        ctl.seal_completed_no_call()?;Ok(())})();
    finish_observation(s,&label,index,&mut ctl,&result,binary::record!({"normal":16,"generation":cases.len(),"teacher":8,"native":p.physical}))?;result
}
fn report(s:&Study)->Result<()> {
    if s.first_decision.is_some(){return first_report(s);}
    if s.full_fit.is_some(){return fit_report(s);}
    if s.event.is_some(){return event_report(s);}
    let h=history(s)?;let end=h.last().ok_or_else(||bad("NOT_RUN"))?;let tok=scoring_tokenizer(s)?;
    for arm in 0..2 {let p=&end.arms[arm];
        for at in [32,128,512,1024].into_iter().filter(|&v|v<=p.evaluated){for(name,es,ms)in panels(s,at,false)?{
            let label=format!("eval-{at}-{name}");let dir=s.root.join(s.arms()[arm]);let raw=binary::read_value_records(&dir.join(format!("{label}.r3rows")))?;
            if raw.len()!=es.len()+1||raw[0]["policy"]!=digest(s)?||raw[0]["cases"]!=digest(&es)?{return Err(bad("raw panel binding/count"));}
            for(i,(e,row))in es.iter().zip(&raw[1..]).enumerate(){call_attempt(&dir,&label,"generation",&raw[0],e,i,Some(row))?;}
            let r=scored(&name,&es,&ms,&raw[1..],&tok)?;let saved:binary::Value=read_confirmed(&dir.join(format!("{label}-score.r3b")))?;
            if r.as_object().unwrap().iter().any(|(k,v)|saved[k]!=*v)||saved["raw_hash"]!=file_hash(&dir.join(format!("{label}.r3rows")))?{return Err(bad("raw score disagreement"));}
            if raw[0]["model"]!=saved["model"]||at==p.local&&saved["model"]!=checkpoint::metadata(&p.native)?.0.model_content_digest{return Err(bad("endpoint panel/native content mismatch"));}
            println!("MUON_RESULT arm={} local={at} panel={name} full={} QB={} SB={} ALL4={} value={} support={} outside={} parse={} EOS={}",s.arms()[arm],r["joint"]["full"],r["joint"]["query_both"],r["joint"]["swap_both"],r["joint"]["all4"],r["value_correct"],r["citation_support_correct"],r["valid_outside_id"],r["parse_failure_rows"],r["joint"]["eos"]);
        }}
        if p.fit {for(name,es,ms)in panels(s,p.local,true)?{
            let dir=s.root.join(s.arms()[arm]);let label=format!("eval-{}-{name}",p.local);let raw=binary::read_value_records(&dir.join(format!("{label}.r3rows")))?;
            if raw.len()!=es.len()+1||raw[0]["policy"]!=digest(s)?||raw[0]["cases"]!=digest(&es)?{return Err(bad("fit raw binding/count"));}
            for(i,(e,row))in es.iter().zip(&raw[1..]).enumerate(){call_attempt(&dir,&label,"generation",&raw[0],e,i,Some(row))?;}
            let score=scored(&name,&es,&ms,&raw[1..],&tok)?;let saved:binary::Value=read_confirmed(&dir.join(format!("{label}-score.r3b")))?;
            if score.as_object().unwrap().iter().any(|(k,v)|saved[k]!=*v)||saved["raw_hash"]!=file_hash(&dir.join(format!("{label}.r3rows")))?{return Err(bad("fit raw recount"));}
            let teachers=binary::read_value_records(&dir.join(format!("{label}-teachers.r3rows")))?;
            if teachers.len()!=es.len()+1||teachers[0]!=raw[0]{return Err(bad("fit teacher binding/count"));}
            for(i,(e,row))in es.iter().zip(&teachers[1..]).enumerate(){call_attempt(&dir,&label,"teacher",&raw[0],e,i,Some(row))?;}
            if saved["teacher_nll"].as_f64()!=Some(teacher_ce(&teachers[1..])?){return Err(bad("fit teacher NLL recount"));}
            println!("FIT arm={} local={} FULL={} ALL4={} NLL={} exposure={}",s.arms()[arm],p.local,score["joint"]["full"],score["joint"]["all4"],saved["teacher_nll"],saved["actual_train_exposures"]);
        }}
        let mut seen=vec![0usize;7680];let(mut input,mut target,mut padding,mut count)=(0u64,0u64,0u64,0usize);
        for index in 0..h.len(){let path=s.root.join(s.arms()[arm]).join(format!("updates-{index:03}.r3rows"));if !path.exists(){continue;}
            for row in binary::read_value_records(&path)?{count+=1;
                if row["local"]!=count||row["model_step"]!=s.parent_step+count||row["optimizer_local"]!=count||row["rows"]!=binary::record!(s.tape[count-1])||row["lr"]!=s.config.lr{return Err(bad("trace clock/tape/LR mismatch"));}
                for &i in &s.tape[count-1]{seen[i]+=1;}input+=row["input"].as_u64().ok_or_else(||bad("input UNKNOWN"))?;target+=row["target"].as_u64().ok_or_else(||bad("target UNKNOWN"))?;padding+=row["padding"].as_u64().ok_or_else(||bad("padding UNKNOWN"))?;
            }
        }
        if count!=p.local{return Err(bad("trace count mismatch"));}
        println!("TRACE arm={} updates={count} exposures={} unique={} input={input} target={target} padding={padding}",s.arms()[arm],count*8,seen.iter().filter(|&&v|v>0).count());
        if p.local==s.max_updates {let mut gate=true;
            for name in ["value","citation","S1Q1","word","renamed"]{
                let r:binary::Value=read_confirmed(&s.root.join(s.arms()[arm]).join(format!("eval-{}-{name}-score.r3b",p.local)))?;
                let (full,pair,all4,component)=if name=="word"||name=="renamed"{(183,88,44,190)}else{(488,232,116,508)};
                gate&=r["joint"]["full"].as_u64().is_some_and(|n|n>=full)&&r["joint"]["query_both"].as_u64().is_some_and(|n|n>=pair)&&r["joint"]["swap_both"].as_u64().is_some_and(|n|n>=pair)&&r["joint"]["all4"].as_u64().is_some_and(|n|n>=all4)&&r["joint"]["errors"]==0;
                if name!="value"{gate&=r["value_correct"].as_u64().is_some_and(|n|n>=component)&&r["citation_support_correct"].as_u64().is_some_and(|n|n>=component)&&r["valid_outside_id"]==0&&r["parse_failure_rows"]==0;}
            }
            println!("DEVELOPMENT_GATE arm={} pass={gate} general_QA_NOT_RUN Goal1=false",s.arms()[arm]);
        }
        println!("ENDPOINT arm={} local={} native={} hash={} stop={:?} fit={}",s.arms()[arm],p.local,p.native.display(),p.physical,p.stop,p.fit);
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

/// A diagnosis descriptor, deliberately incompatible with Study and its train command.
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct ValueReading {
    contract:String, root:PathBuf, evaluation:PathBuf, evaluation_hash:String,
    #[serde(default)] predecessor_plan_hash:String,
    #[serde(default)] revision_authority:String,
    original:PathBuf, original_hash:String, native:PathBuf, physical:String,
    model:String, tokenizer:PathBuf, tokenizer_hash:String, source:String,runtime:RuntimeProfile,
    executable:PathBuf,executable_hash:String,prior_executable:PathBuf,prior_executable_hash:String,
    test_executable:PathBuf,test_executable_hash:String,
    panels:Vec<Panel>, original_indices:[Vec<usize>;2], original_raw:[PathBuf;2],
    original_raw_hash:[String;2], exposure:Vec<usize>,semantic_other_exposure:Vec<usize>,lengths:Vec<binary::Value>,
    generation_cap:usize, token_cap:usize, active_cap:f64, segment_cap:f64, bytes_cap:u64,
}
fn value_selected(request:&ModelRequest)->Result<usize>{
    let mut query=request.input.splitn(3,' ');
    let entity=query.next().unwrap_or("");let context=query.next().unwrap_or("");
    if entity.is_empty()||context.is_empty()||query.next()!=Some(phrases(Intent::Current)[0])
        ||request.evidence.items.len()!=2{return Err(bad("value reading request grammar/record count"));}
    let mut selected=None;let mut ids=BTreeSet::new();
    for(i,r)in request.evidence.items.iter().enumerate(){
        if r.event_id<0||r.event_id.to_string().len()!=8||!ids.insert(r.event_id){return Err(bad("value reading event ID"));}
        let(re,rc,_)=parsed_record(r)?;
        if re==entity&&rc==context&&r.version_status=="current"{
            if selected.replace(i).is_some(){return Err(bad("value reading ambiguous current"));}
        }
    }
    selected.ok_or_else(||bad("value reading missing current"))
}
fn value_variant(e:&Episode,uniform:bool)->Result<Episode>{
    let selected=value_selected(&e.request)?;let other=1-selected;
    let mut out=e.clone();
    if uniform{
        let(_,_,value)=parsed_record(&e.request.evidence.items[selected])?;
        let old=&e.request.evidence.items[other].original_excerpt;
        let(prefix,rest)=old.split_once(" 값은 ").ok_or_else(||bad("value field boundary"))?;
        let prior=rest.strip_suffix("이다.").filter(|v|!v.is_empty()).ok_or_else(||bad("value field suffix"))?;
        let replacement=format!("{prefix} 값은 {value}이다.");
        if replacement==*old||prior==value{return Err(bad("uniform source values already equal"));}
        out.request.evidence.items[other].original_excerpt=replacement;
        let mut restored=out.request.evidence.items[other].clone();restored.original_excerpt=old.clone();
        if restored!=e.request.evidence.items[other]||out.request.evidence.items[selected]!=e.request.evidence.items[selected]{return Err(bad("uniform changed nonvalue field"));}
    }else{
        out.request.evidence.items=vec![e.request.evidence.items[selected].clone()];
    }
    let(re,rc,v)=parsed_record(&e.request.evidence.items[selected])?;
    let mut q=e.request.input.splitn(3,' ');
    if q.next()!=Some(re)||q.next()!=Some(rc)||out.answer!=format!("{v}입니다. [event:{}]",e.request.evidence.items[selected].event_id)
        ||out.request.system!=e.request.system||out.request.input!=e.request.input||out.request.limits!=e.request.limits{
        return Err(bad("value reading independent gold/selector mismatch"));
    }
    if uniform && (0..2).any(|i|parsed_record(&out.request.evidence.items[i]).map_or(true,|x|x.2!=v)){
        return Err(bad("uniform value mismatch"));
    }
    out.id=format!("{}/{}",e.id,if uniform{"U"}else{"S"});Ok(out)
}
fn value_indices(es:&[Episode],ms:&[Meta],train:bool)->Result<Vec<usize>>{
    if es.len()!=192||ms.len()!=192{return Err(bad("value reading original192 only"));}
    let pairs=["words0-1","words0-2","words0-3","words1-2","words1-3","words2-3"];
    let mut groups=BTreeMap::<String,BTreeMap<usize,BTreeMap<String,[Option<usize>;4]>>>::new();
    let mut ids=BTreeSet::new();
    for(i,(e,m))in es.iter().zip(ms).enumerate(){
        let(pair,version)=m.template.split_once("/id").ok_or_else(||bad("value reading pair metadata"))?;
        let version=match version{"0"=>0,"1" if train=>1,_=>return Err(bad("value reading ID version"))};
        if !pairs.contains(&pair)||m.view>3||m.split!=if train{"train"}else{"dev"}
            ||e.id!=m.id||e.family!=m.base||e.sequence!=m.base||e.id!=format!("{}/{}",m.base,m.view)
            ||!e.binding.ends_with(&format!("/{pair}"))||!ids.insert(e.id.clone()){
            return Err(bad("value reading case metadata/duplicate"));
        }
        let slots=groups.entry(pair.into()).or_default().entry(version).or_default().entry(e.binding.clone()).or_insert([None;4]);
        if slots[m.view].replace(i).is_some(){return Err(bad("value reading duplicate view"));}
    }
    if groups.len()!=6{return Err(bad("value reading pair coverage"));}
    let mut selected=vec![];
    for pair in pairs{
        let versions=&groups[pair];
        if versions.len()!=if train{2}else{1}{return Err(bad("value reading ID coverage in returned192"));}
        let mut used=BTreeSet::new();
        for version in if train{vec![0,1]}else{vec![0,0]}{
            let (binding,slots)=versions[&version].iter().find(|(binding,slots)|
                !used.contains(*binding)&&slots.iter().all(Option::is_some))
                .ok_or_else(||bad("value reading distinct complete semantic bases unavailable"))?;
            used.insert(binding.clone());selected.extend(slots.iter().map(|v|v.unwrap()));
        }
    }
    if selected.len()!=48||selected.iter().collect::<BTreeSet<_>>().len()!=48{return Err(bad("value reading 48 unique"));}
    let(mut words,mut versions)=(BTreeMap::<String,usize>::new(),[0usize;2]);
    for &i in &selected{let(value,id)=event_label(&es[i].request,false)?;
        if es[i].answer!=format!("{value}입니다. [event:{id}]"){return Err(bad("value reading gold cross-check"));}
        *words.entry(value).or_default()+=1;versions[usize::from(ms[i].template.ends_with("/id1"))]+=1;
    }
    if words!=["왼쪽","오른쪽","직진","대기"].into_iter().map(|s|(s.into(),12)).collect()
        ||versions!=if train{[24,24]}else{[48,0]}{return Err(bad("value reading word/ID balance"));}
    Ok(selected)
}
fn value_lengths(es:&[Episode],tok:&ByteBpe,m:&checkpoint::Manifest)->Result<binary::Value>{
    let mut lengths=vec![];
    for e in es{
        let p=tok.prepare_with_framing(&e.request,m.framing()?,m.architecture.context as u32,&m.architecture.id()?)?;
        let target=tok.encode(e.answer.as_bytes())?;
        let sample=samples_with_framing(std::slice::from_ref(e),tok,256,m.framing()?)?;
        if sample.len()!=1||sample[0].tokens[..sample[0].response_start]!=p.token_ids
            ||sample[0].tokens[sample[0].response_start..sample[0].tokens.len()-1]!=target
            ||sample[0].tokens.last()!=Some(&EOS)
            ||tok.decode_bytes(&tok.encode(e.request.system.as_bytes())?)?!=e.request.system.as_bytes()
            ||tok.decode_bytes(&tok.encode(e.request.input.as_bytes())?)?!=e.request.input.as_bytes()
            ||e.request.evidence.items.iter().any(|r|tok.encode(r.original_excerpt.as_bytes()).and_then(|ids|tok.decode_bytes(&ids)).ok().as_deref()!=Some(r.original_excerpt.as_bytes()))
            ||tok.decode_bytes(&target)?!=e.answer.as_bytes()||e.request.limits.max_tokens!=32
            ||p.provided.len()!=e.request.evidence.items.len()||!p.excluded.is_empty()
            ||p.token_ids.len()+target.len()+1>256||target.len()+1>32{return Err(bad("value reading tokenizer/length/provided"));}
        lengths.push([p.token_ids.len(),target.len()+1,p.token_ids.len()+target.len()+1]);
    }
    let range=|i|[lengths.iter().map(|r|r[i]).min(),lengths.iter().map(|r|r[i]).max()];
    Ok(binary::record!({"count":es.len(),"prompt":range(0),"target_eos":range(1),"total":range(2)}))
}
fn value_prepare(evaluation:&Path,output:&Path,prior_executable:&Path,test_executable:&Path)->Result<()> {
    if output.file_name().is_none_or(|n|!n.to_string_lossy().starts_with("value-reading-20260925-"))
        ||output.try_exists()?||pending_path(output).exists(){return Err(bad("value reading one new named scope"));}
    let prior:FirstEvalOnly=read_confirmed(&evaluation.join("eval-only-plan.r3b"))?;
    if prior.contract!=FIRST_EVAL_ONLY||prior.root!=evaluation.canonicalize()?||prior.endpoints.len()!=2{
        return Err(bad("value reading completed evaluation plan identity"));
    }
    let original:Study=read_confirmed(&prior.original.join("plan.r3b"))?;
    if original.contract!=FIRST_CONTRACT||original.root!=prior.original
        ||prior.plan_hash!=file_hash(&original.root.join("plan.r3b"))?
        ||prior.terminal_hash!=file_hash(&original.root.join("segment-002-finished.r3b"))?{
        return Err(bad("value reading original source/terminal binding"));
    }
    let b:binary::Value=read_confirmed(&prior.root.join("eval-only-final-b.r3b"))?;
    if b["contract"]!=FIRST_EVAL_ONLY||b["policy"]!=digest(&prior)?||b["raw_integrity"]!="PASS"
        ||b["review_C"]!="PASS"||b["review_W"]!="PASS"{
        return Err(bad("value reading completed independent B absent"));
    }
    let endpoint=&prior.endpoints[0];
    if endpoint.local!=256||endpoint.physical!="1f5656d5c033a5cba3d271c3939aed66aaa14f4a00b11cc30f8081af5db28a90"
        ||file_hash(&endpoint.native)?!=endpoint.physical{return Err(bad("value reading C native identity"));}
    let(m,_)=checkpoint::metadata(&endpoint.native)?;
    let tok=scoring_tokenizer(&original)?;
    if m.model_content_digest.is_empty()||m.training.as_ref().is_none_or(|s|s.step!=18237)
        ||m.optimizer_protocol.as_ref().is_none_or(|o|o.local_step!=3901)
        ||m.framing()?!=neural::Framing::QuestionEvidence{return Err(bad("value reading C clock/framing"));}
    let all=fit_panels(&original)?;
    let mut originals=vec![];let mut lengths=vec![];let mut indices=[vec![],vec![]];let mut raw_paths=[PathBuf::new(),PathBuf::new()];
    let mut raw_hash=[String::new(),String::new()];
    for (split,name) in ["FULL-train","FULL-word"].iter().enumerate(){
        let panel=all.iter().find(|p|p.0==*name).ok_or_else(||bad("value reading original panel"))?;
        let raw=original.root.join("C").join(format!("eval-256-{name}.r3rows"));
        let(_,rows)=event_read_panel(&original,"C",256,&m.model_content_digest,panel,192)?;
        let es=&panel.1[..192];let ms=&panel.2[..192];
        if rows.len()!=192{return Err(bad("value reading O raw count"));}
        indices[split]=value_indices(es,ms,split==0)?;
        let subset:Panel=(format!("O-{}",if split==0{"train"}else{"dev"}),indices[split].iter().map(|&i|es[i].clone()).collect(),indices[split].iter().map(|&i|ms[i].clone()).collect());
        lengths.push(value_lengths(&subset.1,&tok,&m)?);
        raw_hash[split]=file_hash(&raw)?;raw_paths[split]=raw;originals.push(subset);
    }
    let mut panels=originals.clone();
    for (kind,uniform) in [("U",true),("S",false)]{
        for (split,o) in originals.iter().enumerate(){
            let es=o.1.iter().map(|e|value_variant(e,uniform)).collect::<Result<Vec<_>>>()?;
            let name=format!("{kind}-{}",if split==0{"train"}else{"dev"});
            lengths.push(value_lengths(&es,&tok,&m)?);
            panels.push((name,es,o.2.clone()));
        }
    }
    if panels.iter().map(|p|p.1.len()).sum::<usize>()!=288{return Err(bad("value reading 288 inputs"));}
    let order=original.full_fit.as_ref().ok_or_else(||bad("value reading train order"))?.train_order.as_slice();
    if order.len()<192{return Err(bad("value reading train order192"));}
    let corpus=event_inputs(&original,0)?;let mut exposure=vec![0usize;48];let mut semantic_other_exposure=vec![0usize;48];
    for (j,&at) in indices[0].iter().enumerate(){let corpus_index=order[at];
        exposure[j]=original.tape[..256].iter().flatten().filter(|&&i|i==corpus_index).count();
        let binding=&corpus.train[corpus_index].binding;
        semantic_other_exposure[j]=original.tape[..256].iter().flatten().filter(|&&i|i!=corpus_index&&corpus.train[i].binding==*binding).count();
    }
    let tokenizer=original.word_root.join("tokenizer.r3b");
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let mut expected=original.runtime.clone();expected.binary=runtime.binary.clone();
    if runtime!=expected{return Err(bad("value reading Metal F32 runtime drift"));}
    let executable=std::env::current_exe()?.canonicalize()?;
    let prior_executable=prior_executable.canonicalize()?;let test_executable=test_executable.canonicalize()?;
    let prior_executable_hash=file_hash(&prior_executable)?;
    if prior_executable_hash!="3f8cfa8237b602f94dfe3e5c3a774741554403e17ee7ae4d4437a1742afa9bb9"
        ||executable==prior_executable||executable==test_executable||prior_executable==test_executable{
        return Err(bad("value reading three distinct executable identities"));
    }
    let executable_bytes=std::fs::metadata(&executable)?.len()+std::fs::metadata(&prior_executable)?.len()
        +std::fs::metadata(&test_executable)?.len();
    if executable_bytes+4*1024*1024>128*1024*1024{return Err(bad("value reading executable/storage reserve exceeds128MiB"));}
    std::fs::create_dir(output)?;let root=output.canonicalize()?;
    let plan=ValueReading{contract:VALUE_READING.into(),root:root.clone(),evaluation:prior.root.clone(),evaluation_hash:file_hash(&prior.root.join("eval-only-plan.r3b"))?,predecessor_plan_hash:String::new(),revision_authority:String::new(),
        original:original.root.clone(),original_hash:file_hash(&original.root.join("plan.r3b"))?,native:endpoint.native.clone(),physical:endpoint.physical.clone(),
        model:m.model_content_digest,tokenizer:tokenizer.clone(),tokenizer_hash:file_hash(&tokenizer)?,source:sources()?,runtime,
        executable:executable.clone(),executable_hash:file_hash(&executable)?,prior_executable:prior_executable.clone(),prior_executable_hash,
        test_executable:test_executable.clone(),test_executable_hash:file_hash(&test_executable)?,panels,
        original_indices:indices,original_raw:raw_paths,original_raw_hash:raw_hash,exposure,semantic_other_exposure,lengths,
        generation_cap:216,token_cap:6912,active_cap:900.,segment_cap:300.,bytes_cap:128*1024*1024};
    publish_confirmed(&root.join("value-reading-plan.r3b"),&plan)?;
    if value_owned_bytes(&plan)?>plan.bytes_cap{return Err(bad("value reading immutable scope including three executables exceeds128MiB"));}
    println!("VALUE_READING_PREPARED policy={} O96 U96 S96 C={} lengths={:?} exposure={:?} semantic_other={:?} calls0 optimizer0 backward0 teacher0",digest(&plan)?,plan.physical,plan.lengths,plan.exposure,plan.semantic_other_exposure);
    Ok(())
}
fn value_revise(root:&Path)->Result<()> {
    let old_path=root.join("value-reading-plan.r3b");
    let mut p:ValueReading=read_confirmed(&old_path)?;
    if p.contract!=VALUE_READING||p.root!=root.canonicalize()?||!p.predecessor_plan_hash.is_empty()
        ||!p.revision_authority.is_empty()||p.bytes_cap!=128*1024*1024
        ||root.join("value-reading-plan-v2.r3b").exists()
        ||file_hash(&p.executable)?!=p.executable_hash
        ||file_hash(&p.prior_executable)?!=p.prior_executable_hash
        ||file_hash(&p.test_executable)?!=p.test_executable_hash{
        return Err(bad("value reading v1 revision identity"));
    }
    for entry in std::fs::read_dir(root)?{
        let name=entry?.file_name().to_string_lossy().into_owned();
        if name.ends_with(".r3rows")||name.contains("-segment-")||name.ends_with("-finished.r3b")
            ||name=="value-reading-report.r3b"||name=="review-a.r3b"{
            return Err(bad("value reading v1 already observed"));
        }
    }
    let old_executable_hash=p.executable_hash.clone();
    p.predecessor_plan_hash=file_hash(&old_path)?;
    p.revision_authority="user-approved-20260925-160MiB-immutable-scope".into();
    p.bytes_cap=160*1024*1024;
    p.source=sources()?;
    p.executable=std::env::current_exe()?.canonicalize()?;
    p.executable_hash=file_hash(&p.executable)?;
    let device=Backend::Metal0.open()?;
    let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let mut expected=p.runtime.clone();expected.binary=runtime.binary.clone();
    if expected!=runtime||runtime.binary!=p.executable_hash{return Err(bad("value reading v2 runtime drift"));}
    p.runtime=runtime;
    if p.executable==p.prior_executable||p.executable==p.test_executable||p.executable_hash==old_executable_hash{
        return Err(bad("value reading v2 executable identity"));
    }
    if value_owned_bytes(&p)?>p.bytes_cap{return Err(bad("value reading v2 immutable byte cap"));}
    publish_confirmed(&root.join("value-reading-plan-v2.r3b"),&p)?;
    println!("VALUE_READING_REVISED policy={} predecessor={} calls0",digest(&p)?,p.predecessor_plan_hash);
    Ok(())
}
fn value_load(root:&Path)->Result<ValueReading>{
    let p:ValueReading=read_confirmed(&root.join("value-reading-plan-v2.r3b"))?;
    if p.contract!=VALUE_READING||p.root!=root.canonicalize()?||p.panels.len()!=6
        ||p.predecessor_plan_hash!=file_hash(&root.join("value-reading-plan.r3b"))?
        ||p.revision_authority!="user-approved-20260925-160MiB-immutable-scope"
        ||p.panels.iter().map(|x|x.0.as_str()).collect::<Vec<_>>()!=["O-train","O-dev","U-train","U-dev","S-train","S-dev"]
        ||p.panels.iter().any(|x|x.1.len()!=48||x.2.len()!=48)
        ||p.generation_cap!=216||p.token_cap!=6912||p.active_cap!=900.||p.segment_cap!=300.||p.bytes_cap!=160*1024*1024
        ||p.source!=sources()?||p.physical!="1f5656d5c033a5cba3d271c3939aed66aaa14f4a00b11cc30f8081af5db28a90"
        ||file_hash(&p.evaluation.join("eval-only-plan.r3b"))?!=p.evaluation_hash
        ||file_hash(&p.original.join("plan.r3b"))?!=p.original_hash
        ||file_hash(&p.native)?!=p.physical||file_hash(&p.tokenizer)?!=p.tokenizer_hash
        ||file_hash(&p.executable)?!=p.executable_hash||file_hash(&p.prior_executable)?!=p.prior_executable_hash
        ||file_hash(&p.test_executable)?!=p.test_executable_hash
        ||(0..2).any(|i|file_hash(&p.original_raw[i]).ok()!=Some(p.original_raw_hash[i].clone())){
        return Err(bad("value reading plan/source/reference changed"));
    }
    Ok(p)
}
fn value_owned_bytes(p:&ValueReading)->Result<u64>{
    let old:ValueReading=read_confirmed(&p.root.join("value-reading-plan.r3b"))?;
    let mut paths=BTreeSet::new();
    for path in [&p.executable,&p.prior_executable,&p.test_executable,&old.executable]{
        if !path.starts_with(&p.root){paths.insert(path);}
    }
    Ok(owned_bytes(&p.root)?+paths.into_iter().map(|x|std::fs::metadata(x).map(|m|m.len())).collect::<std::io::Result<Vec<_>>>()?.into_iter().sum::<u64>())
}
fn value_binding(p:&ValueReading,panel:&Panel)->Result<binary::Value>{
    Ok(binary::record!({"contract":VALUE_READING,"policy":digest(p)?,"source":p.source,"native":p.physical,
        "model":p.model,"review_a":file_hash(&p.root.join("review-a.r3b"))?,"panel":panel.0,
        "cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,"planned":panel.1.len(),"call_protocol":1}))
}
fn value_aux_binding(p:&ValueReading,label:&str,cases:&[Episode])->Result<binary::Value>{
    Ok(binary::record!({"contract":VALUE_READING,"policy":digest(p)?,"source":p.source,"native":p.physical,
        "review_a":file_hash(&p.root.join("review-a.r3b"))?,"label":label,"cases":digest(&cases)?,"planned":cases.len(),"call_protocol":1}))
}
fn value_parity_cases(p:&ValueReading)->Result<(Vec<Episode>,Vec<binary::Value>)>{
    let mut positions=vec![];let mut words=BTreeMap::<String,usize>::new();
    for (panel_index,panel) in p.panels[..2].iter().enumerate(){for (i,e) in panel.1.iter().enumerate(){
        let word=event_label(&e.request,false)?.0;let n=words.entry(word).or_default();
        if *n<2{positions.push((panel_index,i));*n+=1;}
    }}
    if positions.len()!=8||words.values().any(|&n|n!=2){return Err(bad("value reading parity four words"));}
    let originals=[value_rows(p,0,true)?,value_rows(p,1,true)?];
    Ok((positions.iter().map(|&(j,i)|p.panels[j].1[i].clone()).collect(),
        positions.iter().map(|&(j,i)|originals[j][i].clone()).collect()))
}
fn value_parity_rows(p:&ValueReading)->Result<()> {
    let(cases,expected)=value_parity_cases(p)?;
    let binding=value_aux_binding(p,"parity",&cases)?;
    let raw=binary::read_value_records(&p.root.join("value-parity.r3rows"))?;
    if raw.len()!=9||raw[0]!=binding{return Err(bad("value reading parity RETURNED count/binding"));}
    for(i,((e,a),b))in cases.iter().zip(&raw[1..]).zip(expected.iter()).enumerate(){
        call_attempt(&p.root,"value-parity","generation",&binding,e,i,Some(a))?;
        for key in ["raw_tokens","actual","finish_reason","error","generation_completed"]{
            if a[key]!=b[key]{return Err(bad("value reading O parity mismatch"));}
        }
    }
    Ok(())
}
fn value_rows(p:&ValueReading,index:usize,strict_attempts:bool)->Result<Vec<binary::Value>>{
    let panel=&p.panels[index];let tok=ByteBpe::load(&p.tokenizer)?;let m=checkpoint::metadata(&p.native)?.0;
    if tok.semantic_id()!=read_confirmed::<Study>(&p.original.join("plan.r3b"))?.tokenizer{return Err(bad("value reading tokenizer semantic changed"));}
    let rows=if index<2{
        let raw=binary::read_value_records(&p.original_raw[index])?;
        if raw.len()!=193||file_hash(&p.original_raw[index])?!=p.original_raw_hash[index]{return Err(bad("value reading O raw count/hash"));}
        p.original_indices[index].iter().map(|&i|raw.get(i+1).cloned().ok_or_else(||bad("value reading O index"))).collect::<Result<Vec<_>>>()?
    }else{
        let label=format!("value-{}",panel.0);let path=p.root.join(format!("{label}.r3rows"));
        let raw=binary::read_value_records(&path)?;
        let binding=value_binding(p,panel)?;
        if raw.len()!=panel.1.len()+1||raw[0]!=binding{return Err(bad("value reading RETURNED count/binding"));}
        if strict_attempts{for(i,(e,r))in panel.1.iter().zip(&raw[1..]).enumerate(){call_attempt(&p.root,&label,"generation",&binding,e,i,Some(r))?;}}
        raw[1..].to_vec()
    };
    for(e,r)in panel.1.iter().zip(&rows){verify_output_result(r,&tok)?;
        let prompt=tok.prepare_with_framing(&e.request,m.framing()?,m.architecture.context as u32,&m.architecture.id()?)?;
        if r["id"]!=e.id||r["expected"]!=e.answer||r["question"]!=e.request.input
            ||r["generated_evidence"]!=binary::record!(e.request.evidence)
            ||r["request_digest"]!=digest(&e.request)?||r["native_prompt_digest"]!=prompt.token_digest
            ||r["prompt_digest"]!=digest(&prompt.token_ids)?||r["provided"]!=binary::record!(prompt.provided)
            ||r["excluded"]!=binary::record!(prompt.excluded){return Err(bad("value reading raw case identity"));}
    }
    Ok(rows)
}
fn value_answer(text:&str)->Option<(&str,i64)>{
    let(value,suffix)=text.split_once("입니다. [event:")?;
    let digits=suffix.strip_suffix(']')?;
    if value.is_empty()||digits.is_empty()||!digits.bytes().all(|x|x.is_ascii_digit()){return None;}
    let id=digits.parse::<i64>().ok().filter(|&x|x>0)?;
    (text==format!("{value}입니다. [event:{id}]")&&citations(text).ok()==Some(vec![id])).then_some((value,id))
}
fn value_score(p:&ValueReading,index:usize,rows:&[binary::Value])->Result<binary::Value>{
    let panel=&p.panels[index];let tok=ByteBpe::load(&p.tokenizer)?;
    let base=score(rows,&panel.1,&panel.2)?;
    let(mut values,mut support,mut malformed,mut outside,mut other_word,mut foil,mut exact,mut wholes)=(0,0,0,0,0,0,Vec::new(),Vec::new());
    let mut words=BTreeMap::<String,[usize;3]>::new();let mut pairs=BTreeMap::<String,[usize;3]>::new();let mut semantic=BTreeMap::<String,[usize;3]>::new();
    for(i,(e,r))in panel.1.iter().zip(rows).enumerate(){
        verify_output_result(r,&tok)?;
        let(gold,gold_id)=value_answer(&e.answer).ok_or_else(||bad("value reading gold format"))?;
        let text=r["actual"].as_str();let parsed=text.and_then(value_answer);
        let ids=text.map(identifiable::binding::citation::individually_valid_ids).unwrap_or_default();
        let whole=text.and_then(|v|v.split_once("입니다. ")).is_some_and(|(v,_)|v==gold);
        let supported=ids.len()==1&&ids.contains(&gold_id);
        let full=r["generation_completed"]==true&&r["finish_reason"]=="stop"&&r["error"].is_null()&&text==Some(e.answer.as_str());
        if r["exact_match"]!=full{return Err(bad("value reading exact receipt"));}
        values+=usize::from(whole);support+=usize::from(supported);malformed+=usize::from(parsed.is_none());
        outside+=usize::from(ids.iter().any(|id|!e.request.evidence.items.iter().any(|x|x.event_id==*id)));
        let original=&p.panels[index%2].1[i];
        let selected=value_selected(&original.request)?;
        let(_,_,other)=parsed_record(&original.request.evidence.items[1-selected])?;
        foil+=usize::from(parsed.is_some_and(|(v,_)|v==other));
        other_word+=usize::from(parsed.is_some_and(|(v,_)|v!=gold&&v!=other));
        exact.push(full);wholes.push(whole);
        for (table,key) in [(&mut words,gold.to_owned()),(&mut pairs,panel.2[i].template.clone()),(&mut semantic,panel.2[i].base.clone())]{
            let row=table.entry(key).or_default();row[0]+=1;row[1]+=usize::from(whole);row[2]+=usize::from(full);
        }
    }
    if exact.iter().filter(|&&v|v).count()!=base.exact{return Err(bad("value reading independent FULL"));}
    Ok(binary::record!({"panel":panel.0,"total":rows.len(),"full":base.exact,"whole_value":values,
        "target_support":support,"eos":base.eos,"errors":base.errors,"malformed":malformed,"outside":outside,
        "original_foil_word":foil,"other_word":other_word,"exact":exact,"whole":wholes,
        "by_word":words,"by_pair":pairs,"by_semantic_base":semantic}))
}
fn value_history(p:&ValueReading)->Result<(f64,usize,f64,usize,[usize;6])>{
    let mut elapsed=0.;let mut calls=0usize;let mut main_elapsed=0.;let mut main_calls=0usize;let mut next=[0usize;6];
    for (lane_index,lane) in ["parity","u-train","u-dev","s-train","s-dev","review"].iter().enumerate(){
        let mut last=None;let mut gap=false;
        for i in 0..16{
            let start=p.root.join(format!("{lane}-segment-{i:03}-started.r3b"));
            let finish=p.root.join(format!("{lane}-segment-{i:03}-finished.r3b"));
            if !start.exists(){gap=true;if finish.exists(){return Err(bad("value reading orphan segment finish"));}continue;}
            if gap{return Err(bad("value reading segment gap"));}
            let end:binary::Value=read_confirmed(&finish)?;
            if end["start"]!=file_hash(&start)?||end["success"]!=true&&end["resume"]!=true
                ||end["success"]==true&&end["resume"]==true{
                return Err(bad("value reading segment identity/status"));
            }
            let conditions=end["control"]["observed_conditions"].as_array().ok_or_else(||bad("value reading conditions UNKNOWN"))?;
            if end["success"]==true&& !conditions.is_empty()
                ||end["resume"]==true&&conditions.as_slice()!=[binary::Value::String("TIME_BUDGET".into())]{
                return Err(bad("value reading segment UNKNOWN/cancel"));
            }
            let segment_elapsed=end["control"]["elapsed_seconds"].as_f64().filter(|v|v.is_finite()&&*v>=0.).ok_or_else(||bad("value reading elapsed UNKNOWN"))?;
            let segment_calls=end["control"]["generation_calls"].as_u64().ok_or_else(||bad("value reading calls UNKNOWN"))?as usize;
            elapsed+=segment_elapsed;calls+=segment_calls;
            if lane_index<5{main_elapsed+=segment_elapsed;main_calls+=segment_calls;}
            if end["control"]["teacher_calls"]!=0{return Err(bad("value reading teacher forbidden"));}
            last=Some(end);next[lane_index]=i+1;
        }
        let marker=p.root.join(format!("{lane}-finished.r3b"));
        if marker.exists(){
            let end=last.ok_or_else(||bad("value reading lane marker without segment"))?;
            if end["success"]!=true||read_confirmed::<binary::Value>(&marker)?!=end{
                return Err(bad("value reading lane marker not final verified segment"));
            }
        }else if last.as_ref().is_some_and(|x|x["success"]==true){
            return Err(bad("value reading successful segment missing lane marker"));
        }
    }
    if !elapsed.is_finite()||elapsed>p.active_cap||calls>p.generation_cap{return Err(bad("value reading aggregate usage cap"));}
    Ok((elapsed,calls,main_elapsed,main_calls,next))
}
fn value_report(p:&ValueReading)->Result<binary::Value>{
    value_parity_rows(p)?;
    let mut scores=BTreeMap::new();
    for(i,panel)in p.panels.iter().enumerate(){let rows=value_rows(p,i,true)?;scores.insert(panel.0.clone(),value_score(p,i,&rows)?);}
    let(_,_,active,calls,_)=value_history(p)?;
    for label in ["parity","u-train","u-dev","s-train","s-dev"]{
        let done:binary::Value=read_confirmed(&p.root.join(format!("{label}-finished.r3b")))?;
        if done["success"]!=true{return Err(bad("value reading unfinished lane"));}
    }
    if calls!=200{return Err(bad("value reading 200 RETURNED generation denominator"));}
    let mut paired=BTreeMap::new();let mut paired_whole=BTreeMap::new();
    for split in ["train","dev"]{for kind in ["U","S"]{
        let a=&scores[&format!("O-{split}")];let b=&scores[&format!("{kind}-{split}")];
        let aa:Vec<bool>=binary::from_value(a["exact"].clone())?;let bb:Vec<bool>=binary::from_value(b["exact"].clone())?;
        let mut counts=[0usize;4];for(x,y)in aa.iter().zip(bb.iter()){
            counts[match(*x,*y){(true,true)=>0,(false,true)=>1,(true,false)=>2,_=>3}]+=1;
        }
        paired.insert(format!("O-{kind}-{split}"),counts);
        let aa:Vec<bool>=binary::from_value(a["whole"].clone())?;let bb:Vec<bool>=binary::from_value(b["whole"].clone())?;
        let mut counts=[0usize;4];for(x,y)in aa.iter().zip(bb.iter()){
            counts[match(*x,*y){(true,true)=>0,(false,true)=>1,(true,false)=>2,_=>3}]+=1;
        }
        paired_whole.insert(format!("O-{kind}-{split}"),counts);
    }}
    Ok(binary::record!({"contract":VALUE_READING,"policy":digest(p)?,"scores":scores,"paired_full":paired,"paired_whole_value":paired_whole,
        "quality_approved":false,"diagnostic_only":true,"new_optimizer":0,"new_backward":0,"new_teacher":0,
        "main_active_seconds":active,"main_generation_calls":calls}))
}
fn value_review_cases(p:&ValueReading)->Result<(Vec<Episode>,Vec<binary::Value>)>{
    let raw=(0..6).map(|i|value_rows(p,i,true)).collect::<Result<Vec<_>>>()?;
    let mut positions=vec![];let mut words=BTreeSet::new();
    for(i,e)in p.panels[0].1.iter().enumerate(){let word=event_label(&e.request,false)?.0;
        if words.insert(word){for j in [0,2,4]{positions.push((j,i));}}
    }
    if positions.len()!=12||words.len()!=4{return Err(bad("value reading B fixed four-word O/U/S"));}
    let mut prior_failure=BTreeSet::new();
    for j in [2,4]{
        let mut selected=None;
        for i in 0..48{let mate=i^1;
            if mate>=48||p.panels[j].2[i].base!=p.panels[j].2[mate].base
                ||positions.contains(&(j,i))||positions.contains(&(j,mate))
                ||prior_failure.contains(&p.panels[j].2[i].base){continue;}
            if raw[j][i]["exact_match"]==false{selected=Some((i,mate));break;}
        }
        if selected.is_none(){for i in (0..48).step_by(2){if !positions.contains(&(j,i))&&!positions.contains(&(j,i+1)){
            selected=Some((i,i+1));break;
        }}}
        let(i,mate)=selected.ok_or_else(||bad("value reading B distinct failure/mate unavailable"))?;
        prior_failure.insert(p.panels[j].2[i].base.clone());positions.extend([(j,i),(j,mate)]);
    }
    if positions.len()!=16||positions.iter().collect::<BTreeSet<_>>().len()!=16{return Err(bad("value reading B16 unique"));}
    Ok((positions.iter().map(|&(j,i)|p.panels[j].1[i].clone()).collect(),
        positions.iter().map(|&(j,i)|raw[j][i].clone()).collect()))
}
fn value_run(root:&Path,phase:&str)->Result<()> {
    let p=value_load(root)?;
    if phase=="report"{let report=value_report(&p)?;let path=root.join("value-reading-report.r3b");
        if path.exists(){if read_confirmed::<binary::Value>(&path)?!=report{return Err(bad("value reading report changed"));}}
        else{publish_confirmed(&path,&report)?;}
        println!("VALUE_READING_REPORT policy={} O/U/S=48/48 calls0",digest(&p)?);return Ok(());
    }
    let a:binary::Value=read_confirmed(&root.join("review-a.r3b"))?;
    if a["contract"]!=VALUE_READING||a["policy"]!=digest(&p)?||a["source"]!=p.source||a["verdict"]!="PASS"{
        return Err(bad("value reading independent A absent/mismatch"));
    }
    let device=Backend::Metal0.open()?;p.runtime.verify(&device)?;
    let loaded=checkpoint::load(&p.native,device,false)?;
    if loaded.model.weights_content_id()?!=p.model||loaded.tokenizer.semantic_id()!=read_confirmed::<Study>(&p.original.join("plan.r3b"))?.tokenizer{
        return Err(bad("value reading loaded native/tokenizer mismatch"));
    }
    let (cases,expected,panel_index): (Vec<Episode>,Option<Vec<binary::Value>>,Option<usize>)=if phase=="parity"{
        let(cases,expected)=value_parity_cases(&p)?;(cases,Some(expected),None)
    }else if phase=="review"{
        if read_confirmed::<binary::Value>(&root.join("value-reading-report.r3b"))?!=value_report(&p)?{
            return Err(bad("value reading B requires pure report"));
        }
        let(cases,expected)=value_review_cases(&p)?;(cases,Some(expected),None)
    }else{
        if !p.root.join("parity-finished.r3b").exists(){return Err(bad("value reading parity first"));}
        let names=["u-train","u-dev","s-train","s-dev"];
        let at=names.iter().position(|&n|n==phase).ok_or_else(||bad("value reading phase"))?;
        for earlier in &names[..at]{if !p.root.join(format!("{earlier}-finished.r3b")).exists(){return Err(bad("value reading ordered panels"));}}
        (p.panels[at+2].1.clone(),None,Some(at+2))
    };
    let(elapsed,calls,_,_,next)=value_history(&p)?;
    let segment=["parity","u-train","u-dev","s-train","s-dev","review"].iter().position(|&x|x==phase).map(|i|next[i]).ok_or_else(||bad("value reading phase"))?;
    if calls>=p.generation_cap||elapsed>=p.active_cap||value_owned_bytes(&p)?>p.bytes_cap{return Err(bad("value reading budget/storage exhausted"));}
    let label=phase.to_owned();
    if p.root.join(format!("{label}-finished.r3b")).exists(){return Err(bad("value reading lane already complete"));}
    let binding=if let Some(i)=panel_index{value_binding(&p,&p.panels[i])?}else{value_aux_binding(&p,&label,&cases)?};
    let started=p.root.join(format!("{label}-segment-{segment:03}-started.r3b"));
    publish_confirmed(&started,&binding)?;
    let cancel=std::sync::Arc::new(AtomicBool::new(false));let signal=cancel.clone();
    ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut control=RunControl::new(cancel,std::time::Duration::from_secs_f64((p.active_cap-elapsed).min(p.segment_cap)),12*1024*1024)?;
    control.set_call_limits(p.generation_cap-calls,0);
    let raw_label=panel_index.map_or_else(||format!("value-{label}"),|i|format!("value-{}",p.panels[i].0));
    let result=(||->Result<()>{let rows=generated_until(&loaded,&p.root,&raw_label,&cases,&binding,&mut control,cases.len())?;
        if let Some(expected)=expected{for(a,b)in rows.iter().zip(expected){for key in ["raw_tokens","actual","finish_reason","error","generation_completed"]{
            if a[key]!=b[key]{return Err(bad("value reading O parity mismatch"));}
        }}}
        control.seal_completed_no_call()?;if value_owned_bytes(&p)?>p.bytes_cap{return Err(bad("value reading immutable byte cap"));}Ok(())})();
    if let Err(e)=&result{control.classify_error(e);}
    let clean=control.receipt()["observed_conditions"]==binary::record!(["TIME_BUDGET"]);
    let end=binary::record!({"start":file_hash(&started)?,"success":result.is_ok(),"resume":result.is_err()&&clean,
        "control":control.receipt(),"error":result.as_ref().err().map(ToString::to_string)});
    publish_confirmed(&p.root.join(format!("{label}-segment-{segment:03}-finished.r3b")),&end)?;
    if result.is_ok(){publish_confirmed(&p.root.join(format!("{label}-finished.r3b")),&end)?;}
    result
}
fn review(s:&Study,arm:usize)->Result<()> {
    if s.first_decision.is_some(){return first_review(s,arm);}
    if s.full_fit.is_some(){return fit_review(s);}
    if s.event.is_some(){return event_review(s,arm);}
    let h=history(s)?;let end=h.last().ok_or_else(||bad("not run"))?;
    if end.resume||!end.success||!end.arms.iter().all(|p|p.fit){return Err(bad("B requires normally complete endpoint"));}
    report(s)?;let p=&end.arms[arm];let mut cases=vec![];let mut expected=vec![];let mut failures=vec![];
    for((name,es,_),count)in panels(s,p.local,false)?.into_iter().zip([8,6,6,6,6]){
        let raw=binary::read_value_records(&s.root.join(s.arms()[arm]).join(format!("eval-{}-{name}.r3rows",p.local)))?;
        for(i,e)in es.into_iter().enumerate(){if i<count{cases.push(e);expected.push(raw[i+1].clone());}else if failures.len()<32&&(raw[i+1]["actual"]!=e.answer||raw[i+1]["finish_reason"]!="stop"){failures.push((e,raw[i+1].clone()));}}
    }
    for(e,r)in failures{cases.push(e);expected.push(r);}
    let label=format!("review-{}",s.arms()[arm]);
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
        Action::FirstPrepare{original,output}=>first_prepare(&original,&output),
        Action::FirstEvalPrepare{original,output}=>first_eval_prepare(&original,&output),
        Action::FirstEvalRevise{root}=>first_eval_revise(&root),
        Action::FirstEvalPostMainRevise{root}=>first_eval_post_main_revise(&root),
        Action::FirstEvalPostMainAdmit{root,review}=>first_eval_post_main_admit(&root,&review),
        Action::FirstEvalAdmit{root,review}=>first_eval_admit(&root,&review),
        Action::FirstEval{root,phase}=>first_eval_run(&root,&phase),
        Action::ValueReadingPrepare{evaluation,output,prior_executable,test_executable}=>value_prepare(&evaluation,&output,&prior_executable,&test_executable),
        Action::ValueReadingRevise{root}=>value_revise(&root),
        Action::ValueReading{root,phase}=>value_run(&root,&phase),
        Action::FullFitPosthocPrepare{original,output}=>fit_posthoc_prepare(&original,&output),
        Action::FullFitPosthocAdmit{root,review}=>fit_posthoc_admit(&root,&review),
        Action::FullFitPosthoc{root,phase}=>fit_posthoc_run(&root,&phase),
        Action::FullFitPrepare{prior_study,review_b,output}=>fit_prepare(&prior_study,&review_b,&output),
        Action::Diagnose{command}=>diagnosis::run(command),
        Action::EventPrepare{diagnosis,audit,output}=>event_prepare(&diagnosis,&audit,&output),
        Action::Prepare{parent,word_root,output}=>prepare(&parent,&word_root,&output),
        Action::Admit{root,review}=>admit(&root,&review),
        Action::Baseline{root}=>{let s=load_study(&root,true)?;if s.first_decision.is_some(){first_baseline(&s)}else{baseline(&s)}},
        Action::Train{root}=>{let s=load_study(&root,true)?;if s.first_decision.is_some(){first_train(&s)}else{train(&s)}},
        Action::Report{root}=>report(&load_study(&root,false)?),
        Action::Review{root,arm}=>{let s=load_study(&root,true)?;let index=s.arms().iter().position(|&a|a==arm).ok_or_else(||bad("wrong study arm"))?;review(&s,index)},
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
    fn value_test_plan(root:&Path)->Result<ValueReading>{
        // The archived plan binds the original product source. A test-only edit
        // changes sources(), so inspect the canonical plan and its references.
        let p:ValueReading=read_confirmed(&root.join("value-reading-plan-v2.r3b"))?;
        if p.contract!=VALUE_READING||p.root!=root.canonicalize()?||p.panels.len()!=6
            ||p.panels.iter().map(|x|x.0.as_str()).collect::<Vec<_>>()!=["O-train","O-dev","U-train","U-dev","S-train","S-dev"]
            ||p.panels.iter().any(|x|x.1.len()!=48||x.2.len()!=48)
            ||p.predecessor_plan_hash!=file_hash(&root.join("value-reading-plan.r3b"))?
            ||p.revision_authority!="user-approved-20260925-160MiB-immutable-scope"
            ||p.generation_cap!=216||p.token_cap!=6912||p.active_cap!=900.||p.segment_cap!=300.||p.bytes_cap!=160*1024*1024
            ||file_hash(&p.evaluation.join("eval-only-plan.r3b"))?!=p.evaluation_hash
            ||file_hash(&p.original.join("plan.r3b"))?!=p.original_hash
            ||file_hash(&p.native)?!=p.physical||file_hash(&p.tokenizer)?!=p.tokenizer_hash
            ||file_hash(&p.executable)?!=p.executable_hash||file_hash(&p.prior_executable)?!=p.prior_executable_hash
            ||file_hash(&p.test_executable)?!=p.test_executable_hash
            ||(0..2).any(|i|file_hash(&p.original_raw[i]).ok()!=Some(p.original_raw_hash[i].clone())){
            return Err(bad("value reading test plan/reference changed"));
        }
        Ok(p)
    }
    fn value_fixture_files(root:&Path)->Result<BTreeMap<String,String>>{
        std::fs::read_dir(root)?.map(|entry|{
            let path=entry?.path();
            Ok((path.file_name().unwrap().to_string_lossy().into_owned(),file_hash(&path)?))
        }).collect()
    }
    #[test]
    #[ignore="prepared 288-case manifest, synthetic six-panel report and B16 preflight; zero model calls"]
    fn value_reading_full_report_preflight()->Result<()> {
        let original=PathBuf::from(std::env::var("R3_VALUE_READING_ROOT").map_err(|_|bad("explicit prepared value reading root"))?);
        let mut p=value_test_plan(&original)?;
        let root=std::env::temp_dir().join(format!("value-reading-20260925-preflight-{}",std::process::id()));
        std::fs::create_dir(&root)?;p.root=root.canonicalize()?;
        publish_confirmed(&p.root.join("review-a.r3b"),&binary::record!({"contract":VALUE_READING,"policy":digest(&p)?,"source":p.source,"verdict":"PASS","fixture":true}))?;
        let m=checkpoint::metadata(&p.native)?.0;let tok=ByteBpe::load(&p.tokenizer)?;
        let originals=[value_rows(&p,0,false)?,value_rows(&p,1,false)?];
        for j in 2..6{let panel=&p.panels[j];let binding=value_binding(&p,panel)?;let label=format!("value-{}",panel.0);
            let mut f=std::fs::File::create(p.root.join(format!("{label}.r3rows")))?;append_row(&mut f,&binding)?;
            for(i,e)in panel.1.iter().enumerate(){let mut r=originals[j%2][i].clone();
                let prompt=tok.prepare_with_framing(&e.request,m.framing()?,m.architecture.context as u32,&m.architecture.id()?)?;
                r["id"]=binary::record!(e.id);r["request_digest"]=binary::record!(digest(&e.request)?);
                r["native_prompt_digest"]=binary::record!(prompt.token_digest);r["prompt_digest"]=binary::record!(digest(&prompt.token_ids)?);
                r["provided"]=binary::record!(prompt.provided);r["excluded"]=binary::record!(prompt.excluded);
                r["generated_evidence"]=binary::record!(e.request.evidence);
                let attempt=prepare_call(&p.root,&label,"generation",&binding,e,i)?;
                r["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy().to_string());append_row(&mut f,&r)?;
                publish_confirmed(&attempt.with_file_name(attempt.file_name().unwrap().to_string_lossy().replace("-prepared","-resolved")),
                    &binary::record!({"prepared":file_hash(&attempt)?,"state":"RETURNED","row":digest(&r)?,"calls":1,"resumable":false,"control":{"generation_calls":1}}))?;
            }
        }
        let(cases,expected)=value_parity_cases(&p)?;let binding=value_aux_binding(&p,"parity",&cases)?;
        let mut f=std::fs::File::create(p.root.join("value-parity.r3rows"))?;append_row(&mut f,&binding)?;
        for(i,(e,original))in cases.iter().zip(expected).enumerate(){let mut r=original;
            let attempt=prepare_call(&p.root,"value-parity","generation",&binding,e,i)?;
            r["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy().to_string());append_row(&mut f,&r)?;
            publish_confirmed(&attempt.with_file_name(attempt.file_name().unwrap().to_string_lossy().replace("-prepared","-resolved")),
                &binary::record!({"prepared":file_hash(&attempt)?,"state":"RETURNED","row":digest(&r)?,"calls":1,"resumable":false,"control":{"generation_calls":1}}))?;
        }
        for(lane,calls)in [("parity",8),("u-train",48),("u-dev",48),("s-train",48),("s-dev",48)]{
            let start=p.root.join(format!("{lane}-segment-000-started.r3b"));publish_confirmed(&start,&binding)?;
            let end=binary::record!({"start":file_hash(&start)?,"success":true,"resume":false,
                "control":{"observed_conditions":[],"elapsed_seconds":0.0,"generation_calls":calls,"teacher_calls":0}});
            publish_confirmed(&p.root.join(format!("{lane}-segment-000-finished.r3b")),&end)?;
            publish_confirmed(&p.root.join(format!("{lane}-finished.r3b")),&end)?;
        }
        let report=value_report(&p)?;assert_eq!(report["scores"].as_object().unwrap().len(),6);
        assert_eq!(report["scores"]["S-dev"]["total"],48);
        let(cases,expected)=value_review_cases(&p)?;assert_eq!(cases.len(),16);assert_eq!(expected.len(),16);
        let before=value_fixture_files(&p.root)?;assert_eq!(report,value_report(&p)?);
        assert_eq!(before,value_fixture_files(&p.root)?);
        let mut wrong_model=p.clone();wrong_model.model="wrong-model".into();
        assert!(value_report(&wrong_model).unwrap_err().to_string().contains("RETURNED count/binding"));
        assert_eq!(report,value_report(&p)?);
        let resolution=p.root.join("value-U-train-generation-0000-000-resolved.r3b");
        let resolution_bytes=std::fs::read(&resolution)?;
        std::fs::remove_file(&resolution)?;
        assert!(value_report(&p).unwrap_err().to_string().contains("usage UNKNOWN"));
        std::fs::write(&resolution,&resolution_bytes)?;
        assert_eq!(report,value_report(&p)?);
        let segment=p.root.join("s-dev-segment-000-finished.r3b");
        let marker=p.root.join("s-dev-finished.r3b");
        let segment_bytes=std::fs::read(&segment)?;let marker_bytes=std::fs::read(&marker)?;
        let mut unknown:binary::Value=read_confirmed(&segment)?;
        unknown["control"]["observed_conditions"]=binary::record!(["UNKNOWN"]);
        std::fs::remove_file(&segment)?;std::fs::remove_file(&marker)?;
        publish_confirmed(&segment,&unknown)?;publish_confirmed(&marker,&unknown)?;
        assert!(value_report(&p).unwrap_err().to_string().contains("segment UNKNOWN/cancel"));
        std::fs::write(&segment,&segment_bytes)?;std::fs::write(&marker,&marker_bytes)?;
        assert_eq!(report,value_report(&p)?);
        assert_eq!(before,value_fixture_files(&p.root)?);
        std::fs::remove_dir_all(&p.root)?;Ok(())
    }
    #[test]
    #[ignore="actual C two-row writer/reader/scorer/finalizer fixture; zero model calls"]
    fn value_reading_boundary_fixture()->Result<()> {
        let eval=PathBuf::from(std::env::var("R3_FIRST_EVAL_ROOT").map_err(|_|bad("explicit completed eval root"))?);
        let d:FirstEvalOnly=read_confirmed(&eval.join("eval-only-plan.r3b"))?;
        let s:Study=read_confirmed(&d.original.join("plan.r3b"))?;
        let m=checkpoint::metadata(&d.endpoints[0].native)?.0;let tok=scoring_tokenizer(&s)?;
        for total in [255usize,256,257]{let prompt=tok.encode(&vec![b'~';total-2])?;
            assert_eq!(prompt.len(),total-2);assert_eq!(event_length(prompt.len(),1,32).is_ok(),total<=256);
        }
        let all=fit_panels(&s)?;let mut originals=vec![];let mut paths=[PathBuf::new(),PathBuf::new()];let mut hashes=[String::new(),String::new()];
        let mut indices=[vec![],vec![]];
        for (j,name) in ["FULL-train","FULL-word"].iter().enumerate(){
            let panel=all.iter().find(|p|p.0==*name).ok_or_else(||bad("fixture original panel"))?;
            let path=s.root.join("C").join(format!("eval-256-{name}.r3rows"));
            let raw=binary::read_value_records(&path)?;
            let first=(0..192).step_by(4).find(|&i|raw[i+1]["exact_match"]==false||raw[i+2]["exact_match"]==false)
                .ok_or_else(||bad("fixture wrong RETURNED group"))?;
            indices[j]=vec![first,first+1];paths[j]=path.clone();hashes[j]=file_hash(&path)?;
            originals.push((format!("O-{}",if j==0{"train"}else{"dev"}),
                indices[j].iter().map(|&i|panel.1[i].clone()).collect::<Vec<_>>(),
                indices[j].iter().map(|&i|panel.2[i].clone()).collect::<Vec<_>>()));
        }
        let mut panels=originals.clone();for (kind,uniform) in [("U",true),("S",false)]{for(j,o)in originals.iter().enumerate(){
            panels.push((format!("{kind}-{}",if j==0{"train"}else{"dev"}),o.1.iter().map(|e|value_variant(e,uniform)).collect::<Result<Vec<_>>>()?,o.2.clone()));
        }}
        let e=&originals[0].1[0];let selected=value_selected(&e.request)?;
        let mut malformed=e.clone();malformed.request.evidence.items[1-selected].original_excerpt="invalid record".into();
        assert!(value_variant(&malformed,true).is_err());
        let mut ambiguous=e.clone();ambiguous.request.evidence.items[1-selected].original_excerpt=e.request.evidence.items[selected].original_excerpt.clone();
        ambiguous.request.evidence.items[1-selected].version_status="current".into();
        assert!(value_variant(&ambiguous,true).is_err());
        let root=std::env::temp_dir().join(format!("value-reading-20260925-fixture-{}",std::process::id()));
        std::fs::create_dir(&root)?;let root=root.canonicalize()?;
        let p=ValueReading{contract:VALUE_READING.into(),root:root.clone(),evaluation:eval.clone(),evaluation_hash:file_hash(&eval.join("eval-only-plan.r3b"))?,predecessor_plan_hash:String::new(),revision_authority:String::new(),
            original:s.root.clone(),original_hash:file_hash(&s.root.join("plan.r3b"))?,native:d.endpoints[0].native.clone(),physical:d.endpoints[0].physical.clone(),
            model:m.model_content_digest.clone(),tokenizer:s.word_root.join("tokenizer.r3b"),tokenizer_hash:file_hash(&s.word_root.join("tokenizer.r3b"))?,
            source:sources()?,runtime:s.runtime.clone(),executable:std::env::current_exe()?,executable_hash:String::new(),
            prior_executable:std::env::current_exe()?,prior_executable_hash:String::new(),test_executable:std::env::current_exe()?,test_executable_hash:String::new(),
            panels,original_indices:indices,original_raw:paths,original_raw_hash:hashes,
            exposure:vec![0;2],semantic_other_exposure:vec![0;2],lengths:vec![],generation_cap:216,token_cap:6912,active_cap:900.,segment_cap:300.,bytes_cap:128*1024*1024};
        publish_confirmed(&root.join("review-a.r3b"),&binary::record!({"contract":VALUE_READING,"policy":digest(&p)?,"source":p.source,"verdict":"PASS","fixture":true}))?;
        let originals=[value_rows(&p,0,false)?,value_rows(&p,1,false)?];
        for j in 2..6{let panel=&p.panels[j];let b=value_binding(&p,panel)?;let label=format!("value-{}",panel.0);
            let mut f=std::fs::File::create(root.join(format!("{label}.r3rows")))?;append_row(&mut f,&b)?;
            for(i,e)in panel.1.iter().enumerate(){let mut r=originals[j%2][i].clone();
                let prompt=tok.prepare_with_framing(&e.request,m.framing()?,m.architecture.context as u32,&m.architecture.id()?)?;
                if j==2&&i==0{
                    let bytes=[0xffu8];let ids=tok.encode(&bytes)?;let mut raw=ids.clone();raw.push(EOS);
                    r["raw_tokens"]=binary::record!(raw);r["generation"]["tokens"]=binary::record!(ids);
                    r["generation"]["generated"]=binary::record!(raw.len());r["generation"]["finish"]=binary::record!("stop");
                    r["finish_reason"]=binary::record!("stop");r["generation_completed"]=binary::record!(true);
                    r["actual"]=binary::Value::Null;r["error"]=binary::record!("model: invalid utf-8 diagnostic fixture");
                    r["decode_error"]=r["error"].clone();r["error_class"]=binary::record!("strict_utf8");
                    r["generation_error"]=binary::Value::Null;r["command_stop"]=binary::Value::Null;
                    r["exact_match"]=binary::record!(false);
                }
                r["id"]=binary::record!(e.id);r["request_digest"]=binary::record!(digest(&e.request)?);
                r["native_prompt_digest"]=binary::record!(prompt.token_digest);r["prompt_digest"]=binary::record!(digest(&prompt.token_ids)?);
                r["provided"]=binary::record!(prompt.provided);r["excluded"]=binary::record!(prompt.excluded);
                r["generated_evidence"]=binary::record!(e.request.evidence);
                let attempt=prepare_call(&root,&label,"generation",&b,e,i)?;
                r["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy().to_string());
                append_row(&mut f,&r)?;
                publish_confirmed(&attempt.with_file_name(attempt.file_name().unwrap().to_string_lossy().replace("-prepared","-resolved")),
                    &binary::record!({"prepared":file_hash(&attempt)?,"state":"RETURNED","row":digest(&r)?,"calls":1,"resumable":false,"control":{"generation_calls":1}}))?;
            }
            let rows=value_rows(&p,j,true)?;let summary=value_score(&p,j,&rows)?;
            assert_eq!(summary["total"],2);assert!(summary["full"].as_u64().unwrap()<2);
            if j==2{
                let mut malformed_outside=rows.clone();
                let text="[event:bad [event:87654321] tail";
                assert!(!panel.1[1].request.evidence.items.iter().any(|x|x.event_id==87654321));
                let ids=tok.encode(text.as_bytes())?;let mut raw=ids.clone();raw.push(EOS);
                let r=&mut malformed_outside[1];
                r["raw_tokens"]=binary::record!(raw);r["generation"]["tokens"]=binary::record!(ids);
                r["generation"]["generated"]=binary::record!(raw.len());r["generation"]["finish"]=binary::record!("stop");
                r["finish_reason"]=binary::record!("stop");r["generation_completed"]=binary::record!(true);
                r["actual"]=binary::record!(text);r["error"]=binary::Value::Null;
                r["decode_error"]=binary::Value::Null;r["error_class"]=binary::Value::Null;
                r["generation_error"]=binary::Value::Null;r["command_stop"]=binary::Value::Null;
                r["exact_match"]=binary::record!(false);
                let scored=value_score(&p,j,&malformed_outside)?;
                assert_eq!(scored["malformed"],2);assert_eq!(scored["outside"],1);
            }
        }
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
    #[test]
    #[ignore="read-only C returned192 selection and native identity; zero model calls"]
    fn value_reading_actual_selection()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_FIRST_EVAL_ROOT").map_err(|_|bad("explicit completed eval root"))?);
        let d:FirstEvalOnly=read_confirmed(&root.join("eval-only-plan.r3b"))?;
        let s:Study=read_confirmed(&d.original.join("plan.r3b"))?;
        let p=&d.endpoints[0];
        assert_eq!(file_hash(&p.native)?,"1f5656d5c033a5cba3d271c3939aed66aaa14f4a00b11cc30f8081af5db28a90");
        let model=checkpoint::metadata(&p.native)?.0.model_content_digest;
        for (name,train) in [("FULL-train",true),("FULL-word",false)]{
            let panel=fit_panels(&s)?.into_iter().find(|x|x.0==name).ok_or_else(||bad("panel"))?;
            let (_,rows)=event_read_panel(&s,"C",256,&model,&panel,192)?;
            assert_eq!(rows.len(),192);
            let indices=value_indices(&panel.1[..192],&panel.2[..192],train)?;
            assert_eq!(indices.len(),48);
            let mut duplicate=panel.1[..192].to_vec();duplicate[1].id=duplicate[0].id.clone();
            assert!(value_indices(&duplicate,&panel.2[..192],train).is_err());
            let mut missing=panel.2[..192].to_vec();missing[0].view=3;
            assert!(value_indices(&panel.1[..192],&missing,train).is_err());
            assert!(value_indices(&panel.1[..191],&panel.2[..191],train).is_err());
            println!("VALUE_SELECTION {} 48/192 native={} calls0",name,p.native.display());
        }
        Ok(())
    }
    use super::*;
    #[test]
    #[ignore="read-only closed v7 origin, raw and native receipts; zero model calls"]
    fn first_eval_only_origin_closed_v7()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_FIRST_EVAL_ORIGINAL").map_err(|_|bad("explicit failed v7 root"))?);
        let(s,end,trace,usage,refs)=first_eval_origin(&root)?;
        assert_eq!((end[0].local,end[1].local),(256,256));
        assert_eq!((usage.1,usage.2),(1408,0));
        assert_eq!((usage.1+768+64,usage.2+192+16),(2240,208));
        assert_eq!(trace["C"]["weighted_denominator"],29952);
        assert_eq!(trace["W"]["weighted_denominator"],30976);
        assert!(refs.len()>20);
        assert!(!s.root.join("W/eval-256-value.r3rows").exists());
        let(cases,_,selection)=first_review_cases(&s,0,&end[0])?;
        assert!(cases.len()>=16&&cases.len()<=32);
        assert_eq!(selection["ids"].as_array().unwrap().len(),cases.len());
        assert!(first_eval_output(&s.root,&s.root.parent().unwrap().join("first-decision-20260925-unregistered")).is_err());
        println!("FIRST_EVAL_ORIGIN prior={usage:?} refs={} original_failed=true model_calls=0",refs.len());Ok(())
    }
    #[test]
    #[ignore="model-free immutable main usage after completed B observations"]
    fn first_eval_only_report_usage_sticky_after_reviews()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_FIRST_EVAL_ROOT").map_err(|_|bad("explicit eval-only root"))?);
        let(d,_,mut view)=first_eval_load(&root,false)?;
        let fixture=root.parent().unwrap().join("first-decision-20260925-evidence")
            .join(format!("eval-only-usage-fixture-{}",std::process::id()));
        std::fs::create_dir(&fixture)?;view.root=fixture.clone();
        let mut main=None;
        for(label,seconds,generation,teacher)in [("eval-only",1.,768,192),("review-C",2.,32,8),("review-W",3.,32,8)]{
            let start=fixture.join(format!("{label}-segment-000-started.r3b"));
            publish_confirmed(&start,&binary::record!({"policy":digest(&view)?}))?;
            publish_confirmed(&fixture.join(format!("{label}-segment-000-finished.r3b")),&binary::record!({
                "start":file_hash(&start)?,"success":true,"resume":false,"control":{"observed_conditions":[],
                    "elapsed_seconds":seconds,"generation_calls":generation,"teacher_calls":teacher}}))?;
            let stable=first_eval_usage_labels(&d,&view,&["eval-only"])?;
            if let Some(before)=main{assert_eq!(stable,before);}else{main=Some(stable);}
        }
        let cumulative=first_eval_usage(&d,&view)?;
        assert_eq!((cumulative.1,cumulative.2),(2240,208));
        assert_eq!((main.unwrap().1,main.unwrap().2),(2176,192));
        println!("FIRST_EVAL_STICKY main={:?} after_B={cumulative:?} fixture={} model_calls=0",main.unwrap(),fixture.display());Ok(())
    }
    #[test]
    #[ignore="read-only real completed main panels/teacher and malformed nested parity; zero model calls"]
    fn first_eval_only_completed_main_reader()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_FIRST_EVAL_ROOT").map_err(|_|bad("explicit completed eval-only root"))?);
        let(d,original,main)=first_eval_load(&root,false)?;
        let refs=first_eval_main_refs(&d,&original,&main)?;
        let train=first_teacher_cases(&main,true)?;let dev=first_teacher_cases(&main,false)?;
        let positions=first_review_teacher_positions(&train,&dev)?;
        let es=train.into_iter().chain(dev).collect::<Vec<_>>();let tok=scoring_tokenizer(&main)?;
        for arm in 0..2{
            let raw=first_eval_teacher_read(&main,arm,&d.endpoints[arm])?;
            let teacher=positions.iter().map(|&i|es[i].clone()).collect::<Vec<_>>();
            let selected=positions.iter().map(|&i|raw[i].clone()).collect::<Vec<_>>();
            first_eval_teacher_parity(&teacher,&selected,&selected,&tok)?;
            for key in ["gold","argmax","nll"]{
                let mut bad=selected.clone();let value=&mut bad[0]["teacher"]["target_token_observation"][key];
                let mut parts=value.as_array().ok_or_else(||super::bad("nested teacher array"))?.to_vec();
                parts[0]=if key=="nll"{binary::record!(parts[0].as_f64().unwrap()+1.)}else{binary::record!(999999u32)};
                *value=binary::record!(parts);
                assert!(first_eval_teacher_parity(&teacher,&bad,&selected,&tok).is_err(),"{key} mismatch must fail");
            }
        }
        println!("FIRST_EVAL_MAIN_READER refs={} teacher96x2 nested_negative6 model_calls=0",refs.len());Ok(())
    }
    #[test]
    #[ignore="actual saved six-panel report and both B preflight manifests; zero model calls"]
    fn first_eval_only_post_main_report_reader()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_FIRST_EVAL_ROOT").map_err(|_|bad("explicit completed eval-only root"))?);
        let(d,original,main)=first_eval_load(&root,false)?;
        let(source,runtime,_)=first_eval_post_main(&d,&original,&main)?;
        let mut review=main.clone();review.source=source;review.runtime=runtime;
        first_eval_report(&d,&original,&main,&review)?;
        let report:binary::Value=read_confirmed(&root.join("eval-only-report.r3b"))?;
        assert_eq!(report["scores"].as_array().unwrap().len(),2);
        for arm in 0..2{
            assert_eq!(report["scores"][arm].as_object().unwrap().len(),6);
            assert!(report["scores"][arm]["OLD_FULL"].is_null());
            let src=if arm==0{&original}else{&main};
            let(cases,_,selection)=first_review_cases(src,arm,&d.endpoints[arm])?;
            assert!(cases.len()>=16&&cases.len()<=32);
            assert_eq!(selection["ids"].as_array().unwrap().len(),cases.len());
        }
        assert_eq!(report["usage"][1],2176);assert_eq!(report["usage"][2],192);
        println!("FIRST_EVAL_REPORT_READER six_panels_two_arms B_preflight_two_arms model_calls=0");Ok(())
    }
    fn first_tiny_fixture()->Result<(Study,PathBuf)>{
        let(mut original,root)=event_fixture()?;
        if original.full_fit.is_none(){return Err(bad("first-decision TINY needs FULL preparation"));}
        let origin=root.join("origin");std::fs::create_dir(&origin)?;original.root=origin.canonicalize()?;
        publish_confirmed(&original.root.join("plan.r3b"),&original)?;
        let mut l=checkpoint::load(&original.parent,Device::Cpu,true)?;
        let st=l.manifest.training.as_mut().unwrap();st.step=17981;st.sampler_state=3069;st.config=original.config.clone();
        st.consumed_tokens=st.config.budget_start_tokens;st.target_tokens=0;
        st.resume_binding=Some(bind(&original,0,st,&l.tokenizer)?);
        let mut op=l.manifest.optimizer_protocol.clone().unwrap();op.local_step=3645;op.study_start_step=Some(14912);
        l.manifest.optimizer_protocol=Some(op.clone());
        let parent=root.join("first-tiny-parent.r3m");
        checkpoint::save(&parent,&l.model,&l.tokenizer,l.manifest.clone(),&l.optimizer)?;
        let mut s=original.clone();s.parent=parent;s.parent_hash=file_hash(&s.parent)?;s.parent_content=l.model.weights_content_id()?;
        s.parent_adam=optimizer_hash(&l.optimizer)?;s.parent_step=17981;s.event.as_mut().unwrap().optimizer=op;
        s.config.budget_start_step=17981;s.config.max_steps=18237;s.config.max_tokens=s.config.budget_start_tokens+500_000;
        s.tape=(0..2).map(|i|original.tape[(3069+i)%3072]).collect();s.max_updates=2;
        let(c,tm,_)=inputs(&original)?;let role_map=checkpoint::ResumeBinding::digest_bytes(&first_roles(&c,&tm)?);
        s.first_decision=Some(FirstDecision{original:original.root.clone(),plan_hash:file_hash(&original.root.join("plan.r3b"))?,
            terminal_hash:"0".repeat(64),posthoc:original.root.clone(),posthoc_plan_hash:"0".repeat(64),
            source_index:573,role_map,backward_cap:10,trace_digest:"0".repeat(64)});
        Ok((s,root))
    }
    #[test]
    #[ignore="first-decision Metal TINY C/W continuous2 vs fresh-process1+1; direct8 updates/backwards"]
    fn first_decision_tiny_process_and_binding()->Result<()> {
        if let Ok(root)=std::env::var("R3_FIRST_TINY_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(root).join("plan.r3b"))?;
            let arm=std::env::var("R3_FIRST_TINY_ARM").unwrap().parse::<usize>().unwrap();
            let mut p:Progress=read_confirmed(&s.root.join(format!("progress-{}-1.r3b",s.arms()[arm])))?;
            event_tiny_steps(&s,arm,&mut p,1,1)?;return Ok(());
        }
        let(s,root)=first_tiny_fixture()?;
        let continuous=event_test_root(&s,&root.join("first-decision-20260925-continuous"))?;
        let split=event_test_root(&s,&root.join("first-decision-20260925-split"))?;
        for arm in 0..2{
            let initial=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
            let mut full=initial.clone();event_tiny_steps(&continuous,arm,&mut full,2,0)?;
            let mut partial=initial;event_tiny_steps(&split,arm,&mut partial,1,0)?;
            let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::first_decision_tiny_process_and_binding","--ignored","--nocapture","--test-threads=1"])
                .env("R3_FIRST_TINY_CHILD",&split.root).env("R3_FIRST_TINY_ARM",arm.to_string()).status()?;
            assert!(status.success());
            partial=read_confirmed(&split.root.join(format!("progress-{}-2.r3b",s.arms()[arm])))?;
            let a=checkpoint::load(&full.native,Device::Cpu,true)?;let b=checkpoint::load(&partial.native,Device::Cpu,true)?;
            assert_eq!(a.model.weights_content_id()?,b.model.weights_content_id()?);
            assert_eq!(optimizer_hash(&a.optimizer)?,optimizer_hash(&b.optimizer)?);
            assert_eq!(a.manifest.optimizer_protocol,b.manifest.optimizer_protocol);
            let st=a.manifest.training.as_ref().unwrap();assert_eq!((st.step,st.sampler_state,a.manifest.optimizer_protocol.as_ref().unwrap().local_step),(17983,2,3647));
            assert_eq!(st.resume_binding.as_ref().unwrap().family,if arm==0{checkpoint::ANSWER_MEAN_FAMILY}else{7});
            let mut wrong=s.clone();wrong.parent_hash="0".repeat(64);
            assert!(load_arm(&wrong,arm,&Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false},&Device::Cpu).is_err());
            assert!(load_arm(&split,1-arm,&partial,&Device::Cpu).is_err());
            let mut wrong=s.clone();wrong.first_decision.as_mut().unwrap().role_map=[0;32];
            assert!(load_arm(&wrong,arm,&partial,&Device::Cpu).is_err());
            let mut tampered=checkpoint::load(&partial.native,Device::Cpu,true)?;
            tampered.manifest.training.as_mut().unwrap().resume_binding=None;
            let path=root.join(format!("missing-objective-{arm}.r3m"));
            assert!(checkpoint::save(&path,&tampered.model,&tampered.tokenizer,tampered.manifest,&tampered.optimizer).is_err());
        }
        for local in 1..=2{
            let c:binary::Value=read_confirmed(&continuous.root.join("C").join(format!("fixture-trace-{local}.r3b")))?;
            let w:binary::Value=read_confirmed(&continuous.root.join("W").join(format!("fixture-trace-{local}.r3b")))?;
            for field in ["rows","input","target","padding"]{assert_eq!(c[field],w[field],"C/W FULL corpus {field}");}
        }
        println!("FIRST_TINY direct optimizer8 backward8 generation0 teacher0; synthetic nonzero Adam fixture; evidence={}",root.display());Ok(())
    }
    #[test]
    #[ignore="model-free first-decision C/W FULL corpus, framing, cost and native binding parity"]
    fn first_decision_full_corpus_both_arms()->Result<()> {
        let(s,root)=first_tiny_fixture()?;let full=event_inputs(&s,0)?;let word=event_inputs(&s,1)?;
        let mut old=s.clone();old.first_decision=None;let id_only=event_inputs(&old,1)?;
        assert_ne!(s.event.as_ref().unwrap().corpus_hashes[0],s.event.as_ref().unwrap().corpus_hashes[1]);
        assert_eq!(full.manifest.train.sha256,word.manifest.train.sha256);
        assert_eq!(full.manifest.validation.sha256,word.manifest.validation.sha256);
        assert_eq!(full.train.iter().map(|e|&e.id).collect::<Vec<_>>(),word.train.iter().map(|e|&e.id).collect::<Vec<_>>());
        assert_ne!(full.train[6144].answer,id_only.train[6144].answer);
        let tok=scoring_tokenizer(&s)?;
        let a=samples_with_framing(&full.train,&tok,s.config.seq_len,neural::Framing::QuestionEvidence)?;
        let b=samples_with_framing(&word.train,&tok,s.config.seq_len,neural::Framing::QuestionEvidence)?;
        for draw in &s.tape{
            let mut costs=[[0usize;3];2];
            for (arm,samples) in [&a,&b].into_iter().enumerate(){
                let length=draw.iter().map(|&i|samples[i].tokens.len()-1).max().unwrap();
                for &i in draw{costs[arm][0]+=samples[i].tokens.len()-1;
                    costs[arm][1]+=samples[i].tokens.len()-samples[i].response_start;
                    costs[arm][2]+=length-(samples[i].tokens.len()-1);}
            }
            assert_eq!(costs[0],costs[1]);assert!(costs[0][1]>0);
            for &i in draw{assert_eq!(a[i].tokens,b[i].tokens);assert_eq!(a[i].response_start,b[i].response_start);}
        }
        let p=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
        let(c,_)=load_arm(&s,0,&p,&Device::Cpu)?;let(w,_)=load_arm(&s,1,&p,&Device::Cpu)?;
        let cs=c.manifest.training.as_ref().unwrap();let ws=w.manifest.training.as_ref().unwrap();
        assert_eq!(cs.corpus_hash,full.manifest.train.sha256);assert_eq!(ws.corpus_hash,cs.corpus_hash);
        assert_eq!(cs.validation_hash,full.manifest.validation.sha256);assert_eq!(ws.validation_hash,cs.validation_hash);
        assert_eq!(cs.resume_binding,Some(bind(&s,0,cs,&tok)?));assert_eq!(ws.resume_binding,Some(bind(&s,1,ws,&tok)?));
        assert_ne!(cs.resume_binding,ws.resume_binding);
        println!("FIRST_FULL_BOTH C/W sample IDs, prompt/target framing, tape costs and native FULL hashes; optimizer0 backward0 generation0 teacher0 evidence={}",root.display());Ok(())
    }
    #[test]
    #[ignore="read-only actual FULL metadata teacher48/48 and B8 coverage; no model calls"]
    fn first_decision_teacher_selection_no_call()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_FIRST_P2_ROOT").map_err(|_|bad("explicit completed first-decision root required"))?);
        let s:Study=read_confirmed(&root.join("plan.r3b"))?;
        let c=event_inputs(&s,0)?;let(_,tm,dm)=inputs(&s)?;
        let train=&c.train[6144..7680];let train_meta=&tm[6144..7680];
        let dev=&c.validation[3072..3264];let dev_meta=&dm[3072..3264];
        let ti=first_teacher_indices(train,train_meta,true)?;let di=first_teacher_indices(dev,dev_meta,false)?;
        assert_eq!((ti.len(),di.len()),(48,48));
        let t=ti.iter().map(|&i|train[i].clone()).collect::<Vec<_>>();let d=di.iter().map(|&i|dev[i].clone()).collect::<Vec<_>>();
        assert_eq!(first_teacher_cases(&s,true)?.iter().map(|e|&e.id).collect::<Vec<_>>(),t.iter().map(|e|&e.id).collect::<Vec<_>>());
        assert_eq!(first_teacher_cases(&s,false)?.iter().map(|e|&e.id).collect::<Vec<_>>(),d.iter().map(|e|&e.id).collect::<Vec<_>>());
        let positions=first_review_teacher_positions(&t,&d)?;assert_eq!(positions.len(),8);
        let main=t.iter().chain(&d).collect::<Vec<_>>();let mut words=BTreeSet::new();let mut versions=[0usize;2];
        for &i in &positions{let(value,_)=event_label(&main[i].request,false)?;words.insert(value);
            if i<48{versions[usize::from(main[i].family.ends_with("/id1"))]+=1;}}
        assert_eq!(versions,[2,2]);assert_eq!(words,["왼쪽","오른쪽","직진","대기"].into_iter().map(str::to_owned).collect());
        let mut malformed=train_meta.to_vec();malformed[0].view=4;
        assert!(first_teacher_indices(train,&malformed,true).is_err());
        let mut missing=train_meta.to_vec();missing[1].view=missing[0].view;
        assert!(first_teacher_indices(train,&missing,true).is_err());
        let mut wrong_version=train_meta.to_vec();wrong_version[0].template="words0-1/id9".into();
        assert!(first_teacher_indices(train,&wrong_version,true).is_err());
        let mut duplicate=train.to_vec();duplicate[1].id=duplicate[0].id.clone();
        assert!(first_teacher_indices(&duplicate,train_meta,true).is_err());
        let mut malformed_dev=dev_meta.to_vec();malformed_dev[0].view=4;
        assert!(first_teacher_indices(dev,&malformed_dev,false).is_err());
        let mut wrong_b=t.clone();wrong_b[0].answer="invalid".into();
        assert!(first_review_teacher_positions(&wrong_b,&d).is_err());
        println!("FIRST_TEACHER_SELECT actual train48 dev48 B8 all4words trainID0/1=2/2; malformed/missing/duplicate rejected; optimizer0 backward0 generation0 teacher0 root={}",root.display());Ok(())
    }
    #[test]
    #[ignore="first-decision model-free actual dispatcher/RETURNED/cancel/UNKNOWN/I-O; zero model calls"]
    fn first_decision_dispatch_and_returned()->Result<()> {
        let(s,root)=first_tiny_fixture()?;
        let s=event_test_root(&s,&root.join("first-decision-20260925-dispatch"))?;
        assert_eq!(first_dispatch(0,2,0,2,10)?,FirstDispatch::Train);
        assert_eq!(first_dispatch(2,2,8,2,10)?,FirstDispatch::EvaluateOnly);
        assert_eq!(first_dispatch(1,2,8,2,10)?,FirstDispatch::CostStop);
        assert!(first_dispatch(3,2,0,2,10).is_err());
        assert!(first_dispatch(1,2,9,2,10).is_err());
        let discard=s.root.join("C").join("fixture-discarded.r3b");
        publish_confirmed(&discard,&binary::record!({"forward_backward":1,"optimizer":0,"input":27,"target":5,"reason":"TIME_BUDGET","synthetic_accounting_fixture":true}))?;
        assert_eq!(first_discarded(&s,0)?,[1,27,5]);
        let l=checkpoint::load(&s.parent,Device::Cpu,false)?;
        let mut panel=fit_panels(&s)?.remove(3);panel.1.truncate(4);panel.2.truncate(4);
        let label="first-returned";let binding=event_binding(&s,"C",0,&l.model.weights_content_id()?,&panel)?;
        event_returned_fixture(&s,&s.root.join("C"),label,&binding,&panel.1,&l.tokenizer,false)?;
        let raw=s.root.join("C").join(format!("{label}.r3rows"));let before=file_hash(&raw)?;
        let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;
        zero.set_call_limits(0,0);
        assert_eq!(generated_until(&l,&s.root.join("C"),label,&panel.1,&binding,&mut zero,4)?.len(),4);
        zero.seal_completed_no_call()?;
        assert_eq!(zero.receipt()["generation_calls"],0);assert_eq!(before,file_hash(&raw)?);
        let cancel=std::sync::Arc::new(AtomicBool::new(true));
        let mut ctl=RunControl::new(cancel,std::time::Duration::from_secs(30),u64::MAX)?;
        assert!(generated_until(&l,&s.root.join("C"),"first-cancelled",&panel.1,&binding,&mut ctl,1).is_err());
        assert_eq!(ctl.receipt()["generation_calls"],0);
        let pending=prepare_call(&s.root.join("C"),"first-unknown","generation",&binding,&panel.1[0],0)?;
        let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),u64::MAX)?;
        assert!(generated_until(&l,&s.root.join("C"),"first-unknown",&panel.1,&binding,&mut ctl,1).is_err());
        assert_eq!(ctl.receipt()["generation_calls"],0);assert!(pending.exists());
        let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),u64::MAX)?;
        assert!(generated_until(&l,&s.root.join("missing-directory"),"first-io",&panel.1,&binding,&mut ctl,1).is_err());
        assert_eq!(ctl.receipt()["generation_calls"],0);
        // A synthetic clocked native proves the actual pair dispatcher can
        // finish a segment with no new optimizer, backward or model call.
        let mut ps=vec![];
        for arm in 0..2{
            let mut p=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
            let(mut fork,mut opt)=load_arm(&s,arm,&p,&Device::Cpu)?;
            p.local=1;opt.protocol.local_step=3646;
            let st=fork.manifest.training.as_mut().unwrap();st.step=17982;st.sampler_state=1;
            save_arm(&s,0,arm,&mut fork,&opt,&mut p)?;ps.push(p);
        }
        publish_confirmed(&s.root.join("review-a.r3b"),&binary::record!({"active_seconds":0.,"synthetic_fixture":true}))?;
        publish_confirmed(&s.root.join("baseline-finished.r3b"),&binary::record!({"success":true,"synthetic_fixture":true}))?;
        publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?,"previous":null,"phase":0,"arms":ps}))?;
        let receipt=binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]});
        publish_confirmed(&s.root.join("segment-000-finished.r3b"),&Segment{policy:digest(&s)?,previous:None,phase:0,arms:ps,success:false,resume:true,control:receipt,error:Some("TIME_BUDGET".into())})?;
        first_train(&s)?;let h=history(&s)?;let latest=h.last().unwrap();
        assert_eq!(latest.phase,1);assert!(latest.resume);
        assert!(latest.arms.iter().all(|p|p.local==1&&p.evaluated==0));
        assert_eq!(latest.control["generation_calls"],0);assert_eq!(latest.control["teacher_calls"],0);
        println!("FIRST_DISPATCH actual RETURNED4, cancel/UNKNOWN/I-O rejected, optimizer0 backward0 generation0 teacher0 evidence={}",root.display());Ok(())
    }
    #[test]
    #[ignore="model-free actual cost-stop caller, saved endpoints and unequal-endpoint report; zero model calls"]
    fn first_decision_cost_stop_endpoints()->Result<()> {
        if let Ok(path)=std::env::var("R3_FIRST_COST_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(path).join("plan.r3b"))?;
            first_train(&s)?;first_report(&s)?;
            let h=history(&s)?;let end=h.last().unwrap();let unequal=s.root.file_name().unwrap().to_string_lossy().contains("unequal");
            assert!(end.success&&!end.resume&&end.arms.iter().all(|p|p.fit));
            assert_eq!((end.arms[0].local,end.arms[1].local),if unequal{(1,0)}else{(0,0)});
            assert_eq!(end.arms[if unequal{1}else{0}].stop.as_deref(),Some("BUDGET_EXHAUSTED"));
            if unequal{assert_eq!(end.arms[0].stop.as_deref(),Some("UNMATCHED_COST_ENDPOINT"));}
            assert_eq!(end.control["generation_calls"],0);assert_eq!(end.control["teacher_calls"],0);
            assert!(first_train(&s).is_err(),"closed cost endpoint must not resume training");
            return Ok(());
        }
        let(s,root)=first_tiny_fixture()?;
        for unequal in [false,true]{
            let s=event_test_root(&s,&root.join(if unequal{"cost-unequal"}else{"cost-equal"}))?;
            publish_confirmed(&s.root.join("review-a.r3b"),&binary::record!({"active_seconds":0.,"synthetic_fixture":true}))?;
            publish_confirmed(&s.root.join("baseline-finished.r3b"),&binary::record!({"success":true,"synthetic_fixture":true}))?;
            let mut progress=vec![Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};2];
            if unequal{
                let(mut fork,mut opt)=load_arm(&s,0,&progress[0],&Device::Cpu)?;
                progress[0].local=1;opt.protocol.local_step=3646;
                let st=fork.manifest.training.as_mut().unwrap();st.step=17982;st.sampler_state=1;
                save_arm(&s,0,0,&mut fork,&opt,&mut progress[0])?;
                let samples=samples_with_framing(&event_inputs(&s,0)?.train,&fork.tokenizer,s.config.seq_len,neural::Framing::QuestionEvidence)?;
                let draw=&s.tape[0];let length=draw.iter().map(|&i|samples[i].tokens.len()-1).max().unwrap();
                let mut cost=[0usize;3];for &i in draw{cost[0]+=samples[i].tokens.len()-1;cost[1]+=samples[i].tokens.len()-samples[i].response_start;cost[2]+=length-(samples[i].tokens.len()-1);}
                let mut trace=std::fs::File::create(s.root.join("C/updates-000.r3rows"))?;
                append_row(&mut trace,&binary::record!({"local":1,"model_step":17982,"optimizer_local":3646,"source_tape_index":573,"rows":draw,
                    "input":cost[0],"target":cost[1],"padding":cost[2],"examples":8,"weighted_denominator":cost[1],
                    "plain_ce":1.,"first_nll":1.,"answer_ce":1.,"synthetic_accounting_fixture":true}))?;
                let receipt=binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]});
                publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?,"previous":null,"phase":0,"arms":progress}))?;
                publish_confirmed(&s.root.join("segment-000-finished.r3b"),&Segment{policy:digest(&s)?,previous:None,phase:0,arms:progress.clone(),success:false,resume:true,control:receipt,error:Some("TIME_BUDGET".into())})?;
            }
            let stopped=if unequal{1}else{0};
            for i in 0..8{publish_confirmed(&s.root.join(s.arms()[stopped]).join(format!("fixture-{i}-discarded.r3b")),
                &binary::record!({"forward_backward":1,"optimizer":0,"input":27,"target":5,"synthetic_accounting_fixture":true}))?;}
            let l=checkpoint::load(&s.parent,Device::Cpu,false)?;let model=l.model.weights_content_id()?;
            for arm in 0..2{for panel in fit_panels(&s)?.into_iter().take(6){
                let local=progress[arm].local;let label=format!("eval-{local}-{}",panel.0);
                let binding=event_binding(&s,s.arms()[arm],local,&model,&panel)?;
                event_returned_fixture(&s,&s.root.join(s.arms()[arm]),&label,&binding,&panel.1[..4],&l.tokenizer,false)?;
            }}
            let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::first_decision_cost_stop_endpoints","--ignored","--nocapture","--test-threads=1"])
                .env("R3_FIRST_COST_CHILD",&s.root).status()?;assert!(status.success());
        }
        println!("FIRST_COST actual equal0 and unequal1/0 report, RETURNED fixtures, optimizer0 backward0 generation0 teacher0 evidence={}",root.display());Ok(())
    }
    #[test]
    #[ignore="model-free +128 regular decision followed by exhausted discard reserve and cost-final report"]
    fn first_decision_cost_after_regular_128()->Result<()> {
        if let Ok(path)=std::env::var("R3_FIRST_COST128_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(path).join("plan.r3b"))?;
            let before:binary::Value=read_confirmed(&s.root.join("fixture-regular-hashes.r3b"))?;
            first_train(&s)?;first_report(&s)?;
            let h=history(&s)?;let p=&h.last().unwrap().arms;
            let unequal=s.root.file_name().unwrap().to_string_lossy().contains("unequal");
            assert!(h.last().unwrap().success&&!h.last().unwrap().resume&&p.iter().all(|x|x.fit));
            assert_eq!((p[0].local,p[1].local),if unequal{(127,128)}else{(128,128)});
            assert_eq!(p[0].stop.as_deref(),Some("BUDGET_EXHAUSTED"));
            assert_eq!(p[1].stop.as_deref(),Some(if unequal{"UNMATCHED_COST_ENDPOINT"}else{"MATCHED_COST_ENDPOINT"}));
            assert_eq!(h.last().unwrap().control["generation_calls"],0);
            assert_eq!(h.last().unwrap().control["teacher_calls"],0);
            for arm in if unequal{1..2}else{0..2}{
                let regular=s.root.join(s.arms()[arm]).join("decision-128.r3b");
                assert_eq!(file_hash(&regular)?,before[s.arms()[arm]]);
                let cost:binary::Value=read_confirmed(&s.root.join(s.arms()[arm]).join("cost-decision-128.r3b"))?;
                assert_eq!(cost["prior_decision_hash"],before[s.arms()[arm]]);
                assert_eq!(cost["scores"]["FULL-word"]["joint"]["total"],4);
            }
            assert!(first_train(&s).is_err());return Ok(());
        }
        let(mut s,root)=first_tiny_fixture()?;
        let original:Study=read_confirmed(&s.first_decision.as_ref().unwrap().original.join("plan.r3b"))?;
        s.tape=(0..256).map(|i|original.tape[(3069+i)%3072]).collect();s.max_updates=256;
        s.first_decision.as_mut().unwrap().backward_cap=264;
        for unequal in [false,true]{let s=event_test_root(&s,&root.join(if unequal{"first-decision-20260925-cost128-unequal"}else{"first-decision-20260925-cost128-equal"}))?;
        publish_confirmed(&s.root.join("review-a.r3b"),&binary::record!({"active_seconds":0.,"synthetic_fixture":true}))?;
        publish_confirmed(&s.root.join("baseline-finished.r3b"),&binary::record!({"success":true,"synthetic_fixture":true}))?;
        let tok=scoring_tokenizer(&s)?;let samples=samples_with_framing(&event_inputs(&s,0)?.train,&tok,s.config.seq_len,neural::Framing::QuestionEvidence)?;
        let mut ps=vec![];let mut hashes=BTreeMap::new();
        for arm in 0..2{
            let mut p=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
            let(mut fork,mut opt)=load_arm(&s,arm,&p,&Device::Cpu)?;
            p.local=if unequal&&arm==0{127}else{128};opt.protocol.local_step=3645+p.local;
            let st=fork.manifest.training.as_mut().unwrap();st.step=17981+p.local;st.sampler_state=p.local as u64;
            save_arm(&s,0,arm,&mut fork,&opt,&mut p)?;
            let mut trace=std::fs::File::create(s.root.join(s.arms()[arm]).join("updates-000.r3rows"))?;
            for (i,draw) in s.tape[..p.local].iter().enumerate(){
                let length=draw.iter().map(|&j|samples[j].tokens.len()-1).max().unwrap();let mut cost=[0usize;3];
                for &j in draw{cost[0]+=samples[j].tokens.len()-1;cost[1]+=samples[j].tokens.len()-samples[j].response_start;cost[2]+=length-(samples[j].tokens.len()-1);}
                append_row(&mut trace,&binary::record!({"local":i+1,"model_step":17982+i,"optimizer_local":3646+i,
                    "source_tape_index":(573+i)%3072,"rows":draw,"input":cost[0],"target":cost[1],"padding":cost[2],
                    "examples":8,"weighted_denominator":cost[1]+if arm==1{4}else{0},"plain_ce":1.,"first_nll":1.,"answer_ce":1.,
                    "synthetic_accounting_fixture":true}))?;
            }
            for panel in fit_panels(&s)?.into_iter().take(6){
                let label=format!("eval-{}-{}",p.local,panel.0);
                let binding=event_binding(&s,s.arms()[arm],p.local,&fork.model.weights_content_id()?,&panel)?;
                event_returned_fixture(&s,&s.root.join(s.arms()[arm]),&label,&binding,&panel.1[..4],&fork.tokenizer,false)?;
            }
            if p.local==128{
                let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;ctl.set_call_limits(0,0);
                let scores=first_evaluate(&s,arm,&mut fork,&p,false,&mut ctl)?;
                assert_eq!(first_decision(&s,arm,&p,&scores)?,None);
                hashes.insert(s.arms()[arm].to_owned(),file_hash(&s.root.join(s.arms()[arm]).join("decision-128.r3b"))?);
                p.evaluated=128;
            }
            ps.push(p);
        }
        publish_confirmed(&s.root.join("fixture-regular-hashes.r3b"),&hashes)?;
        for i in 0..8{publish_confirmed(&s.root.join("C").join(format!("fixture-{i}-discarded.r3b")),
            &binary::record!({"forward_backward":1,"optimizer":0,"input":27,"target":5,"synthetic_accounting_fixture":true}))?;}
        publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?,"previous":null,"phase":2,"arms":ps}))?;
        publish_confirmed(&s.root.join("segment-000-finished.r3b"),&Segment{policy:digest(&s)?,previous:None,phase:2,arms:ps,success:true,resume:true,
            control:binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":[]}),error:None})?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::first_decision_cost_after_regular_128","--ignored","--nocapture","--test-threads=1"])
            .env("R3_FIRST_COST128_CHILD",&s.root).status()?;assert!(status.success());
        }
        println!("FIRST_COST128 equal and unequal regular hashes preserved, cost finals separately bound, RETURNED generation0 teacher0 optimizer0 backward0 evidence={}",root.display());Ok(())
    }
    #[test]
    #[ignore="read-only real budget endpoint; no optimizer/backward/generation/teacher"]
    fn full_fit_posthoc_scope()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_FULL_FIT_PREPARATION").map_err(|_|bad("explicit completed fit root"))?);
        let(s,p,trace,usage)=fit_posthoc_origin(&root)?;let h=history(&s)?;let end=h.last().unwrap();
        assert!(fit_posthoc_output(&s.root,&s.root).is_err());assert!(fit_posthoc_output(&s.root,&s.root.join("posthoc-forbidden")).is_err());
        assert!(fit_posthoc_output(&s.root,&s.root.with_extension("new-posthoc-fixture")).is_ok());
        let scratch=std::env::temp_dir().join(format!("replica-posthoc-registration-{}",std::process::id()));std::fs::create_dir(&scratch)?;
        let original=scratch.join("original");let value=binary::record!({"explicit_test_spec":true});fit_posthoc_register(&original,&value)?;
        let registered=original.with_extension("full-fit-posthoc.r3b");let before=file_hash(&registered)?;
        assert!(fit_posthoc_register(&original,&value).is_err());assert_eq!(before,file_hash(&registered)?);assert!(!pending_path(&registered).exists());
        assert_eq!(p.local,3069);assert_eq!(trace["backward"],3072);assert_eq!(usage.1,3280);assert_eq!(usage.2,64);
        for mode in 0..5{let mut wrong=end.clone();match mode{0=>wrong.resume=true,1=>wrong.success=true,2=>wrong.arms[0].fit=true,
            3=>wrong.control["observed_conditions"]=binary::record!(["INTEGRITY_FAIL","CANCELLED"]),_=>wrong.error=Some("UNKNOWN".into())};assert!(fit_posthoc_stop(&s,&wrong,&trace).is_err());}
        let mut wrong=trace.clone();wrong["backward"]=binary::record!(3071);assert!(fit_posthoc_stop(&s,end,&wrong).is_err());
        assert!(previous_usage(&s,&h).is_err(),"original sticky failure must remain blocked");
        println!("POSTHOC_SCOPE budget-only endpoint; old failure unchanged; all model calls0");Ok(())
    }
    #[test]
    #[ignore="new-process four-row explicit fixture on production collector, all model calls0"]
    fn full_fit_posthoc_returned_process()->Result<()> {
        if let Ok(root)=std::env::var("R3_POSTHOC_RETURNED_CHILD"){
            let root=PathBuf::from(root);let s:Study=read_confirmed(&root.join("fixture-view.r3b"))?;
            let p:Progress=read_confirmed(&root.join("fixture-endpoint.r3b"))?;
            let mut panel=fit_panels(&s)?.remove(3);panel.1.truncate(4);panel.2.truncate(4);
            let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;ctl.set_call_limits(0,0);
            let b=event_binding(&s,"F",p.local,&checkpoint::metadata(&p.native)?.0.model_content_digest,&panel)?;
            let(i,mut ctl)=start_observation_control(&s,"posthoc",&b,ctl)?;
            let l=checkpoint::load(&p.native,Device::Cpu,false)?;
            fit_posthoc_collect(&s,&p,&l,&panel,&mut ctl)?;ctl.seal_completed_no_call()?;
            finish_observation(&s,"posthoc",i,&mut ctl,&Ok(()),binary::record!({"test_spec_count":4,"optimizer":0,"backward":0}))?;
            assert_eq!(ctl.receipt()["generation_calls"],0);assert_eq!(ctl.receipt()["teacher_calls"],0);
            assert_eq!(observation_history(&s,"posthoc")?.last().unwrap()["success"],true);
            assert!(start_observation_control(&s,"posthoc",&b,ctl).is_err(),"complete observations are read-only");
            return Ok(());
        }
        let original=PathBuf::from(std::env::var("R3_FULL_FIT_PREPARATION").map_err(|_|bad("explicit completed fit root"))?);
        let(mut s,p,_,_)=fit_posthoc_origin(&original)?;let original_terminal=file_hash(&original.join("segment-004-finished.r3b"))?;
        let root=std::env::temp_dir().join(format!("replica-fit-posthoc-{}",std::process::id()));std::fs::create_dir(&root)?;std::fs::create_dir(root.join("F"))?;
        s.root=root.clone();let l=checkpoint::load(&p.native,Device::Cpu,false)?;
        let mut panel=fit_panels(&s)?.remove(3);panel.1.truncate(4);panel.2.truncate(4);
        let b=event_binding(&s,"F",p.local,&l.model.weights_content_id()?,&panel)?;let label=format!("eval-{}-{}",p.local,panel.0);
        event_returned_fixture_faults(&s,&root.join("F"),&label,&b,&panel.1,&l.tokenizer,false,0..2)?;
        let path=root.join("F").join(format!("{label}.r3rows"));let before=file_hash(&path)?;
        publish_confirmed(&root.join("fixture-view.r3b"),&s)?;publish_confirmed(&root.join("fixture-endpoint.r3b"),&p)?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::full_fit_posthoc_returned_process","--ignored","--nocapture","--test-threads=1"]).env("R3_POSTHOC_RETURNED_CHILD",&root).status()?;
        assert!(status.success());assert_eq!(before,file_hash(&path)?);assert_eq!(original_terminal,file_hash(&original.join("segment-004-finished.r3b"))?);
        let(r,_)=event_read_panel(&s,"F",p.local,&l.model.weights_content_id()?,&panel,4)?;assert_eq!(r["joint"]["full"],2);
        // Missing rows at exhausted time are never generated or called complete.
        let missing="fixture-missing";let zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;
        let(i,mut zero)=start_observation_control(&s,missing,&b,zero)?;
        let result=generated_until(&l,&root,missing,&panel.1,&b,&mut zero,4).map(|_|());assert!(result.is_err());
        finish_observation(&s,missing,i,&mut zero,&result,binary::record!({"test_spec_count":4}))?;
        assert!(!root.join(format!("{missing}-finished.r3b")).exists());assert_eq!(zero.receipt()["generation_calls"],0);
        println!("POSTHOC_RETURNED fixture={} complete wrong rows preserved; new-process/final/missing calls0",root.display());Ok(())
    }
    #[test]
    fn full_fit_guard_and_legacy_encoding()->Result<()> {
        let mut j=binary::record!({"total":64,"full":64,"all4":16,"errors":0});assert_eq!(fit_guard_panel(1,&j)?,(0,false));
        j["full"]=binary::record!(56);assert_eq!(fit_guard_panel(0,&j)?,(1,false));assert_eq!(fit_guard_panel(1,&j)?,(2,true));
        j["full"]=binary::record!(48);assert!(fit_guard_panel(0,&j)?.1);j["full"]=binary::record!(64);j["errors"]=binary::record!(4);assert!(fit_guard_panel(0,&j)?.1);
        j["errors"]=binary::record!(0);j["all4"]=binary::record!(12);assert_eq!(fit_guard_panel(0,&j)?,(1,false));j["total"]=binary::record!(63);assert!(fit_guard_panel(0,&j).is_err());
        let p=Progress{local:64,native:"fixture.r3m".into(),physical:"a".repeat(64),stop:None,evaluated:64,fit:true};
        assert_eq!(binary::to_vec(&[p.clone(),p.clone()])?,binary::to_vec(&vec![p.clone(),p])?);
        assert_eq!(fit_counts(768,false,false),[64,64,64,64,64,192,0]);assert_eq!(fit_counts(1536,false,false),[512,512,512,192,192,192,0]);
        assert_eq!(fit_counts(1536,false,true)[5],1536);assert_eq!(fit_counts(3072,true,false),[512,512,512,192,192,1536,64]);Ok(())
    }
    fn event_fixture() -> Result<(Study,PathBuf)> {
        let prepared=PathBuf::from(std::env::var("R3_FULL_FIT_PREPARATION").or_else(|_|std::env::var("R3_EVENT_PREPARATION")).map_err(|_|bad("explicit event preparation required"))?);
        let mut s:Study=read_confirmed(&prepared.join("plan.r3b"))?;
        let root=if let Ok(base)=std::env::var("R3_FIRST_EVIDENCE"){
            PathBuf::from(base).join(format!("first-decision-20260925-tiny-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))
        }else{std::env::temp_dir().join(format!("replica-event-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))};std::fs::create_dir(&root)?;
        let tok=scoring_tokenizer(&s)?;let mut cfg=Config::tiny(tok.vocab_size());cfg.context=2048;cfg.profile="NATIVE_TRPP_EXPERIMENTAL_V1".into();
        let device=Backend::Metal0.open()?;s.runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
        let model=Transformer::init(cfg,20260924,device)?;
        let mut op=Optimizer::fresh(&model.config,&model.vars,false,digest(&s.runtime)?,14336)?;
        // Explicit synthetic inherited nonzero moments; no hidden512 updates.
        for (name,t)in &mut op.tensors{*t=Tensor::full(if name.starts_with("adam.m."){0.0001f32}else{0.0002},t.dims(),t.device())?;}
        let fit=s.full_fit.is_some();op.protocol.local_step=if fit{576}else{512};
        if fit{op.protocol.version=2;op.protocol.family="ADAMW-INHERITED-V1".into();op.protocol.study_start_step=Some(14848);}
        let mut manifest=checkpoint::initialized(&model,&tok,20260924,"e".repeat(64))?;
        let mut config=s.config.clone();config.budget_start_step=if fit{14848}else{14336};config.max_steps=15360;
        let mut state=fixture_state(&config,&tok,s.parent_step);state.sampler_state=if fit{64}else{512};state.initial_weight_hash=manifest.initial_weight_hash.clone();state.consumed_tokens=config.budget_start_tokens;
        manifest.training=Some(state);manifest.optimizer_protocol=Some(op.protocol.clone());manifest.status="DIAGNOSTIC_COMPLETE".into();
        let path=root.join("tiny-parent.r3m");checkpoint::save(&path,&model,&tok,manifest,&op.tensors)?;
        s.parent=path;s.parent_hash=file_hash(&s.parent)?;s.parent_content=model.weights_content_id()?;s.parent_adam=optimizer_hash(&op.tensors)?;s.event.as_mut().unwrap().optimizer=op.protocol;
        Ok((s,root))
    }
    #[test]
    #[ignore="FULL single-arm inherited Metal TINY continuous2 vs1+process1; four optimizer/backward calls"]
    fn full_fit_inherited_process()->Result<()> {
        if let Ok(root)=std::env::var("R3_FIT_RESTART_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(root).join("plan.r3b"))?;
            let mut p:Progress=read_confirmed(&s.root.join("progress-F-1.r3b"))?;
            assert_eq!(s.arms(),["F"]);assert_eq!(s.clock(1),577);event_tiny_steps(&s,0,&mut p,1,1)?;return Ok(());
        }
        let(s,root)=event_fixture()?;assert!(s.full_fit.is_some());
        let continuous=event_test_root(&s,&root.join("continuous"))?;let split=event_test_root(&s,&root.join("split"))?;
        let p=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
        let mut a=p.clone();event_tiny_steps(&continuous,0,&mut a,2,0)?;
        let mut b=p;event_tiny_steps(&split,0,&mut b,1,0)?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::full_fit_inherited_process","--ignored","--nocapture","--test-threads=1"]).env("R3_FIT_RESTART_CHILD",&split.root).status()?;assert!(status.success());
        b=read_confirmed(&split.root.join("progress-F-2.r3b"))?;
        let a=checkpoint::load(&a.native,Device::Cpu,true)?;let b=checkpoint::load(&b.native,Device::Cpu,true)?;
        assert_eq!(a.model.weights_content_id()?,b.model.weights_content_id()?);assert_eq!(optimizer_hash(&a.optimizer)?,optimizer_hash(&b.optimizer)?);
        assert_eq!(a.manifest.optimizer_protocol,b.manifest.optimizer_protocol);
        let sa=a.manifest.training.as_ref().unwrap();let sb=b.manifest.training.as_ref().unwrap();
        assert_eq!((sa.step,sa.sampler_state,sa.consumed_tokens,sa.target_tokens),(sb.step,sb.sampler_state,sb.consumed_tokens,sb.target_tokens));
        assert_eq!((sa.step,sa.sampler_state,a.manifest.optimizer_protocol.as_ref().unwrap().local_step),(14914,2,578));
        assert_eq!(file_hash(&s.parent)?,s.parent_hash);
        println!("FULL_FIT_TINY exact weights/m/v/clock/cursor; updates4 backward4 generation0 teacher0 path={}",root.display());Ok(())
    }
    fn event_test_root(s:&Study,root:&Path)->Result<Study>{let mut s=s.clone();std::fs::create_dir(root)?;s.root=root.canonicalize()?;
        for arm in s.arms(){std::fs::create_dir(s.root.join(arm))?;}publish_confirmed(&s.root.join("plan.r3b"),&s)?;Ok(s)}
    #[test]
    #[ignore="model-free FULL preparation,3072 trace,1536 panel,actual final-only process"]
    fn full_fit_final_process_and_large_counts()->Result<()> {
        if let Ok(root)=std::env::var("R3_FIT_FINAL_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(root).join("plan.r3b"))?;train(&s)?;
            let h=history(&s)?;let last=h.last().unwrap();let mode=std::env::var("R3_FIT_FINAL_MODE").unwrap();
            if mode=="train-fail"{assert!(!last.success&&last.resume&&!last.arms[0].fit);assert_eq!(last.phase,3);}
            else{assert!(last.success&&!last.resume&&last.arms[0].fit);}
            assert_eq!(last.arms.len(),1);assert_eq!(last.control["generation_calls"],0);assert_eq!(last.control["teacher_calls"],0);
            assert_eq!(h[0].arms[0].physical,last.arms[0].physical);
            let scores=fit_read_endpoint(&s,&last.arms[0],mode!="train-fail")?;
            if mode=="stop"{assert_eq!(last.arms[0].stop.as_deref(),Some("RETENTION_STOP"));assert_eq!(scores["FULL-train"]["joint"]["total"],192);assert!(!s.root.join("F/teacher-768-FULL-teachers.r3rows").exists());}
            else{assert!(fit_development(&scores));assert_eq!(fit_quality(&scores["FULL-train"],"FULL-train"),mode!="train-fail");}return Ok(());
        }
        let prepared=PathBuf::from(std::env::var("R3_FULL_FIT_PREPARATION").map_err(|_|bad("FULL preparation required"))?);
        let real:Study=read_confirmed(&prepared.join("plan.r3b"))?;fit_verify_inputs(&real)?;
        let f=real.full_fit.as_ref().unwrap();assert_eq!(f.train_order.len(),1536);assert_eq!(f.train_order.iter().collect::<BTreeSet<_>>().len(),1536);
        let source=fit_rotation(&real.tape,3072-576)?;assert_eq!(real.tape,fit_rotation(&source,576)?);assert!(fit_rotation(&source[..3071],576).is_err());
        assert!(fit_exposure(&real.tape)?[6144..].iter().all(|&x|x==8));assert_eq!(real.clock(3072),3648);
        let(template,root)=event_fixture()?;
        for(mode,step,phase)in [("final",3072,4),("early",1536,2),("train-fail",1536,2),("stop",768,1)]{
        let s=event_test_root(&template,&root.join(mode))?;
        publish_confirmed(&s.root.join("review-a.r3b"),&binary::record!({"active_seconds":7200.,"synthetic_fixture":true}))?;
        publish_confirmed(&s.root.join("baseline-finished.r3b"),&binary::record!({"success":true,"synthetic_fixture":true}))?;
        let mut p=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:2304,fit:false};
        let(mut l,mut o)=load_arm(&s,0,&p,&Device::Cpu)?;p.local=step;p.evaluated=FIT_STEPS[phase-1];o.protocol.local_step=s.clock(step);
        let st=l.manifest.training.as_mut().unwrap();st.step=s.parent_step+step;st.sampler_state=step as u64;save_arm(&s,0,0,&mut l,&o,&mut p)?;
        let mut previous:Option<String>=None;
        for &prior in &FIT_STEPS[..phase]{let path=s.root.join("F").join(format!("decision-{prior}.r3b"));
            publish_confirmed(&path,&binary::record!({"policy":digest(&s)?,"local":prior,"previous":previous,"streak":if mode=="stop"{[1,0,0]}else{[0,0,0]},"stop":null,"synthetic_fixture":true}))?;previous=Some(file_hash(&path)?);}
        let mut hashes=BTreeMap::new();let dir=s.root.join("F");
        let counts=fit_counts(step,mode!="train-fail",mode!="stop");
        for(panel,&n)in fit_panels(&s)?.into_iter().zip(&counts){if n==0{continue;}let label=format!("eval-{step}-{}",panel.0);let b=event_binding(&s,"F",step,&l.model.weights_content_id()?,&panel)?;
            let wrong=if mode=="stop"&&panel.0=="value"{0..8}else if mode=="train-fail"&&panel.0=="FULL-train"{1520..1536}else{0..0};
            event_returned_fixture_faults(&s,&dir,&label,&b,&panel.1[..n],&l.tokenizer,false,wrong)?;let path=dir.join(format!("{label}.r3rows"));hashes.insert(path.clone(),file_hash(&path)?);
            if panel.0=="FULL-train"{let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;
                let r=event_panel_score(&s,"F",step,&l,&panel,192,&mut zero)?;assert_eq!(r["joint"]["total"],192);assert_eq!(zero.receipt()["generation_calls"],0);}
        }
        if mode!="stop"{let es=event_teacher_cases(&s,0)?;let label=format!("teacher-{step}-FULL");
        let b=binary::record!({"policy":digest(&s)?,"source":s.source,"runtime":s.runtime,"model":l.model.weights_content_id()?,"arm":"F","local":step,"mode":0,"cases":digest(&es)?,"planned":64,"call_protocol":1});
        event_returned_fixture(&s,&dir,&label,&b,&es,&l.tokenizer,true)?;}
        let control=binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]});
        publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?}))?;
        let seg=Segment{policy:digest(&s)?,previous:None,phase,arms:vec![p.clone()],success:false,resume:true,control,error:Some("TIME_BUDGET".into())};
        publish_confirmed(&s.root.join("segment-000-finished.r3b"),&seg)?;
        if mode=="final"{let c=event_inputs(&s,0)?;let ss=samples_with_framing(&c.train,&l.tokenizer,256,neural::Framing::QuestionEvidence)?;
        let path=dir.join("updates-000.r3rows");let mut trace=std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
        for(n,draw)in s.tape.iter().enumerate(){let width=draw.iter().map(|&i|ss[i].tokens.len()-1).max().unwrap();let mut cost=[0;3];
            for &i in draw{cost[0]+=ss[i].tokens.len()-1;cost[1]+=ss[i].tokens.len()-ss[i].response_start;cost[2]+=width-(ss[i].tokens.len()-1);}
            append_row(&mut trace,&binary::record!({"local":n+1,"model_step":s.parent_step+n+1,"optimizer_local":s.clock(n+1),"source_tape_index":(576+n)%3072,"rows":draw,"lr":3e-5,"examples":8,"input":cost[0],"target":cost[1],"padding":cost[2],"answer_ce":0.12345678901234567f64}))?;
        }
        assert_eq!(fit_trace(&s,std::slice::from_ref(&seg))?["updates"],3072);
        let mut short=seg.clone();short.arms[0].local=3071;assert!(fit_trace(&s,&[short]).is_err());}
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::full_fit_final_process_and_large_counts","--ignored","--nocapture","--test-threads=1"]).env("R3_FIT_FINAL_CHILD",&s.root).env("R3_FIT_FINAL_MODE",mode).status()?;assert!(status.success());
        for(path,hash)in hashes{assert_eq!(file_hash(&path)?,hash);}let last=history(&s)?.last().unwrap().clone();
        let panel=fit_panels(&s)?.remove(5);assert!(event_read_panel(&s,"F",step,&"0".repeat(64),&panel,1536).is_err());
        assert!(event_read_panel(&s,"F",step,&l.model.weights_content_id()?,&panel,1537).is_err());
        let decision=s.root.join("F").join(format!("decision-{step}.r3b"));let hash=file_hash(&decision)?;
        assert_eq!(fit_decision(&s,&last.arms[0],&l.model.weights_content_id()?)?,last.arms[0].stop);assert_eq!(file_hash(&decision)?,hash);
        println!("FULL_FIT_FINAL {mode} actual new-process at7200; prefix/guard/early/continuation; calls0 updates0; evidence={}",s.root.display());}Ok(())
    }
    fn event_tiny_steps(s:&Study,arm:usize,p:&mut Progress,updates:usize,index:usize)->Result<()> {
        let device=Backend::Metal0.open()?;let(l,mut o)=load_arm(s,arm,p,&device)?;let mut l=l;
        let ss=samples_with_framing(&event_inputs(s,arm)?.train,&l.tokenizer,256,neural::Framing::QuestionEvidence)?;
        let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(120),u64::MAX)?;
        for _ in 0..updates{
            let b=batch(&ss,&s.tape[p.local],&device)?;let entry=s.root.join(s.arms()[arm]).join(format!("fixture-{}-entered.r3b",p.local+1));
            publish_confirmed(&entry,&binary::record!({"fixture":true,"cursor":p.local,"tape":s.tape[p.local]}))?;
            let mut entered=false;let trace=train_update(s,arm,&mut l,&mut o,p,&b,&mut ctl,&mut entered,&entry,Instant::now())?;
            assert!(!entered);assert_eq!(trace["optimizer_local"],s.clock(p.local));assert_eq!(trace["model_step"],s.parent_step+p.local);assert!(o.protocol.roles.values().all(|v|!*v));
            publish_confirmed(&s.root.join(s.arms()[arm]).join(format!("fixture-trace-{}.r3b",p.local)),&trace)?;
        }
        save_arm(s,index,arm,&mut l,&o,p)?;publish_confirmed(&s.root.join(format!("progress-{}-{}.r3b",s.arms()[arm],p.local)),p)?;Ok(())
    }
    #[test]
    #[ignore="actual F/I Metal caller; direct8 or independent8 updates; teacher2; generation0"]
    fn event_inherited_process_restart()->Result<()> {
        if let Ok(root)=std::env::var("R3_EVENT_RESTART_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(root).join("plan.r3b"))?;
            let arm=std::env::var("R3_EVENT_ARM").unwrap().parse::<usize>().unwrap();
            let mut p:Progress=read_confirmed(&s.root.join(format!("progress-{}-1.r3b",s.arms()[arm])))?;
            assert_eq!(p.local,1);event_tiny_steps(&s,arm,&mut p,1,1)?;return Ok(());
        }
        let(s,root)=event_fixture()?;let continuous=event_test_root(&s,&root.join("continuous"))?;let split=event_test_root(&s,&root.join("split"))?;
        for arm in 0..2{
            let initial=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
            let mut full=initial.clone();event_tiny_steps(&continuous,arm,&mut full,2,0)?;
            let mut partial=initial;event_tiny_steps(&split,arm,&mut partial,1,0)?;
            let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::event_inherited_process_restart","--ignored","--nocapture","--test-threads=1"]).env("R3_EVENT_RESTART_CHILD",&split.root).env("R3_EVENT_ARM",arm.to_string()).status()?;assert!(status.success());
            partial=read_confirmed(&split.root.join(format!("progress-{}-2.r3b",s.arms()[arm])))?;
            let a=checkpoint::load(&full.native,Device::Cpu,true)?;let b=checkpoint::load(&partial.native,Device::Cpu,true)?;
            assert_eq!(a.model.weights_content_id()?,b.model.weights_content_id()?);assert_eq!(optimizer_hash(&a.optimizer)?,optimizer_hash(&b.optimizer)?);
            assert_eq!(a.manifest.optimizer_protocol,b.manifest.optimizer_protocol);
            let sa=a.manifest.training.as_ref().unwrap();let sb=b.manifest.training.as_ref().unwrap();
            assert_eq!(sa.step,sb.step);assert_eq!(sa.sampler_state,2);assert_eq!(sa.sampler_state,sb.sampler_state);assert_eq!(sa.consumed_tokens,sb.consumed_tokens);assert_eq!(sa.target_tokens,sb.target_tokens);
            assert_eq!(a.manifest.optimizer_protocol.as_ref().unwrap().local_step,514);
            let moved=root.join(format!("moved-{arm}.r3m"));std::fs::copy(&partial.native,&moved)?;
            let mut moved_progress=partial.clone();moved_progress.native=moved;
            load_arm(&split,arm,&moved_progress,&Device::Cpu)?;
            assert!(load_arm(&split,1-arm,&moved_progress,&Device::Cpu).is_err());
            let mut wrong=split.clone();wrong.runtime.binary="0".repeat(64);
            assert!(load_arm(&wrong,arm,&moved_progress,&Device::Cpu).is_err());
            let l=checkpoint::load(&partial.native,Backend::Metal0.open()?,false)?;let es=event_teacher_cases(&split,arm)?;
            let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(60),u64::MAX)?;
            let binding=binary::record!({"fixture":true,"policy":digest(&split)?,"native":partial.physical,"mode":arm,"cases":digest(&&es[..1])?,"call_protocol":1});
            let rows=teacher_prefix(&split.root,&format!("tiny-teacher-{arm}"),&binding,&l,&es[..1],&mut ctl)?;
            let score=diagnosis::teacher_scores(&es[..1],&rows,&l.tokenizer)?;
            assert_eq!(rows[0]["teacher"]["native_forward"]["input_device"],split.runtime.actual_device);assert_eq!(rows[0]["teacher"]["native_forward"]["logits_device"],split.runtime.actual_device);
            assert_eq!(ctl.receipt()["teacher_calls"],1);if arm==1{assert_eq!(score["roles"]["ID"][0],8);assert!(score["roles"]["VALUE"].is_null());}else{assert_eq!(score["roles"]["ID"][0],8);}
        }
        assert_eq!(file_hash(&s.parent)?,s.parent_hash);
        println!("EVENT_TINY_PROCESS PASS updates8 backward8 teacher2 generation0; inherited512->514; continuous2/split1+process1; evidence={}",root.display());Ok(())
    }
    #[test]
    #[ignore="new event train caller; synthetic complete RETURNED at exhausted budget; zero model calls"]
    fn event_final_no_call_process()->Result<()> {
        if let Ok(root)=std::env::var("R3_EVENT_FINAL_CHILD"){
            let s:Study=read_confirmed(&PathBuf::from(root).join("plan.r3b"))?;train(&s)?;
            let h=history(&s)?;let last=h.last().unwrap();assert!(last.success&&!last.resume&&last.arms.iter().all(|p|p.fit));
            assert_eq!(last.control["generation_calls"],0);assert_eq!(last.control["teacher_calls"],0);
            assert_eq!(h[0].arms.iter().map(|p|&p.physical).collect::<Vec<_>>(),last.arms.iter().map(|p|&p.physical).collect::<Vec<_>>());return Ok(());
        }
        let(s,root)=event_fixture()?;let s=event_test_root(&s,&root.join("final-only"))?;
        publish_confirmed(&s.root.join("review-a.r3b"),&binary::record!({"active_seconds":s.active_cap,"synthetic_fixture":true}))?;
        publish_confirmed(&s.root.join("baseline-finished.r3b"),&binary::record!({"success":true,"synthetic_fixture":true}))?;
        let mut ps=vec![];let mut preserved=BTreeMap::new();
        for arm in 0..2{
            let mut p=Progress{local:0,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
            let(mut l,mut o)=load_arm(&s,arm,&p,&Device::Cpu)?;
            // Synthetic endpoint state, not an optimizer execution or quality result.
            p.local=128;p.evaluated=128;o.protocol.local_step=640;
            let st=l.manifest.training.as_mut().unwrap();st.step=s.parent_step+128;st.sampler_state=128;
            save_arm(&s,0,arm,&mut l,&o,&mut p)?;
            let dir=s.root.join(s.arms()[arm]);
            for panel in event_panels(&s,true,false)?{
                let binding=event_binding(&s,s.arms()[arm],128,&l.model.weights_content_id()?,&panel)?;
                let label=format!("eval-128-{}",panel.0);
                event_returned_fixture(&s,&dir,&label,&binding,&panel.1,&l.tokenizer,false)?;
                let path=dir.join(format!("{label}.r3rows"));preserved.insert(path.clone(),file_hash(&path)?);
                // First64 may already have been sealed by the final training guard.
                if panel.0=="FULL-word"||panel.0=="ID_ONLY-word"{
                    let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;
                    event_panel_score(&s,s.arms()[arm],128,&l,&panel,64,&mut ctl)?;
                    assert_eq!(ctl.receipt()["generation_calls"],0);
                }
            }
            for mode in 0..2{
                let es=event_teacher_cases(&s,mode)?;let label=format!("teacher-128-{}",if mode==0{"FULL"}else{"ID_ONLY"});
                let b=binary::record!({"policy":digest(&s)?,"source":s.source,"runtime":s.runtime,"model":l.model.weights_content_id()?,"arm":s.arms()[arm],"local":128,"mode":mode,"cases":digest(&es)?,"planned":64,"call_protocol":1});
                event_returned_fixture(&s,&dir,&label,&b,&es,&l.tokenizer,true)?;
                let path=dir.join(format!("{label}-teachers.r3rows"));preserved.insert(path.clone(),file_hash(&path)?);
            }
            ps.push(p);
        }
        let control=binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]});
        publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?}))?;
        publish_confirmed(&s.root.join("segment-000-finished.r3b"),&Segment{policy:digest(&s)?,previous:None,phase:4,arms:ps.try_into().ok().unwrap(),success:false,resume:true,control,error:Some("TIME_BUDGET".into())})?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::event_final_no_call_process","--ignored","--nocapture","--test-threads=1"]).env("R3_EVENT_FINAL_CHILD",&s.root).status()?;assert!(status.success());
        for(p,h)in preserved{assert_eq!(file_hash(&p)?,h);}
        println!("EVENT_FINAL_ONLY PASS fresh_process budget3600 exhausted RETURNED preserved, native clock640 unchanged; optimizer0 generation0 teacher0 evidence={}",s.root.display());Ok(())
    }
    fn event_returned_fixture(s:&Study,dir:&Path,label:&str,b:&binary::Value,es:&[Episode],tok:&ByteBpe,teacher:bool)->Result<()> {
        event_returned_fixture_faults(s,dir,label,b,es,tok,teacher,0..0)
    }
    fn event_returned_fixture_faults(s:&Study,dir:&Path,label:&str,b:&binary::Value,es:&[Episode],tok:&ByteBpe,teacher:bool,wrong:std::ops::Range<usize>)->Result<()> {
        let path=dir.join(format!("{label}{}.r3rows",if teacher{"-teachers"}else{""}));
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;append_row(&mut f,b)?;
        let ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),u64::MAX)?;
        for(i,e)in es.iter().enumerate(){
            let attempt=prepare_call(dir,label,if teacher{"teacher"}else{"generation"},b,e,i)?;
            let actual=if wrong.contains(&i){"틀림"}else{&e.answer};let output=tok.encode(actual.as_bytes())?;let mut ids=output.clone();ids.push(EOS);
            let samples=samples_with_framing(std::slice::from_ref(e),tok,s.config.seq_len,neural::Framing::QuestionEvidence)?;let sample=&samples[0];let n=sample.response_start;
            let row=if teacher{binary::record!({"ordinal":i,"id":e.id,"case":digest(e)?,"teacher":{"mean_nll":1.,"target_tokens_including_eos":ids.len(),"training_prompt_matches_generation":true,"answer_tokenizer_roundtrip":true,"target_token_observation":{"gold":ids,"argmax":ids,"nll":vec![1.;ids.len()]},"native_forward":{"model_device":s.runtime.actual_device,"input_device":s.runtime.actual_device,"logits_device":s.runtime.actual_device,"input_dtype":"U32","logits_dtype":"F32","logits_finite":true,"input_shape":[1,sample.tokens.len()-1],"prompt_tokens":n,"prompt_digest":digest(&&sample.tokens[..n])?,"input_tokens_digest":digest(&&sample.tokens[..sample.tokens.len()-1])?,"logits_shape":[sample.tokens.len()-n,tok.vocab_size()]}},"attempt":attempt.file_name().unwrap().to_string_lossy()})}
                else{binary::record!({"row_version":2,"id":e.id,"expected":e.answer,"question":e.request.input,"generated_evidence":e.request.evidence,"raw_tokens":ids,"actual":actual,"error":null,"finish_reason":"stop","generation_completed":true,"generation":{"tokens":output,"generated":ids.len(),"finish":"stop"},"exact_match":!wrong.contains(&i),"attempt":attempt.file_name().unwrap().to_string_lossy()})};
            append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),&ctl)?;
        }Ok(())
    }
// Independently authored review fragment. Insert within muon::tests.
// No model construction, forward, backward, generation, or optimizer step.
#[test]
fn event_review_request_and_clock_boundaries() -> Result<()> {
    let records = vec![
        record("장치0", "구역0", "왼쪽", 12_345_678, 0, "current"),
        record("장치1", "구역0", "오른쪽", 87_654_321, 0, "current"),
    ];
    let source = Episode {
        id: "independent-event-fixture/0".into(), category: 0,
        family: "independent-event-fixture".into(), binding: "keys0-1/words0-1".into(),
        sequence: "independent-event-fixture".into(),
        request: ModelRequest {
            request_id: String::new(), system: SYSTEM.into(),
            input: format!("장치0 구역0 {}", phrases(Intent::Current)[0]),
            evidence: EvidenceBundle { items: records, ..Default::default() },
            limits: GenerationLimits { context_tokens: 2048, max_tokens: 32, timeout_ms: 120_000 },
        }, answer: "왼쪽입니다. [event:12345678]".into(),
    };
    let full = event_episode(&source, false)?;
    let id = event_episode(&source, true)?;
    assert_eq!(full.request.system, EVENT_SYSTEM);
    assert_eq!(id.request.system, EVENT_SYSTEM);
    assert_eq!(full.request.input, source.request.input);
    assert_eq!(full.answer, source.answer);
    assert_eq!(id.request.input, format!("장치0 구역0 {EVENT_TASK}"));
    assert_eq!(id.answer, "12345678");
    assert_eq!(full.request.evidence, source.request.evidence);
    assert_eq!(id.request.evidence, source.request.evidence);
    assert_ne!(digest(&full.request)?, digest(&id.request)?);
    assert_eq!(event_label(&id.request, true)?, ("왼쪽".into(), 12_345_678));

    let mut query = id.request.clone();
    query.input = format!("장치1 구역0 {EVENT_TASK}");
    assert_eq!(event_label(&query, true)?, ("오른쪽".into(), 87_654_321));
    let mut swapped = source.clone();
    swapped.request.evidence.items[0].original_excerpt = "장치0의 구역0 값은 오른쪽이다.".into();
    swapped.request.evidence.items[1].original_excerpt = "장치1의 구역0 값은 왼쪽이다.".into();
    swapped.answer = "오른쪽입니다. [event:12345678]".into();
    assert_eq!(event_episode(&swapped, true)?.answer, id.answer);
    assert_ne!(event_episode(&swapped, false)?.answer, full.answer);
    let mut renamed = source.clone();
    renamed.request.evidence.items[0].event_id = 23_456_789;
    renamed.request.evidence.items[1].event_id = 98_765_432;
    renamed.answer = "왼쪽입니다. [event:23456789]".into();
    assert_eq!(event_episode(&renamed, true)?.answer, "23456789");
    let mut reordered = id.request.clone();
    reordered.evidence.items.reverse();
    assert_eq!(event_label(&reordered, true)?, event_label(&id.request, true)?);
    // A matching entity in another context must not shadow the selected record.
    let mut contexts = id.request.clone();
    contexts.evidence.items[1].original_excerpt = "장치0의 구역1 값은 오른쪽이다.".into();
    assert_eq!(event_label(&contexts, true)?.1, 12_345_678);
    contexts.input = format!("장치0 구역1 {EVENT_TASK}");
    assert_eq!(event_label(&contexts, true)?.1, 87_654_321);

    let mut broken = source.clone();
    broken.answer = "왼쪽입니다. [event:87654321]".into();
    assert!(event_episode(&broken, true).is_err(), "gold extraction is not a resolver");
    for input in [
        format!("장치0 구역1 {EVENT_TASK}"),
        format!("장치2 구역0 {EVENT_TASK}"),
        format!("장치0 구역0 {EVENT_TASK} "),
        format!("장치0  구역0 {EVENT_TASK}"),
        format!(" 장치0 구역0 {EVENT_TASK}"),
        format!("장치0 구역0 {EVENT_TASK}{EVENT_TASK}"),
    ] {
        let mut q = id.request.clone(); q.input = input;
        assert!(event_label(&q, true).is_err(), "malformed or unmatched query: {}", q.input);
    }
    let mut missing = id.request.clone();
    missing.evidence.items[0].version_status = "superseded".into();
    assert!(event_label(&missing, true).is_err());
    let mut ambiguous = id.request.clone();
    ambiguous.evidence.items[1].original_excerpt = "장치0의 구역0 값은 오른쪽이다.".into();
    assert!(event_label(&ambiguous, true).is_err());
    let mut duplicate = id.request.clone();
    duplicate.evidence.items[1].event_id = duplicate.evidence.items[0].event_id;
    assert!(event_label(&duplicate, true).is_err());
    for (entity, context) in [("", "구역0"), ("장치0 추가", "구역0"), ("장치0", "구역0 추가")] {
        let mut q = id.request.clone();
        q.input = format!("{entity} {context} {EVENT_TASK}");
        q.evidence.items[0].original_excerpt = format!("{entity}의 {context} 값은 왼쪽이다.");
        assert!(event_label(&q, true).is_err(), "ambiguous entity/context grammar");
    }

    let tok = ByteBpe::train(&[b"independent native descriptor".to_vec()], &"a".repeat(64), 264)?;
    let config = Config::tiny(tok.vocab_size());
    let train = TrainConfig { lr: 3e-5, warmup: 0, microbatch: 8, accumulation: 1,
        seq_len: 32, budget_start_step: 14_848, max_steps: 14_976,
        max_tokens: 1_000_000, ..Default::default() };
    let mut state = fixture_state(&train, &tok, 14_848); state.sampler_state = 0;
    let legacy = OptimizerProtocol::new(&config, false, "a".repeat(64), 14_336)?;
    assert!(!binary::record!(&legacy).as_object().unwrap().contains_key("study_start_step"));
    let mut p = legacy.clone(); p.version = 2; p.family = "ADAMW-INHERITED-V1".into();
    p.local_step = 512; p.study_start_step = Some(14_848);
    p.validate(&config, &state)?;
    let roundtrip: OptimizerProtocol = binary::from_slice(&binary::to_vec(&p)?)?;
    assert_eq!(roundtrip, p);
    assert!(p.roles.values().all(|v| !*v));
    assert_eq!(p.roles.keys().filter(|name| name.as_str() == "embedding").count(), 1);
    for kind in 0..10 {
        let mut q = p.clone();
        match kind {
            0 => q.version = 1,
            1 => q.family = "ADAMW-FRESH-V1".into(),
            2 => q.family = "ADAMW-INHERITED-V2".into(),
            3 => q.study_start_step = None,
            4 => q.study_start_step = Some(14_336),
            5 => q.study_start_step = Some(14_849),
            6 => q.parent_step = 14_335,
            7 => q.local_step = 0,
            8 => { q.roles.insert("embedding".into(), true); },
            _ => q.runtime_digest = "x".repeat(64),
        }
        assert!(q.validate(&config, &state).is_err(), "invalid native descriptor {kind}");
    }
    for kind in 0..5 {
        let mut s = state.clone();
        match kind {
            0 => s.config.budget_start_step = 14_336,
            1 => s.sampler_state = 512,
            2 => s.step = 14_849,
            3 => s.resume_binding.as_mut().unwrap().execution = 0,
            _ => s.resume_binding.as_mut().unwrap().family = 1,
        }
        assert!(p.validate(&config, &s).is_err(), "invalid native state {kind}");
    }
    for cursor in [1usize, 128] {
        let mut q = p.clone(); q.local_step = 512 + cursor;
        let mut s = state.clone(); s.step = 14_848 + cursor; s.sampler_state = cursor as u64;
        q.validate(&config, &s)?;
    }
    println!("INDEPENDENT_REQUEST_CLOCK boundaries PASS; model_construct0 forward0 backward0 optimizer0 generation0 teacher0");
    Ok(())
}
// Independently authored fixture: production strict scorer -> production gate.
// The rows below are explicitly synthetic, never recorded model outputs.
#[test]
#[ignore = "requires the prepared event study; synthetic output rows, model calls0"]
fn event_review_strict_scorer_gate() -> Result<()> {
    let root = PathBuf::from(std::env::var("R3_EVENT_PREPARATION")
        .map_err(|_| bad("explicit prepared event study required"))?);
    let s: Study = read_confirmed(&root.join("plan.r3b"))?;
    let corpus = event_inputs(&s, 1)?;
    let (_, _, dm) = inputs(&s)?;
    let tok = scoring_tokenizer(&s)?;
    // Fixed tokenizer, no model call: boundary tokens go through the exact
    // preparation length validator and are never truncated to fit.
    let gold=tok.encode(corpus.validation[3072].answer.as_bytes())?;
    for total in [255usize,256,257]{
        let wanted=total-gold.len()-1;
        let prompt=tok.encode(&vec![b'~';wanted])?;
        assert_eq!(prompt.len(),wanted);let before=prompt.clone();
        assert_eq!(event_length(prompt.len(),gold.len(),32).is_ok(),total<=256);
        assert_eq!(prompt,before);
    }
    assert!(event_length(1,gold.len(),8).is_err());
    let es = &corpus.validation[3072..3264];
    let ms = &dm[3072..3264];
    let make = |e: &Episode, bytes: &[u8], eos: bool| -> Result<binary::Value> {
        let tokens = tok.encode(bytes)?;
        let mut raw = tokens.clone(); if eos { raw.push(EOS); }
        let decoded = tok.decode(&tokens);
        let actual = decoded.as_ref().ok().cloned();
        let error = decoded.as_ref().err().map(ToString::to_string);
        Ok(binary::record!({
            "synthetic_fixture": true, "row_version": 2,
            "id": e.id, "expected": e.answer, "question": e.request.input,
            "request_digest": digest(&e.request)?, "generated_evidence": e.request.evidence,
            "generation_started": true, "generation_completed": true,
            "raw_tokens": raw, "raw_generated_count": raw.len(), "actual": actual,
            "error": error, "generation_error": null, "decode_error": error,
            "error_class": if error.is_some() { Some("strict_utf8") } else { None },
            "finish_reason": if eos { "stop" } else { "length" },
            "generation": { "tokens": tokens, "generated": raw.len(), "finish": if eos { "stop" } else { "length" } },
            "exact_match": eos && actual.as_deref() == Some(e.answer.as_str()) && error.is_none()
        }))
    };
    let pristine = es.iter().map(|e| make(e, e.answer.as_bytes(), true)).collect::<Result<Vec<_>>>()?;
    let good = event_score("ID_ONLY-word", es, ms, &pristine, &tok)?;
    assert!(event_gate(&good));
    assert_eq!(good["id_exact"], 192); assert_eq!(good["joint"]["query_both"], 96);
    assert_eq!(good["value_swap_invariance_both"], 96); assert_eq!(good["joint"]["all4"], 48);
    assert_eq!(good["digit_correct"], binary::record!(vec![192; 8]));
    let other = es[0].request.evidence.items.iter().map(|r| r.event_id.to_string())
        .find(|id| id != &es[0].answer).unwrap();
    let outside = (10_000_000..=99_999_999i64).find(|id|
        es[0].request.evidence.items.iter().all(|r| r.event_id != *id)).unwrap().to_string();
    let cases = vec![
        ("other", other.into_bytes(), true, "other_provided_id", true),
        ("outside", outside.into_bytes(), true, "valid_outside_id", false),
        ("space", format!(" {}", es[0].answer).into_bytes(), true, "parse_failure_rows", false),
        ("seven", es[0].answer.as_bytes()[..7].to_vec(), true, "parse_failure_rows", false),
        ("nine", format!("{}0", es[0].answer).into_bytes(), true, "parse_failure_rows", false),
        ("no_eos", es[0].answer.as_bytes().to_vec(), false, "parse_failure_rows", false),
        ("invalid_utf8", vec![0x80], true, "parse_failure_rows", false),
    ];
    for (name, bytes, eos, class, accepted) in cases {
        let mut rows = pristine.clone(); rows[0] = make(&es[0], &bytes, eos)?;
        let score = event_score("ID_ONLY-word", es, ms, &rows, &tok)?;
        assert_eq!(score["id_exact"], 191, "{name}"); assert_eq!(score[class], 1, "{name}");
        assert_eq!(event_gate(&score), accepted, "actual gate {name}");
        assert_eq!(score["joint"]["query_both"], 95); assert_eq!(score["joint"]["all4"], 47);
        if name == "no_eos" || name == "invalid_utf8" { assert_eq!(score["joint"]["errors"], 1); }
        if name == "seven" { assert_eq!(score["digit_correct"][7], 191, "absent eighth digit cannot be correct"); }
    }
    // Lying about EOS or repaired/trimmed text is an integrity rejection, not a score.
    let mut rows = pristine.clone();
    let ids: Vec<u32> = binary::from_value(rows[0]["generation"]["tokens"].clone())?;
    rows[0]["raw_tokens"] = binary::record!(ids);
    assert!(event_score("ID_ONLY-word", es, ms, &rows, &tok).is_err());
    let mut rows = pristine.clone(); rows[0] = make(&es[0], format!(" {}", es[0].answer).as_bytes(), true)?;
    rows[0]["actual"] = binary::record!(es[0].answer);
    assert!(event_score("ID_ONLY-word", es, ms, &rows, &tok).is_err());
    println!("INDEPENDENT_SCORER_GATE valid8/other/outside/space/seven/nine/noEOS/invalidUTF8 PASS; synthetic_only optimizer0 backward0 generation0 teacher0");
    Ok(())
}
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
    #[ignore="RETURNED decode quality vs runtime; actual collector/scorer/guard, model calls0"]
    fn returned_decode_quality_guard()->Result<()> {
        let root=PathBuf::from(std::env::var("R3_MUON_PREPARATION").map_err(|_|bad("explicit prepared study required"))?);
        let mut s:Study=read_confirmed(&root.join("plan.r3b"))?;
        let(c,_,dm)=inputs(&s)?;let es=&c.validation[..64];let ms=&dm[..64];
        let l=checkpoint::load(&s.parent,Device::Cpu,false)?;
        let dir=std::env::temp_dir().join(format!("replica-muon-output-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&dir)?;
        s.root=dir.clone();std::fs::create_dir(dir.join("A"))?;
        let mut rows=vec![];
        for e in es {
            let tokens=l.tokenizer.encode(e.answer.as_bytes())?;let mut raw=tokens.clone();raw.push(EOS);
            rows.push(binary::record!({"row_version":2,"id":e.id,"expected":e.answer,"question":e.request.input,"generated_evidence":e.request.evidence,"raw_tokens":raw,"actual":e.answer,"error":null,"generation_error":null,"decode_error":null,"finish_reason":"stop","generation_completed":true,"generation":{"tokens":tokens,"generated":raw.len(),"finish":"stop"},"exact_match":true}));
        }
        let valid=scored("value",es,ms,&rows,&l.tokenizer)?;
        let invalid=l.tokenizer.encode(&[0x80])?;let mut raw=invalid.clone();raw.push(EOS);
        let error=l.tokenizer.decode(&invalid).unwrap_err().to_string();
        rows[0]["raw_tokens"]=binary::record!(raw);rows[0]["actual"]=binary::Value::Null;
        rows[0]["generation"]=binary::record!({"tokens":invalid,"generated":raw.len(),"finish":"stop"});
        rows[0]["error"]=binary::record!(error);rows[0]["decode_error"]=binary::record!(error);
        rows[0]["error_class"]=binary::record!("strict_utf8");rows[0]["exact_match"]=binary::record!(false);
        verify_output_result(&rows[0],&l.tokenizer)?;
        for i in [4,8,12] {let ids=rows[i]["generation"]["tokens"].clone();rows[i]["raw_tokens"]=ids.clone();rows[i]["finish_reason"]=binary::record!("length");rows[i]["generation"]["finish"]=binary::record!("length");rows[i]["generation"]["generated"]=binary::record!(ids.as_array().unwrap().len());rows[i]["exact_match"]=binary::record!(false);}
        let binding=binary::record!({"synthetic_fixture":true,"cases":digest(&es)?,"call_protocol":1});
        let ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(10),u64::MAX)?;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join("output.r3rows"))?;append_row(&mut f,&binding)?;
        for(i,row)in rows.iter_mut().enumerate(){let attempt=prepare_call(&dir,"output","generation",&binding,&es[i],i)?;row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());append_row(&mut f,row)?;resolve_call(&attempt,Some(row),&ctl)?;}
        let hash=file_hash(&dir.join("output.r3rows"))?;
        let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;zero.set_call_limits(0,0);
        let reused=generated(&l,&dir,"output",es,&binding,&mut zero)?;zero.seal_completed_no_call()?;
        assert_eq!(reused,rows);assert_eq!(file_hash(&dir.join("output.r3rows"))?,hash);assert_eq!(zero.receipt()["generation_calls"],0);
        let score=scored("value",es,ms,&reused,&l.tokenizer)?;
        assert_eq!(score["joint"]["errors"],4);assert_eq!(score["joint"]["eos"],61);assert_eq!(score["joint"]["full"],60);
        let p=Progress{local:32,native:s.parent.clone(),physical:s.parent_hash.clone(),stop:None,evaluated:0,fit:false};
        let scores=BTreeMap::from([("value".into(),score),("citation".into(),valid.clone()),("S1Q1".into(),valid)]);
        assert_eq!(decision(&s,0,&p,&scores)?,Some("SEVERE_RETENTION".into()));
        for count in [0usize,1] {
            let label=format!("timeout{count}");let mut row=rows[1].clone();let ids=l.tokenizer.encode(es[1].answer.as_bytes())?;
            row["raw_tokens"]=binary::record!(&ids[..count]);row["raw_generated_count"]=binary::record!(count);
            row["actual"]=binary::Value::Null;row["generation"]=binary::Value::Null;
            row["generation_started"]=binary::record!(true);row["generation_completed"]=binary::record!(false);
            row["error"]=binary::record!("model: native generation timeout");row["generation_error"]=row["error"].clone();
            row["error_class"]=binary::record!("timeout");row["finish_reason"]=binary::record!("timeout");row["exact_match"]=binary::record!(false);
            row["timeout_cap_source"]=binary::record!("command");row["command_stop"]=binary::record!("TIME_BUDGET");row["interruption"]=row["command_stop"].clone();
            row["effective_timeout_ms"]=binary::record!(1);row["original_timeout_ms"]=binary::record!(1000);
            verify_output_result(&row,&l.tokenizer)?;
            for field in ["timeout_cap_source","command_stop","diagnostic_stop_error"] {let mut broken=row.clone();broken[field]=binary::record!("not a clean command timeout");assert!(verify_output_result(&broken,&l.tokenizer).is_err());}
            let attempt=prepare_call(&dir,&label,"generation",&binding,&es[1],0)?;row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());
            let path=dir.join(format!("{label}.r3rows"));let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
            append_row(&mut f,&binding)?;append_row(&mut f,&row)?;resolve_call(&attempt,Some(&row),&ctl)?;let hash=file_hash(&path)?;
            let mut zero=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::ZERO,u64::MAX)?;zero.set_call_limits(0,0);
            assert_eq!(generated(&l,&dir,&label,&es[1..2],&binding,&mut zero)?,vec![row]);zero.seal_completed_no_call()?;
            assert_eq!(file_hash(&path)?,hash);assert_eq!(zero.receipt()["generation_calls"],0);
        }
        for field in ["generation_completed","generation_error","decode_error","diagnostic_stop_error","command_stop"]{
            let mut broken=rows[0].clone();broken[field]=if field=="generation_completed"{binary::record!(false)}else if field=="decode_error"{binary::Value::Null}else{binary::record!("runtime failure")};
            assert!(verify_output_result(&broken,&l.tokenizer).is_err(),"{field}");
        }
        let mut cancelled=RunControl::new(std::sync::Arc::new(AtomicBool::new(true)),std::time::Duration::from_secs(1),u64::MAX)?;
        assert!(cancelled.seal_completed_no_call().is_err());
        println!("OUTPUT_GUARD strictUTF8/EOS1+length3 => severe4; raw unchanged; runtime/cancel blocked; optimizer0 generation0 teacher0 fixture={}",dir.display());Ok(())
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
            assert_eq!(h[0].arms.iter().map(|p|&p.physical).collect::<Vec<_>>(),last.arms.iter().map(|p|&p.physical).collect::<Vec<_>>());return Ok(());
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
            let root=s.root.join(s.arms()[arm]);std::fs::create_dir(&root)?;
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
                let binding=binary::record!({"policy":digest(&s)?,"arm":s.arms()[arm],"model":l.model.weights_content_id()?,"local":p.local,"absolute":s.parent_step+p.local,"runtime":s.runtime,"cases":digest(&episodes)?,"metadata":digest(&meta)?,"tokenizer":s.tokenizer,"planned":episodes.len(),"call_protocol":1});
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
        let arms=ps;let control=binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]});
        publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?}))?;
        publish_confirmed(&s.root.join("segment-000-finished.r3b"),&Segment{policy:digest(&s)?,previous:None,phase:8,arms,success:false,resume:true,control,error:Some("TIME_BUDGET".into())})?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::final_evaluation_only_process","--ignored","--nocapture","--test-threads=1"]).env("R3_MUON_FINALIZE_CHILD",&s.root).status()?;assert!(status.success());
        println!("ACTUAL_CALLER exhausted7200 fresh_process finalfit/score/saved_native unchanged; fixture_rows_only; optimizer0 generation0 teacher0 path={}",s.root.display());Ok(())
    }
}
