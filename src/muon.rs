//! The fixed Metal F32 optimizer comparison; training-only, no inference routing.
use super::*;
use neural::{Backend, RuntimeProfile, checkpoint::OptimizerProtocol};
use recovery::{ObservedCall,RunControl};

const CONTRACT:&str="R3-METAL-F32-MUON-QUALITY-1.0";
const ARMS:[&str;2]=["A","M"];
const EVENT_CONTRACT:&str="R3-SELECTED-EVENT-ID-PROTOCOL-1.1";
const EVENT_SYSTEM:&str="제공된 기록과 질문만으로 답하세요. 질문에서 지정한 출력 형식만 사용하세요. 기록에 없는 정보를 만들지 마세요. 근거가 없거나 모호하면 구별해서 유보하세요. 순서만으로 원인을 단정하지 마세요.";
const EVENT_TASK:&str="유효한 현재 기록의 사건 번호만 8자리 숫자로 답하라.";
#[path="muon_diagnosis.rs"]
mod diagnosis;
#[derive(Subcommand)]
pub enum Action {
    EventPrepare { #[arg(long)] diagnosis:PathBuf, #[arg(long)] audit:PathBuf, #[arg(long)] output:PathBuf },
    /// Separately authorized, read-only endpoint observations; never resumes training.
    Diagnose { #[command(subcommand)] command:diagnosis::Action },
    Prepare { #[arg(long)] parent:PathBuf, #[arg(long)] word_root:PathBuf, #[arg(long)] output:PathBuf },
    Admit { #[arg(long)] root:PathBuf, #[arg(long)] review:PathBuf },
    Baseline { #[arg(long)] root:PathBuf },
    Train { #[arg(long)] root:PathBuf },
    Report { #[arg(long)] root:PathBuf },
    Review { #[arg(long)] root:PathBuf, #[arg(long,value_parser=["A","M","F","I"])] arm:String },
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
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct EventProtocol {
    optimizer:OptimizerProtocol, original:PathBuf, original_hash:String,
    corpus:[PathBuf;2], corpus_hashes:[String;2], refs:BTreeMap<PathBuf,String>,
    retention:BTreeMap<String,binary::Value>, parent_exposure:Vec<usize>,
}
impl Study {
    fn arms(&self)->[&'static str;2]{if self.event.is_some(){["F","I"]}else{ARMS}}
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
    verified_corpus(&e.corpus[arm],&e.corpus_hashes[arm])
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
        baseline_hash:file_hash(&baseline_raw)?,baseline_raw,max_updates:1024,generation_cap:6400,teacher_cap:6400,active_cap:7200.,segment_cap:900.,bytes_cap:1610612736,event:None};
    for arm in ARMS {std::fs::create_dir(root.join(arm))?;}
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    let roles=[OptimizerProtocol::new(&l.model.config,false,digest(&s.runtime)?,st.step)?,OptimizerProtocol::new(&l.model.config,true,digest(&s.runtime)?,st.step)?];
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"policy":digest(&s)?,"parent":s.parent_hash,"content":s.parent_content,"parent_adam_preserved":s.parent_adam,"roles":roles,"fresh_state":"all zeros on allocation; no inherited moments","costs":s.costs,"optimizer_calls":0,"generation_calls":0,"phase_order":[["A",32],["M",32],["M",128],["A",128],["A",512],["M",512],["M",1024],["A",1024]]}))?;
    println!("MUON_PREPARED policy={} parent={} input={input} target={target} padding={padding} new_optimizer0 A_PENDING",digest(&s)?,s.parent_hash);Ok(())
}

fn load_study(root:&Path,execute:bool)->Result<Study>{
    let s:Study=read_confirmed(&root.join("plan.r3b"))?;
    if s.contract!=if s.event.is_some(){EVENT_CONTRACT}else{CONTRACT}||s.root!=root.canonicalize()?{return Err(bad("study root/contract binding"));}
    inputs(&s)?;
    if s.event.is_some(){event_inputs(&s,0)?;event_inputs(&s,1)?;}
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
        ||(s.event.is_some()&&r["boundaries"]!=binary::record!(["request_labels","strict_scorer_gate","tape_cost","inherited_native_process","actual_teacher_modes","final_no_call"]))
        ||r["active_seconds"].as_f64().is_none_or(|v|!v.is_finite()||v<0.||v>=s.active_cap) {
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
    let labels=if s.event.is_some(){["baseline","review-F","review-I"]}else{["baseline","review-A","review-M"]};
    let observations=labels.into_iter().map(|label|observation_history(s,label)).collect::<Result<Vec<_>>>()?;
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
    b.policy=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&(s,s.arms()[arm]))?);b.provenance=b.policy;
    b.train_order=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&s.tape)?);b.framing=neural::Framing::QuestionEvidence.digest();Ok(b)
}
fn load_arm(s:&Study,arm:usize,p:&Progress,device:&Device)->Result<(checkpoint::Loaded,Optimizer)> {
    let mut l=checkpoint::load(&p.native,device.clone(),true)?;let mut state=l.manifest.training.clone().ok_or_else(||bad("missing native training state"))?;
    let corpus=if s.event.is_some(){event_inputs(s,arm)?}else{verified_corpus(&s.word_root.join("corpus.r3cor"),&s.corpus_hash)?};
    let opt=if p.local==0 {
        if p.physical!=s.parent_hash||state.step!=s.parent_step||l.model.weights_content_id()?!=s.parent_content||optimizer_hash(&l.optimizer)?!=s.parent_adam{return Err(bad("fresh parent weights/Adam provenance"));}
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
    let logits=l.model.forward(&b.input,Some(&b.valid))?;let(_,loss,targets,examples)=response_objective(&logits,b,1.,true)?;
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
    Ok(binary::record!({"local":p.local,"model_step":st.step,"optimizer_local":o.protocol.local_step,"rows":s.tape[p.local-1],"input":b.tokens,"target":targets,"padding":b.input.elem_count()-b.tokens,"examples":examples,"answer_ce":value,"lr":s.config.lr,"stats":stats,"forward_backward_seconds":fb,"optimizer_seconds":opt,"step_seconds":start.elapsed().as_secs_f64(),"arm":s.arms()[arm]}))
}
fn train(s:&Study)->Result<()> {
    let baseline:binary::Value=read_confirmed(&s.root.join("baseline-finished.r3b"))?;if baseline["success"]!=true{return Err(bad("baseline not accepted"));}
    let h=history(s)?;if h.last().is_some_and(|s|!s.resume){return Err(bad("closed pair; no resume"));}
    if s.event.is_some()&&baseline["extra"]["id_sufficient"]==true{return Err(bad("BASELINE_ID_CONTRACT_SUFFICIENT; no learning"));}
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
    println!("MUON_SEGMENT index={index} phase={phase} local={:?} resume={} stop={:?} active={} new_bytes={}",seg.arms.each_ref().map(|p|p.local),seg.resume,seg.arms.each_ref().map(|p|&p.stop),seg.control["elapsed_seconds"],owned_bytes(&s.root)?);
    if pure_time{Ok(())}else{result}
}

fn report(s:&Study)->Result<()> {
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
fn review(s:&Study,arm:usize)->Result<()> {
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
        Action::Diagnose{command}=>diagnosis::run(command),
        Action::EventPrepare{diagnosis,audit,output}=>event_prepare(&diagnosis,&audit,&output),
        Action::Prepare{parent,word_root,output}=>prepare(&parent,&word_root,&output),
        Action::Admit{root,review}=>admit(&root,&review),
        Action::Baseline{root}=>baseline(&load_study(&root,true)?),
        Action::Train{root}=>train(&load_study(&root,true)?),
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
    use super::*;
    fn event_fixture() -> Result<(Study,PathBuf)> {
        let prepared=PathBuf::from(std::env::var("R3_EVENT_PREPARATION").map_err(|_|bad("explicit event preparation required"))?);
        let mut s:Study=read_confirmed(&prepared.join("plan.r3b"))?;
        let root=std::env::temp_dir().join(format!("replica-event-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&root)?;
        let tok=scoring_tokenizer(&s)?;let mut cfg=Config::tiny(tok.vocab_size());cfg.context=2048;cfg.profile="NATIVE_TRPP_EXPERIMENTAL_V1".into();
        let device=Backend::Metal0.open()?;s.runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
        let model=Transformer::init(cfg,20260924,device)?;
        let mut op=Optimizer::fresh(&model.config,&model.vars,false,digest(&s.runtime)?,14336)?;
        // Explicit synthetic inherited nonzero moments; no hidden512 updates.
        for (name,t)in &mut op.tensors{*t=Tensor::full(if name.starts_with("adam.m."){0.0001f32}else{0.0002},t.dims(),t.device())?;}
        op.protocol.local_step=512;
        let mut manifest=checkpoint::initialized(&model,&tok,20260924,"e".repeat(64))?;
        let mut config=s.config.clone();config.budget_start_step=14336;config.max_steps=15360;
        let mut state=fixture_state(&config,&tok,14848);state.sampler_state=512;state.initial_weight_hash=manifest.initial_weight_hash.clone();state.consumed_tokens=config.budget_start_tokens;
        manifest.training=Some(state);manifest.optimizer_protocol=Some(op.protocol.clone());manifest.status="DIAGNOSTIC_COMPLETE".into();
        let path=root.join("tiny-parent.r3m");checkpoint::save(&path,&model,&tok,manifest,&op.tensors)?;
        s.parent=path;s.parent_hash=file_hash(&s.parent)?;s.parent_content=model.weights_content_id()?;s.parent_adam=optimizer_hash(&op.tensors)?;s.event.as_mut().unwrap().optimizer=op.protocol;
        Ok((s,root))
    }
    fn event_test_root(s:&Study,root:&Path)->Result<Study>{let mut s=s.clone();std::fs::create_dir(root)?;s.root=root.canonicalize()?;
        for arm in s.arms(){std::fs::create_dir(s.root.join(arm))?;}publish_confirmed(&s.root.join("plan.r3b"),&s)?;Ok(s)}
    fn event_tiny_steps(s:&Study,arm:usize,p:&mut Progress,updates:usize,index:usize)->Result<()> {
        let device=Backend::Metal0.open()?;let(l,mut o)=load_arm(s,arm,p,&device)?;let mut l=l;
        let ss=samples_with_framing(&event_inputs(s,arm)?.train,&l.tokenizer,256,neural::Framing::QuestionEvidence)?;
        let mut ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(120),u64::MAX)?;
        for _ in 0..updates{
            let b=batch(&ss,&s.tape[p.local],&device)?;let entry=s.root.join(s.arms()[arm]).join(format!("fixture-{}-entered.r3b",p.local+1));
            publish_confirmed(&entry,&binary::record!({"fixture":true,"cursor":p.local,"tape":s.tape[p.local]}))?;
            let mut entered=false;let trace=train_update(s,arm,&mut l,&mut o,p,&b,&mut ctl,&mut entered,&entry,Instant::now())?;
            assert!(!entered);assert_eq!(trace["optimizer_local"],512+p.local);assert_eq!(trace["model_step"],14848+p.local);assert!(o.protocol.roles.values().all(|v|!*v));
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
            assert_eq!(h[0].arms.each_ref().map(|p|&p.physical),last.arms.each_ref().map(|p|&p.physical));return Ok(());
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
        let path=dir.join(format!("{label}{}.r3rows",if teacher{"-teachers"}else{""}));
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;append_row(&mut f,b)?;
        let ctl=RunControl::new(std::sync::Arc::new(AtomicBool::new(false)),std::time::Duration::from_secs(30),u64::MAX)?;
        for(i,e)in es.iter().enumerate(){
            let attempt=prepare_call(dir,label,if teacher{"teacher"}else{"generation"},b,e,i)?;
            let output=tok.encode(e.answer.as_bytes())?;let mut ids=output.clone();ids.push(EOS);
            let samples=samples_with_framing(std::slice::from_ref(e),tok,s.config.seq_len,neural::Framing::QuestionEvidence)?;let sample=&samples[0];let n=sample.response_start;
            let row=if teacher{binary::record!({"ordinal":i,"id":e.id,"case":digest(e)?,"teacher":{"mean_nll":1.,"target_tokens_including_eos":ids.len(),"training_prompt_matches_generation":true,"answer_tokenizer_roundtrip":true,"target_token_observation":{"gold":ids,"argmax":ids,"nll":vec![1.;ids.len()]},"native_forward":{"model_device":s.runtime.actual_device,"input_device":s.runtime.actual_device,"logits_device":s.runtime.actual_device,"input_dtype":"U32","logits_dtype":"F32","logits_finite":true,"input_shape":[1,sample.tokens.len()-1],"prompt_tokens":n,"prompt_digest":digest(&&sample.tokens[..n])?,"input_tokens_digest":digest(&&sample.tokens[..sample.tokens.len()-1])?,"logits_shape":[sample.tokens.len()-n,tok.vocab_size()]}},"attempt":attempt.file_name().unwrap().to_string_lossy()})}
                else{binary::record!({"row_version":2,"id":e.id,"expected":e.answer,"question":e.request.input,"generated_evidence":e.request.evidence,"raw_tokens":ids,"actual":e.answer,"error":null,"finish_reason":"stop","generation_completed":true,"generation":{"tokens":output,"generated":ids.len(),"finish":"stop"},"exact_match":true,"attempt":attempt.file_name().unwrap().to_string_lossy()})};
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
        let arms:[Progress;2]=ps.try_into().ok().unwrap();let control=binary::record!({"elapsed_seconds":0.,"generation_calls":0,"teacher_calls":0,"observed_conditions":["TIME_BUDGET"]});
        publish_confirmed(&s.root.join("segment-000-started.r3b"),&binary::record!({"policy":digest(&s)?}))?;
        publish_confirmed(&s.root.join("segment-000-finished.r3b"),&Segment{policy:digest(&s)?,previous:None,phase:8,arms,success:false,resume:true,control,error:Some("TIME_BUDGET".into())})?;
        let status=std::process::Command::new(std::env::current_exe()?).args(["--exact","training::fresh::muon::tests::final_evaluation_only_process","--ignored","--nocapture","--test-threads=1"]).env("R3_MUON_FINALIZE_CHILD",&s.root).status()?;assert!(status.success());
        println!("ACTUAL_CALLER exhausted7200 fresh_process finalfit/score/saved_native unchanged; fixture_rows_only; optimizer0 generation0 teacher0 path={}",s.root.display());Ok(())
    }
}
