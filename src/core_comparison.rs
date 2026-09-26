//! Bounded fresh FULL-corpus comparison. The two cores never share recurrent state.
use super::*;
use neural::{artifact::{self,ComparisonCore,ComparisonState},gru,Backend,RuntimeProfile,Framing};
use recovery::{RunControl,ObservedCall};
use std::{sync::Arc,time::Duration};
const CONTRACT:&str="R3-QUALITY-GRU-EXECUTION-1.0";
const ARMS:[&str;2]=["TRPP","GRU"];
const SEED:u64=20260925;
#[path = "core_binding_recovery.rs"]
mod binding_recovery;
#[derive(Subcommand)]
pub enum Action {
    /// Parent-bound TR++ contrast study; original core comparison stays closed.
    BindingRecovery { #[command(subcommand)] action:binding_recovery::Action },
    Prepare {#[arg(long)] word_root:PathBuf,#[arg(long)] output:PathBuf,#[arg(long)] audit_only:bool},
    Revise {#[arg(long)] previous:PathBuf,#[arg(long)] output:PathBuf,#[arg(long)] failed_review:PathBuf},
    Admit {#[arg(long)] root:PathBuf,#[arg(long)] review:PathBuf},
    Execute {#[arg(long)] root:PathBuf,#[arg(long,value_parser=["TRPP","GRU"])] core:String,
        #[arg(long,value_parser=["baseline","train","evaluate","finalize","review"])] phase:String,
        #[arg(long,default_value_t=3072)] until:usize},
    Report {#[arg(long)] root:PathBuf},
    VerifyTiny {#[arg(long)] continuous:PathBuf,#[arg(long)] split:PathBuf},
    /// Bounded numerical fixture, not a SMALL learning run.
    Tiny {#[arg(long)] output:PathBuf,#[arg(long,value_parser=["TRPP","GRU"])] core:String,
        #[arg(long,default_value_t=2)] until:usize,#[arg(long)] resume:bool,#[arg(long)] cancel_after_commit:bool},
}
enum Core {Tr(Transformer),Gru(gru::Gru)}
impl Core {
    fn vars(&self)->&BTreeMap<String,Var>{match self{Self::Tr(m)=>&m.vars,Self::Gru(m)=>&m.vars}}
    fn device(&self)->&Device{match self{Self::Tr(m)=>&m.device,Self::Gru(m)=>&m.device}}
    fn descriptor(&self)->ComparisonCore{match self{Self::Tr(m)=>ComparisonCore::Trpp(m.config.clone()),Self::Gru(m)=>ComparisonCore::Gru(m.config.clone())}}
    fn forward(&self,b:&Batch)->Result<Tensor>{match self{Self::Tr(m)=>m.forward(&b.input,Some(&b.valid)),Self::Gru(m)=>m.forward(&b.input,Some(&b.valid))}}
    fn bind(&mut self,tok:&str)->Result<()>{match self{Self::Tr(m)=>m.bind_tokenizer(tok),Self::Gru(m)=>m.bind_tokenizer(tok)}}
    fn content(&mut self)->Result<String>{match self{Self::Tr(m)=>{m.refresh_identity()?;m.weights_content_id()},Self::Gru(m)=>{m.refresh_identity()?;m.weights_content_id()}}}
    fn load(desc:&ComparisonCore,vars:BTreeMap<String,Tensor>,device:Device)->Result<Self>{
        match desc{ComparisonCore::Trpp(c)=>Ok(Self::Tr(Transformer::from_tensors(c.clone(),vars,device)?)),
            ComparisonCore::Gru(c)=>Ok(Self::Gru(gru::Gru::from_tensors(c.clone(),vars,device)?))}
    }
    fn observe(&self,tok:&ByteBpe,e:&Episode,c:&mut RunControl)->ObservedCall<binary::Value>{
        let descriptor=self.descriptor();let context=match &descriptor{ComparisonCore::Trpp(c)=>c.context,ComparisonCore::Gru(c)=>c.context};
        recovery::observe_core_generation(tok,Framing::QuestionEvidence,context,&descriptor.id().unwrap_or_default(),e,&e.request,c,None,true,
            |p,n,t,cancel,scope,observe|match self{Self::Tr(m)=>m.generate_profiled(p,n,t,cancel,scope,observe),Self::Gru(m)=>m.generate_observed(p,n,t,cancel,scope,observe)})
    }
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    contract:String,root:PathBuf,artifact_root:PathBuf,source:String,runtime:RuntimeProfile,word_root:PathBuf,
    references:BTreeMap<PathBuf,String>,tokenizer:PathBuf,tokenizer_id:String,
    cores:[ComparisonCore;2],config:TrainConfig,tape:Vec<[usize;8]>,cache_key:[[u8;32];6],
    costs:binary::Value,old_qa:binary::Value,quality:binary::Value,
    #[serde(default,skip_serializing_if="Option::is_none")]
    revision:Option<InitialReference>,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct InitialReference {plan:PathBuf,plan_hash:String,cache:PathBuf,cache_hash:String,failed_review:PathBuf,review_hash:String,endpoints:[Endpoint;2]}
#[derive(Clone,Serialize,Deserialize,PartialEq)]
#[serde(deny_unknown_fields)]
struct Endpoint {state:ComparisonState,path:PathBuf,hash:String,content:String}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Segment {
    policy:String,core:String,phase:String,index:usize,endpoint:Endpoint,
    backwards:usize,generation:usize,generated_tokens:usize,seconds:f64,stop:String,
    trace:PathBuf,trace_hash:String,control:binary::Value,
}
fn source()->Result<String>{digest(&(source_digest()?,neural::hash(include_bytes!("core_comparison.rs")),neural::hash(include_bytes!("neural/gru.rs"))))}
fn config()->TrainConfig{TrainConfig{lr:3e-4,beta1:0.9,beta2:0.999,eps:1e-8,weight_decay:0.,clip:1.,warmup:128,
    max_steps:3072,max_tokens:20_000_000,microbatch:8,sample_group_size:1,accumulation:1,seq_len:256,validate_every:512,seed:SEED,
    first_target_weight:1.,curriculum_steps:0,budget_start_step:0,budget_start_tokens:0}}
fn state(s:&Study,i:usize)->Result<ComparisonState>{Ok(ComparisonState{schema:1,core:s.cores[i].clone(),policy:digest(s)?,tokenizer:s.tokenizer_id.clone(),framing:Framing::QuestionEvidence.digest(),runtime:s.runtime.clone(),objective:checkpoint::ANSWER_MEAN_FAMILY,optimizer:"FRESH_ADAMW_ALL_V1".into(),config:s.config.clone(),committed:0,adam_clock:0,input_tokens:0,target_tokens:0})}
fn key_hash(s:&str)->Result<[u8;32]>{let mut out=[0;32];if s.len()!=64{return Err(bad("cache digest"));}for(i,b)in out.iter_mut().enumerate(){*b=u8::from_str_radix(&s[2*i..2*i+2],16).map_err(|_|bad("cache hex"))?;}Ok(out)}
fn prepare(word_root:&Path,output:&Path,audit_only:bool)->Result<()> {
    let word_root=word_root.canonicalize()?;let p:Plan=read(&word_root.join("plan.r3b"))?;
    let corpus=verified_corpus(&word_root.join("corpus.r3cor"),&p.corpus)?;let(tm,dm,_)=verified_metadata(&word_root,&p)?;
    let tokenizer=word_root.join("tokenizer.r3b");let tok=ByteBpe::load(&tokenizer)?;
    if p.framing()!=Framing::QuestionEvidence||corpus.train.len()!=7680||corpus.validation.len()!=3456||tm.len()!=7680||dm.len()!=3456{return Err(bad("FULL native corpus dimensions/framing"));}
    let rows=&p.identifiable.as_ref().ok_or_else(||bad("word native tape absent"))?.rows;
    let tape=rows.get(p.origin_step()..p.origin_step()+3072).ok_or_else(||bad("word original cycle incomplete"))?.to_vec();
    let samples=samples_with_framing(&corpus.train,&tok,256,p.framing())?;
    let mut identities=vec![];let mut max_len=0;
    for(e,s)in corpus.train.iter().zip(&samples){
        let prompt=tok.prepare_with_framing(&e.request,p.framing(),2048,"comparison-audit")?;
        let answer=tok.encode(e.answer.as_bytes())?;
        if s.tokens.len()>256||tok.decode(&answer)?!=e.answer||prompt.provided.len()!=2||!prompt.excluded.is_empty()
            ||s.tokens[..s.response_start]!=prompt.token_ids||s.tokens[s.response_start..s.tokens.len()-1]!=answer||s.tokens.last()!=Some(&EOS){return Err(bad(&format!("FULL no-truncation framing/roundtrip {}",e.id)));}
        max_len=max_len.max(s.tokens.len());identities.push(digest(&(&s.tokens,s.response_start))?);
    }
    let mut exposures=vec![0usize;7680];let(mut input,mut targets)=(0u64,0u64);
    for row in &tape {if row[..4].iter().any(|&i|i>=6144)||row[4..].iter().any(|&i|!(6144..7680).contains(&i)){return Err(bad("FULL tape review4/word4"));}
        for &i in row{exposures[i]+=1;input+=(samples[i].tokens.len()-1)as u64;targets+=(samples[i].tokens.len()-samples[i].response_start)as u64;}}
    if exposures[6144..].iter().any(|&n|n!=8)||exposures[..6144].iter().sum::<usize>()!=12288{return Err(bad("FULL cycle exposures"));}
    let selection_path=p.fork.as_ref().ok_or_else(||bad("word fork"))?.study.join("selection.r3b");
    let selection:binary::Value=read(&selection_path)?;
    for transfer in [false,true]{identifiable::binding::citation::bridge_old_qa(&selection,transfer)?;}
    let mut refs=BTreeMap::new();for path in [word_root.join("plan.r3b"),word_root.join("corpus.r3cor"),word_root.join("metadata.r3b"),tokenizer.clone(),selection_path]{refs.insert(path.clone(),file_hash(&path)?);}
    if audit_only {publish_confirmed(output,&binary::record!({"references":refs,"tape":digest(&tape)?,"exact_token_identities":identities,"exposures":exposures,"max_train_length":max_len,"input":input,"target":targets,"model_calls":0}))?;
        println!("DATA_AUDIT train={} validation={} tape={} length={max_len} input={input} target={targets} model_calls=0",samples.len(),corpus.validation.len(),tape.len());return Ok(());}
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let tr=Transformer::init(Config::small(tok.vocab_size()),SEED,device.clone())?;
    let gr=gru::Gru::init(gru::Config::matched(tok.vocab_size()),SEED,&tr.vars["embedding"],device)?;
    let tp=tr.vars.values().map(|v|v.elem_count()).sum::<usize>();let gp=gr.vars.values().map(|v|v.elem_count()).sum::<usize>();
    if tp.abs_diff(gp)as f64/tp as f64>0.05{return Err(bad("parameter match exceeds preregistered five percent"));}
    let cores=[ComparisonCore::Trpp(tr.config.clone()),ComparisonCore::Gru(gr.config.clone())];
    let cache_key=[key_hash(&p.corpus)?,key_hash(&digest(&identities)?)?,key_hash(&digest(&tape)?)?,key_hash(&tok.semantic_id())?,Framing::QuestionEvidence.digest(),key_hash(&digest(&config())?)?];
    let artifact_root=output.parent().ok_or_else(||bad("comparison evidence root"))?.canonicalize()?;
    if !artifact_root.join("build-before.txt").is_file(){return Err(bad("comparison requires its scoped evidence root and build baseline"));}
    std::fs::create_dir(output)?;let root=output.canonicalize()?;
    let quality=binary::record!({"V512":[488,232,116],"VC512":[488,232,116],"S1Q1_512":[488,232,116],"citation_value_support_min":508,"outside_malformed":0,
        "word192":{"full":183,"query_both":88,"swap_both":88,"all4":44,"value":190,"support":190,"outside":0,"malformed":0},
        "fit192":"representative diagnostic; not full1536 fit gate","QA":"original bucket gates, diagnostic only","Goal1":false});
    let s=Study{contract:CONTRACT.into(),root:root.clone(),artifact_root,source:source()?,runtime,word_root,references:refs,tokenizer,tokenizer_id:tok.semantic_id(),cores,config:config(),tape,cache_key,
        costs:binary::record!({"input_per_arm":input,"target_per_arm":targets,"exposures":exposures,"prompt_target_identities":identities,"max_train_length":max_len,"parameters":[tp,gp]}),old_qa:selection,quality,revision:None};
    recovery::experiment_record::token_cache::comparison_cache(&root.join("samples.r3tok"),cache_key,256,tok.vocab_size(),&samples,true)?;
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    for(i,mut core)in [Core::Tr(tr),Core::Gru(gr)].into_iter().enumerate(){let dir=root.join(ARMS[i]);std::fs::create_dir(&dir)?;core.bind(&s.tokenizer_id)?;
        let adam=Adam::new(core.vars())?;let st=state(&s,i)?;let ep=save_endpoint(&dir,&mut core,&adam,st,"initial")?;publish_confirmed(&dir.join("initial.r3b"),&ep)?;}
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"policy":digest(&s)?,"source":s.source,"runtime":s.runtime,"spec":"SPEC_AVAILABLE_FROM_RECOVERED_HANDOVER",
        "spec_sha256":"c6af6422c544e98526295aabb7d6b72523d273b95d827eb148eb3275f77742be","equation":gru::EQUATION,
        "initialization":"TR original normal0.02, shared embedding; GRU name-seeded Xavier; bias0/norm1","costs":s.costs,"quality":s.quality,
        "optimizer":0,"generation":0,"teacher":0,"independent_A":"PENDING"}))?;
    println!("PREPARED parameters={tp}/{gp} length={max_len} input={input} target={targets} policy={} A_PENDING",digest(&s)?);Ok(())
}
#[derive(Default)]
struct InputProfile {reads:usize,hashes:usize,decodes:usize,reference_seconds:f64,
    corpus_seconds:f64,metadata_seconds:f64,tokenizer_seconds:f64,samples_seconds:f64,cache_seconds:f64}
fn inputs(s:&Study)->Result<(data::native::Corpus,Vec<Meta>,Vec<Meta>,ByteBpe,Vec<Sample>)>{
    Ok(inputs_profiled(s)?.0)
}
fn inputs_profiled(s:&Study)->Result<((data::native::Corpus,Vec<Meta>,Vec<Meta>,ByteBpe,Vec<Sample>),InputProfile)>{
    let mut profile=InputProfile::default();let started=Instant::now();
    let mut snapshots=BTreeMap::new();
    for(path,expected)in &s.references{
        let bytes=neural::read_bounded(path,128*1024*1024)?;profile.reads+=1;
        if neural::hash(&bytes)!=*expected{return Err(bad("frozen comparison input changed"));}
        profile.hashes+=1;snapshots.insert(path.clone(),bytes);
    }
    profile.reference_seconds=started.elapsed().as_secs_f64();
    let bytes=|path:&Path|snapshots.get(path).map(Vec::as_slice).ok_or_else(||bad("comparison reference missing"));
    let plan_path=s.word_root.join("plan.r3b");
    let p:Plan=binary::from_slice(bytes(&plan_path)?)?;profile.decodes+=1;
    let started=Instant::now();
    let c=data::native::decode(bytes(&s.word_root.join("corpus.r3cor"))?)?;profile.decodes+=1;
    if hex(&c.physical)!=p.corpus{return Err(bad("owned native corpus differs from frozen bytes"));}
    profile.corpus_seconds=started.elapsed().as_secs_f64();
    let started=Instant::now();
    let metadata=bytes(&s.word_root.join("metadata.r3b"))?;
    if neural::hash(metadata)!=p.metadata{return Err(bad("owned metadata differs from plan"));}
    profile.hashes+=1;
    let(tm,dm,_):(Vec<Meta>,Vec<Meta>,Vec<Meta>)=binary::from_slice(metadata)?;profile.decodes+=1;
    profile.metadata_seconds=started.elapsed().as_secs_f64();
    let started=Instant::now();
    let tok=ByteBpe::from_bytes(bytes(&s.tokenizer)?)?;profile.decodes+=1;
    if tok.semantic_id()!=s.tokenizer_id{return Err(bad("comparison tokenizer changed"));}
    profile.tokenizer_seconds=started.elapsed().as_secs_f64();
    let started=Instant::now();
    let ss=samples_with_framing(&c.train,&tok,256,Framing::QuestionEvidence)?;
    profile.samples_seconds=started.elapsed().as_secs_f64();
    let started=Instant::now();
    let cache=if let Some(r)=&s.revision{
        for(path,expected)in [(&r.plan,&r.plan_hash),(&r.failed_review,&r.review_hash)]{
            let bytes=neural::read_bounded(path,128*1024*1024)?;profile.reads+=1;
            if neural::hash(&bytes)!=*expected{return Err(bad("initial reference changed"));}
            profile.hashes+=1;
        }r.cache.clone()
    }else{s.root.join("samples.r3tok")};
    let cache_bytes=neural::read_bounded(&cache,128*1024*1024)?;profile.reads+=1;
    if let Some(r)=&s.revision {if neural::hash(&cache_bytes)!=r.cache_hash{return Err(bad("initial reference changed"));}
        profile.hashes+=1;}
    let ss=recovery::experiment_record::token_cache::comparison_cache_bytes(cache_bytes,s.cache_key,256,tok.vocab_size(),&ss)?;
    profile.decodes+=1;profile.cache_seconds=started.elapsed().as_secs_f64();
    Ok(((c,tm,dm,tok,ss),profile))
}
fn revise(previous:&Path,output:&Path,failed_review:&Path)->Result<()> {
    let old:Study=read_confirmed(&previous.join("plan.r3b"))?;let r:binary::Value=read(failed_review)?;
    if old.contract!=CONTRACT||old.root!=previous.canonicalize()?||old.config!=config()||old.revision.is_some()
        ||r["verdict"]!="FAIL"||r["policy"]!=digest(&old)?||r["source"]!=old.source{return Err(bad("only unused initial pair after bound A FAIL may be revised"));}
    inputs(&old)?;
    let initial=[read_confirmed::<Endpoint>(&old.root.join("TRPP/initial.r3b"))?,read_confirmed::<Endpoint>(&old.root.join("GRU/initial.r3b"))?];
    for(i,e)in initial.iter().enumerate(){if !history(&old,i)?.is_empty()||e.state.committed!=0||e.state.adam_clock!=0||e.state.policy!=digest(&old)?||e.state.core!=old.cores[i]{return Err(bad("revision cannot reopen a trained run"));}
        load_endpoint(e,Device::Cpu)?;}
    if output.parent().map(Path::canonicalize).transpose()?.as_ref()!=Some(&old.artifact_root){return Err(bad("revision evidence root"));}
    std::fs::create_dir(output)?;let mut s=old.clone();s.root=output.canonicalize()?;s.source=source()?;s.runtime=RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?;
    let plan=old.root.join("plan.r3b");let cache=old.root.join("samples.r3tok");
    s.revision=Some(InitialReference{plan_hash:file_hash(&plan)?,plan,cache_hash:file_hash(&cache)?,cache,failed_review:failed_review.canonicalize()?,review_hash:file_hash(failed_review)?,endpoints:initial.clone()});
    publish_confirmed(&s.root.join("plan.r3b"),&s)?;
    for(i,e)in initial.iter().enumerate(){let dir=s.root.join(ARMS[i]);std::fs::create_dir(&dir)?;publish_confirmed(&dir.join("initial.r3b"),e)?;}
    publish_confirmed(&s.root.join("preparation.r3b"),&binary::record!({"policy":digest(&s)?,"source":s.source,"runtime":s.runtime,"reused_initial":initial,"prior_plan":digest(&old)?,"costs":s.costs,"optimizer":0,"generation":0,"teacher":0,"new_model_bytes":0,"independent_A":"PENDING"}))?;
    println!("REVISED policy={} original_step0_reused=true new_model_bytes=0 optimizer=0 A_PENDING",digest(&s)?);Ok(())
}
fn initial_reference(s:&Study,arm:usize,ep:&Endpoint)->bool{
    s.revision.as_ref().is_some_and(|r|r.endpoints[arm]==*ep&&ep.state.committed==0&&ep.state.adam_clock==0&&ep.state.core==s.cores[arm])
}
fn study(root:&Path,execute:bool)->Result<Study>{let s:Study=read_confirmed(&root.join("plan.r3b"))?;
    if s.contract!=CONTRACT||s.root!=root.canonicalize()?||s.root.parent()!=Some(s.artifact_root.as_path())||s.source!=source()?||s.config!=config()||s.tape.len()!=3072{return Err(bad("comparison policy/source mismatch"));}
    if execute{let r:binary::Value=read_confirmed(&root.join("review-a.r3b"))?;let path=Path::new(r["path"].as_str().ok_or_else(||bad("independent A missing"))?);
        if r["policy"]!=digest(&s)?||r["report_hash"]!=file_hash(path)?||r["accepted"]!=true{return Err(bad("independent A binding"));}}
    Ok(s)
}
fn admit(root:&Path,review:&Path)->Result<()>{let s=study(root,false)?;let r:binary::Value=read(review)?;
    if r["contract"]!=CONTRACT||r["policy"]!=digest(&s)?||r["source"]!=s.source||r["runtime"]!=binary::record!(s.runtime)||r["verdict"]!="PASS"
        ||r["boundaries"]!=binary::record!(["reset_after_vjp","state_padding_reset","native_process_adam","full_inputs_tape","shared_loss_generation_scorer","budgets"]){return Err(bad("independent A incomplete"));}
    publish_confirmed(&root.join("review-a.r3b"),&binary::record!({"accepted":true,"policy":digest(&s)?,"path":review.canonicalize()?,"report_hash":file_hash(review)?}))
}
fn save_endpoint(dir:&Path,core:&mut Core,adam:&Adam,state:ComparisonState,label:&str)->Result<Endpoint>{
    let path=dir.join(format!("{label}-{}.r3model",state.committed));let content=core.content()?;
    artifact::save_comparison(&path,state.clone(),core.vars(),&adam.moments)?;
    Ok(Endpoint{state,path:path.clone(),hash:file_hash(&path)?,content})
}
fn load_endpoint(e:&Endpoint,device:Device)->Result<(Core,Adam)>{
    let bytes=neural::read_bounded(&e.path,512*1024*1024)?;
    if neural::hash(&bytes)!=e.hash{return Err(bad("endpoint physical hash"));}
    let(st,vars,moments)=artifact::load_comparison_bytes(&bytes,&e.state,&device)?;
    let mut core=Core::load(&st.core,vars,device)?;core.bind(&st.tokenizer)?;
    if core.content()?!=e.content{return Err(bad("endpoint semantic weights"));}Ok((core,Adam{moments}))
}
fn preserve_completed(dir:&Path,core:&mut Core,adam:&Adam,current:&Endpoint,before:&Endpoint,in_optimizer:bool,label:&str)->Result<Endpoint>{
    if !in_optimizer&&current.state.committed>before.state.committed {
        save_endpoint(dir,core,adam,current.state.clone(),label)
    }else{Ok(before.clone())}
}
fn history(s:&Study,arm:usize)->Result<Vec<Segment>>{let dir=s.root.join(ARMS[arm]);let mut out=vec![];
    let mut previous:Endpoint=read_confirmed(&dir.join("initial.r3b"))?;
    for i in 0..128 {let entered=dir.join(format!("segment-{i:03}-entered.r3b"));let returned=dir.join(format!("segment-{i:03}-returned.r3b"));
        if !entered.exists(){if returned.exists(){return Err(bad("segment return without entry"));}break;}
        let r:Segment=read_confirmed(&returned).map_err(|_|bad("UNKNOWN comparison segment; no automatic resume"))?;
        let start:binary::Value=read_confirmed(&entered)?;
        if r.policy!=digest(s)?||r.core!=ARMS[arm]||r.index!=i||r.trace_hash!=file_hash(&r.trace)?||r.endpoint.hash!=file_hash(&r.endpoint.path)?||!r.seconds.is_finite()||r.seconds<0.
            ||start["endpoint"]!=binary::record!(previous)||start["policy"]!=r.policy||start["index"]!=i||start["phase"]!=r.phase
            ||(r.endpoint.state.policy!=r.policy&&!initial_reference(s,arm,&r.endpoint))||r.endpoint.state.core!=s.cores[arm]||r.endpoint.state.committed<previous.state.committed
            ||r.endpoint.state.committed>3072||r.endpoint.state.committed-previous.state.committed>r.backwards
            ||r.control["teacher_calls"]!=0||r.control["generation_calls"]!=r.generation{return Err(bad("comparison segment binding"));}
        if !["COMPLETED","TIME_BUDGET"].contains(&r.stop.as_str())
            ||r.control["observed_conditions"].as_array().is_none_or(|v|v.iter().any(|v|v!="TIME_BUDGET")){return Err(bad("sticky comparison stop"));}
        previous=r.endpoint.clone();out.push(r);
    }Ok(out)
}
fn update(core:&Core,adam:&mut Adam,st:&mut ComparisonState,b:&Batch,c:&mut RunControl,backwards:&mut usize,in_optimizer:&mut bool)->Result<binary::Value>{
    c.check("before_core_forward")?;let started=Instant::now();let logits=core.forward(b)?;
    let(plain,loss,n,_)=response_objective(&logits,b,1.,true)?;let value=loss.to_scalar::<f32>()?;
    if !value.is_finite(){return Err(bad("nonfinite fresh loss"));}
    *backwards+=1;let graph=loss.backward()?;let mut grads=BTreeMap::new();
    for(name,var)in core.vars(){grads.insert(name.clone(),graph.get(var).ok_or_else(||bad("missing core gradient"))?.detach());}
    core.device().synchronize()?;let fb=started.elapsed().as_secs_f64();c.check("before_core_optimizer")?;
    let clock=st.committed+1;let lr=if st.config.warmup==0 {st.config.lr}
        else {st.config.lr*(clock.min(st.config.warmup)as f64/st.config.warmup as f64)};
    *in_optimizer=true;
    let(norm,delta)=adam.apply_admitted_step(core.vars(),&grads,&st.config,clock,lr,|_,_,_,_|Ok(()))?;
    core.device().synchronize()?;st.committed=clock;st.adam_clock=clock;st.input_tokens+=b.tokens as u64;st.target_tokens+=n as u64;
    *in_optimizer=false;
    Ok(binary::record!({"commit":clock,"adam_clock":clock,"input":b.tokens,"target":n,"loss":value,"token_ce":plain.to_scalar::<f32>()?,"lr":lr,
        "gradient_norm":norm,"update_l2":delta,"forward_backward_seconds":fb,"seconds":started.elapsed().as_secs_f64(),"rss_kib":rss_kib()?}))
}
type Panel=(String,Vec<Episode>,Vec<Meta>);
fn panels(s:&Study,step:usize,baseline:bool)->Result<Vec<Panel>>{
    let(c,tm,dm,_,_)=inputs(s)?;
    panels_from_inputs(s,step,baseline,&c,&tm,&dm)
}
fn panels_from_inputs(s:&Study,step:usize,baseline:bool,c:&data::native::Corpus,tm:&[Meta],dm:&[Meta])->Result<Vec<Panel>>{
    let mut out=vec![];
    if baseline{return Ok(vec![("baseline32".into(),c.validation[3072..3104].to_vec(),dm[3072..3104].to_vec())]);}
    if ![512,1536,3072].contains(&step){return Err(bad("unscheduled comparison panel"));}
    for(name,at,mid,last)in [("value",0,32,512),("citation",512,32,512),("S1Q1",2560,32,512),("word",3072,64,192),("renamed",3264,64,192)]{
        let n=if step==3072{last}else{mid};out.push((name.into(),c.validation[at..at+n].to_vec(),dm[at..at+n].to_vec()));
    }
    if step==3072{
        let ids=(0..2).flat_map(|v|(0..24).flat_map(move|b|(0..4).map(move|q|6144+v*768+b*4+q))).collect::<Vec<_>>();
        out.push(("train192".into(),ids.iter().map(|&i|c.train[i].clone()).collect(),ids.iter().map(|&i|tm[i].clone()).collect()));
        for transfer in [false,true]{out.push(identifiable::binding::citation::bridge_old_qa(&s.old_qa,transfer)?);}
    }Ok(out)
}
fn panel_score(p:&Panel,rows:&[binary::Value],tok:&ByteBpe,model:&str,raw:&str)->Result<binary::Value>{
    let(name,es,ms)=p;
    if name.starts_with("qa-"){identifiable::binding::citation::qa_rows_score(p,rows,tok,false,model,raw)}
    else if name=="value"{Ok(binary::record!({"joint":identifiable::binding::orbit_score(es,ms,rows,tok)?}))}
    else {identifiable::binding::citation::word::rows_score(es,ms,rows,tok)}
}
#[derive(Default)]
struct CollectionTiming {observe_seconds:f64,raw_publish_seconds:f64,validation_seconds:f64,
    new_calls:usize,reused_rows:usize}
fn collect(core:&Core,tok:&ByteBpe,dir:&Path,label:&str,es:&[Episode],binding:&binary::Value,
    ctl:&mut RunControl,generated:&mut usize,remaining_tokens:usize,
    timing:Option<&mut CollectionTiming>)->Result<Vec<binary::Value>>{
    let mut timing=timing;
    let validation_started=Instant::now();
    let path=dir.join(format!("{label}.r3rows"));let existed=path.exists();let entries=if existed{binary::read_value_records(&path)?}else{vec![]};
    if existed&&entries.first()!=Some(binding){return Err(bad("comparison collector binding"));}
    let mut rows=entries.into_iter().skip(1).collect::<Vec<_>>();if rows.len()>es.len(){return Err(bad("extra comparison RETURNED"));}
    for(i,row)in rows.iter().enumerate(){call_attempt(dir,label,"generation",binding,&es[i],i,Some(row))?;verify_generated(row,tok)?;
        if row["id"]!=es[i].id||row["expected"]!=es[i].answer{return Err(bad("comparison raw case"));}}
    if let Some(t)=timing.as_deref_mut(){t.reused_rows=rows.len();t.validation_seconds+=validation_started.elapsed().as_secs_f64();}
    let mut file=std::fs::OpenOptions::new().append(true).create_new(!existed).open(&path)?;
    if !existed{append_row(&mut file,binding)?;std::fs::File::open(dir)?.sync_all()?;}
    for(i,e)in es.iter().enumerate().skip(rows.len()){
        ctl.check("before_core_generation")?;
        if generated.saturating_add(e.request.limits.max_tokens as usize)>remaining_tokens{return Err(bad("comparison generation token cap"));}
        let attempt=prepare_call(dir,label,"generation",binding,e,i)?;
        let observe_started=Instant::now();
        let observed=core.observe(tok,e,ctl);
        core.device().synchronize()?;
        if let Some(t)=timing.as_deref_mut(){t.observe_seconds+=observe_started.elapsed().as_secs_f64();t.new_calls+=usize::from(matches!(&observed,ObservedCall::Returned(_)));}
        let mut row=match observed{ObservedCall::NotInvoked(_)=>{resolve_call(&attempt,None,ctl)?;ctl.stop_result()?;return Err(bad("uninvoked without stop"));},ObservedCall::Returned(r)=>r};
        row["attempt"]=binary::record!(attempt.file_name().unwrap().to_string_lossy());
        let publishing=Instant::now();
        append_row(&mut file,&row)?;resolve_call(&attempt,Some(&row),ctl)?;
        if let Some(t)=timing.as_deref_mut(){t.raw_publish_seconds+=publishing.elapsed().as_secs_f64();}
        *generated+=row["raw_tokens"].as_array().map_or(0,Vec::len);verify_generated(&row,tok)?;rows.push(row);
        ctl.stop_result()?;
        if (i+1)%64==0{println!("GENERATION panel={label} returned={}/{} new_tokens={generated}",i+1,es.len());}
    }Ok(rows)
}
fn evaluate(s:&Study,arm:usize,core:&Core,ep:&Endpoint,tok:&ByteBpe,ctl:&mut RunControl,generated:&mut usize,token_limit:usize,baseline:bool)->Result<()> {
    let dir=s.root.join(ARMS[arm]);
    for panel in panels(s,ep.state.committed,baseline)?{
        let label=format!("eval-{}-{}",ep.state.committed,panel.0);
        let binding=binary::record!({"policy":digest(s)?,"core":ARMS[arm],"native":ep.hash,"model":ep.content,"step":ep.state.committed,"runtime":s.runtime,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,"tokenizer":s.tokenizer_id});
        let rows=collect(core,tok,&dir,&label,&panel.1,&binding,ctl,generated,token_limit,None)?;
        let score=save_score(s,&dir,ep,tok,&panel,&rows)?;
        println!("SCORE core={} step={} panel={} joint={}",ARMS[arm],ep.state.committed,panel.0,score["joint"]);
    }ctl.seal_completed_no_call()
}
fn save_score(s:&Study,dir:&Path,ep:&Endpoint,tok:&ByteBpe,panel:&Panel,rows:&[binary::Value])->Result<binary::Value>{
    let label=format!("eval-{}-{}",ep.state.committed,panel.0);let raw=file_hash(&dir.join(format!("{label}.r3rows")))?;
    let mut score=panel_score(panel,rows,tok,&ep.content,&raw)?;score["model"]=binary::record!(ep.content);score["raw_hash"]=binary::record!(raw);score["policy"]=binary::record!(digest(s)?);
    if panel.0=="train192"{let(c,_,_,_,_)=inputs(s)?;let mut exposure=vec![0usize;c.train.len()];for &i in s.tape[..ep.state.committed].iter().flatten(){exposure[i]+=1;}
        score["actual_exposures"]=binary::record!(panel.1.iter().map(|e|c.train.iter().position(|v|v.id==e.id).map(|i|exposure[i])).collect::<Vec<_>>());}
    let path=dir.join(format!("{label}-score.r3b"));if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("comparison score mismatch"));}}else{publish_confirmed(&path,&score)?;}Ok(score)
}
fn finalize(s:&Study,arm:usize)->Result<()> {
    let lock=std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(s.root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("comparison process active"))?;
    let h=history(s,arm)?;let ep=h.last().ok_or_else(||bad("no finalizable endpoint"))?.endpoint.clone();
    let(_,_,_,tok,_)=inputs(s)?;let dir=s.root.join(ARMS[arm]);let mut pending=vec![];
    for p in panels(s,ep.state.committed,ep.state.committed==0)?{
        let label=format!("eval-{}-{}",ep.state.committed,p.0);let raw=binary::read_value_records(&dir.join(format!("{label}.r3rows")))?;
        let expected=binary::record!({"policy":digest(s)?,"core":ARMS[arm],"native":ep.hash,"model":ep.content,"step":ep.state.committed,"runtime":s.runtime,"cases":digest(&p.1)?,"metadata":digest(&p.2)?,"tokenizer":s.tokenizer_id});
        if raw.len()!=p.1.len()+1||raw.first()!=Some(&expected){return Err(bad("no-call finalization incomplete/binding"));}
        for(i,r)in raw[1..].iter().enumerate(){call_attempt(&dir,&label,"generation",&expected,&p.1[i],i,Some(r))?;verify_generated(r,&tok)?;}
        pending.push((p,raw));
    }
    for(p,raw)in pending{save_score(s,&dir,&ep,&tok,&p,&raw[1..])?;}
    println!("FINALIZED core={} step={} generation=0 teacher=0 optimizer=0 remaining_calls=0",ARMS[arm],ep.state.committed);Ok(())
}
fn review(s:&Study,arm:usize,core:&Core,ep:&Endpoint,tok:&ByteBpe,ctl:&mut RunControl,generated:&mut usize,token_limit:usize)->Result<()> {
    let step=ep.state.committed;let ps=panels(s,step,false)?;let dir=s.root.join(ARMS[arm]);let mut cases=vec![];
    for p in ps{let raw=binary::read_value_records(&dir.join(format!("eval-{step}-{}.r3rows",p.0)))?;
        if raw.len()!=p.1.len()+1{return Err(bad("review incomplete source"));}
        for(e,r)in p.1.into_iter().zip(raw.into_iter().skip(1)){verify_generated(&r,tok)?;if r["id"]!=e.id||r["expected"]!=e.answer{return Err(bad("review raw case"));}cases.push((p.0.clone(),e,r));}
    }
    // First four word orbits, selected solely by the frozen metadata order.
    let selected=cases.iter().enumerate().filter(|(_,v)|v.0=="word").take(16).map(|(i,_)|i).collect::<Vec<_>>();
    if selected.len()!=16{return Err(bad("review fixed16 cardinality"));}
    let es=selected.iter().map(|&i|cases[i].1.clone()).collect::<Vec<_>>();let original=selected.iter().map(|&i|cases[i].2.clone()).collect::<Vec<_>>();
    let label=format!("review-{step}");let binding=binary::record!({"policy":digest(s)?,"native":ep.hash,"cases":digest(&es)?,"source_raw":digest(&original)?,"indices":selected,"accuracy_denominator":false});
    let admission=dir.join("review-binding.r3b");if admission.exists(){if read_confirmed::<binary::Value>(&admission)?!=binding{return Err(bad("B limited to one endpoint/core"));}}else{publish_confirmed(&admission,&binding)?;}
    let rows=collect(core,tok,&dir,&label,&es,&binding,ctl,generated,token_limit,None)?;
    for(a,b)in rows.iter().zip(&original){for f in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"]{if a[f]!=b[f]{return Err(bad("fresh-process reproduction mismatch"));}}}
    ctl.seal_completed_no_call()?;publish_confirmed(&dir.join(format!("review-{step}-result.r3b")),&binary::record!({"policy":digest(s)?,"native":ep.hash,"matched":16,"teacher":0,"optimizer":0,"source_raw":digest(&original)?,"indices":selected}))
}
fn new_bytes(root:&Path)->Result<u64>{let mut n=0;for entry in std::fs::read_dir(root)?{let e=entry?;let m=e.metadata()?;n+=if m.is_dir(){new_bytes(&e.path())?}else{m.len()};}Ok(n)}
fn execute(s:&Study,arm:usize,phase:&str,until:usize)->Result<()> {
    let lock=std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(s.root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("comparison heavy process already active"))?;
    let all=[history(s,0)?,history(s,1)?];let h=&all[arm];let dir=s.root.join(ARMS[arm]);let index=h.len();
    let mut ep=if let Some(last)=h.last(){last.endpoint.clone()}else{read_confirmed(&dir.join("initial.r3b"))?};
    if (ep.state.policy!=digest(s)?&&!initial_reference(s,arm,&ep))||ep.state.core!=s.cores[arm]{return Err(bad("comparison endpoint core/policy"));}
    if phase=="baseline"&&ep.state.committed!=0{return Err(bad("baseline after learning"));}
    if phase=="train" {
        if until<=ep.state.committed||until>3072||![1,512,1536,3072].contains(&until){return Err(bad("training endpoint bound"));}
        read_confirmed::<binary::Value>(&dir.join("eval-0-baseline32-score.r3b"))?;
        if [512,1536].contains(&ep.state.committed){for p in panels(s,ep.state.committed,false)?{read_confirmed::<binary::Value>(&dir.join(format!("eval-{}-{}-score.r3b",ep.state.committed,p.0)))?;}}
    }
    let used=h.iter().map(|v|v.seconds).sum::<f64>();let backwards=h.iter().map(|v|v.backwards).sum::<usize>();
    let calls=all.iter().flatten().map(|v|v.generation).sum::<usize>();let tokens=all.iter().flatten().map(|v|v.generated_tokens).sum::<usize>();
    // Reserve one complete native and bounded receipts before entering model work.
    if used>=7200.||calls>8192||tokens>1048576||new_bytes(&s.artifact_root)?.saturating_add(160*1024*1024)>2*1024*1024*1024{return Err(bad("comparison resource cap reached"));}
    let(_,_,_,tok,samples)=inputs(s)?;let device=Backend::Metal0.open()?;s.runtime.verify(&device)?;
    let(core,mut adam)=load_endpoint(&ep,device)?;let mut core=core;
    let cancel=Arc::new(AtomicBool::new(false));let signal=cancel.clone();ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut ctl=RunControl::new(cancel,Duration::from_secs_f64((7200.-used).min(1800.)),16*1024*1024)?;ctl.set_call_limits(8192-calls,0);
    let trace=dir.join(format!("segment-{index:03}-trace.r3rows"));let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&trace)?;
    let before=ep.clone();publish_confirmed(&dir.join(format!("segment-{index:03}-entered.r3b")),&binary::record!({"policy":digest(s)?,"phase":phase,"endpoint":ep,"index":index,"remaining_seconds":7200.-used,"generation_remaining":8192-calls,"backward_remaining":3088-backwards}))?;
    let began=Instant::now();let(mut new_backward,mut generated)=(0,0);let mut in_optimizer=false;
    let result=(||->Result<()>{
        if phase=="train"{
            ep.state.policy=digest(s)?;ep.state.runtime=s.runtime.clone();
            let next=[1,512,1536,3072].into_iter().find(|&n|n>ep.state.committed).unwrap();let end=until.min(next);
            while ep.state.committed<end {
                ctl.check("before_comparison_update")?;if backwards+new_backward>=3088{return Err(bad("comparison backward cap"));}
                let row=&s.tape[ep.state.committed];let b=batch(&samples,row,core.device())?;
                append_row(&mut f,&binary::record!({"phase":"ENTERED","cursor":ep.state.committed+1,"input":b.tokens,"batch":row}))?;
                let trace=update(&core,&mut adam,&mut ep.state,&b,&mut ctl,&mut new_backward,&mut in_optimizer)?;append_row(&mut f,&trace)?;
                if ep.state.committed%64==0||ep.state.committed==1{println!("TRAIN core={} step={}/3072 input={} target={} loss={} rss={} last_saved={}",ARMS[arm],ep.state.committed,ep.state.input_tokens,ep.state.target_tokens,trace["loss"],trace["rss_kib"],before.state.committed);}
            }Ok(())
        }else if phase=="review"{review(s,arm,&core,&ep,&tok,&mut ctl,&mut generated,1048576-tokens)}
        else{evaluate(s,arm,&core,&ep,&tok,&mut ctl,&mut generated,1048576-tokens,phase=="baseline")}
    })();
    if let Err(e)=&result{ctl.classify_error(e);}
    let control=ctl.receipt();
    let stop=if ctl.reason()==Some("TIME_BUDGET")&&control["observed_conditions"]!=binary::record!(["TIME_BUDGET"]){"INTEGRITY_FAIL"}else{ctl.reason().unwrap_or("COMPLETED")}.to_string();
    // Preserve safe completed work even after cancellation; the sticky stop still blocks resume.
    // An interrupted optimizer may have partially changed tensors and must never be published.
    if phase=="train"{ep=preserve_completed(&dir,&mut core,&adam,&ep,&before,in_optimizer,&format!("segment-{index:03}"))?;}
    append_row(&mut f,&binary::record!({"phase":"RETURNED","endpoint":ep,"stop":stop,"discarded":new_backward.saturating_sub(ep.state.committed.saturating_sub(before.state.committed))}))?;
    let receipt=ctl.receipt();let segment=Segment{policy:digest(s)?,core:ARMS[arm].into(),phase:phase.into(),index,endpoint:ep.clone(),backwards:new_backward,
        generation:receipt["generation_calls"].as_u64().unwrap_or(0)as usize,generated_tokens:generated,seconds:began.elapsed().as_secs_f64(),stop:stop.clone(),trace:trace.clone(),trace_hash:file_hash(&trace)?,control:receipt};
    publish_confirmed(&dir.join(format!("segment-{index:03}-returned.r3b")),&segment)?;
    println!("SEGMENT core={} phase={phase} step={} backward={new_backward} generation={} tokens={generated} seconds={:.3} stop={stop} native={} new_bytes={}",ARMS[arm],ep.state.committed,segment.generation,segment.seconds,ep.path.display(),new_bytes(&s.artifact_root)?);
    result
}
fn tiny(output:&Path,arm:&str,until:usize,resume:bool,cancel_after_commit:bool)->Result<()> {
    if cancel_after_commit&&(resume||until!=1){return Err(bad("cancel regression requires one fresh update"));}
    if !(1..=2).contains(&until){return Err(bad("TINY only two updates"));}
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let tok=ByteBpe::train(&[b"tiny answer".to_vec()],&neural::hash(b"tiny answer"),264)?;
    let tc=Config::tiny(tok.vocab_size());let gc=gru::Config{equation:gru::EQUATION.into(),vocab:tok.vocab_size(),embedding:32,hidden:32,layers:2,context:64,eps:1e-6};
    let mut cfg=config();cfg.seq_len=16;cfg.microbatch=2;
    let mut st=ComparisonState{schema:1,core:if arm=="TRPP"{ComparisonCore::Trpp(tc.clone())}else{ComparisonCore::Gru(gc.clone())},policy:neural::hash(b"quality-gru-direct-tiny-v1"),tokenizer:tok.semantic_id(),framing:Framing::QuestionEvidence.digest(),runtime,objective:checkpoint::ANSWER_MEAN_FAMILY,optimizer:"FRESH_ADAMW_ALL_V1".into(),config:cfg,committed:0,adam_clock:0,input_tokens:0,target_tokens:0};
    let(mut core,mut adam)=if resume{
        let ep:Endpoint=read_confirmed(&output.join("step-1.r3b"))?;
        let mut expected=st.clone();expected.committed=1;expected.adam_clock=1;expected.input_tokens=ep.state.input_tokens;expected.target_tokens=ep.state.target_tokens;
        if expected!=ep.state{return Err(bad("TINY expected exact native policy"));}st=ep.state.clone();let loaded=load_endpoint(&ep,device)?;
        if let Core::Gru(model)=&loaded.0{let bytes=std::fs::read(output.join("state-1.r3b"))?;let state=model.restore_state(&bytes,"process-snapshot")?;if state.positions()!=[3]{return Err(bad("new process hidden snapshot positions"));}}
        loaded
    }else{
        std::fs::create_dir(output)?;let tr=Transformer::init(tc,SEED,device.clone())?;
        let mut core=if arm=="TRPP"{Core::Tr(tr)}else{Core::Gru(gru::Gru::init(gc,SEED,&tr.vars["embedding"],device)?)};
        core.bind(&tok.semantic_id())?;let adam=Adam::new(core.vars())?;
        if adam.moments.values().any(|t|t.flatten_all().and_then(|t|t.to_vec1::<f32>()).unwrap().iter().any(|v|*v!=0.)){return Err(bad("fresh moments nonzero"));}
        (core,adam)
    };
    if until<=st.committed{return Err(bad("TINY no repeated optimizer"));}
    let samples=vec![Sample{tokens:vec![BOS,8,9,10,11,EOS],response_start:3,curriculum:false},Sample{tokens:vec![BOS,9,8,EOS],response_start:2,curriculum:false}];
    let cancel=Arc::new(AtomicBool::new(false));
    let mut ctl=RunControl::new(cancel.clone(),Duration::from_secs(120),16*1024*1024)?;
    let before=if cancel_after_commit{Some(save_endpoint(output,&mut core,&adam,st.clone(),"before-cancel")?)}else{None};
    let mut backward=0;let mut in_optimizer=false;
    while st.committed<until{let b=batch(&samples,&[0,1],core.device())?;let row=update(&core,&mut adam,&mut st,&b,&mut ctl,&mut backward,&mut in_optimizer)?;
        publish_confirmed(&output.join(format!("update-{}.r3b",st.committed)),&row)?;}
    let ep=if let Some(before)=before{
        cancel.store(true,Ordering::Relaxed);assert!(ctl.check("before_comparison_update").is_err());
        assert_eq!(ctl.reason(),Some("CANCELLED"));
        let mut current=before.clone();current.state=st;
        let saved=preserve_completed(output,&mut core,&adam,&current,&before,in_optimizer,"tiny")?;
        assert_eq!(saved.state.committed,1);
        assert!(preserve_completed(output,&mut core,&adam,&current,&before,true,"must-not-save")?==before);
        publish_confirmed(&output.join("cancelled.r3b"),&binary::record!({"stop":ctl.reason(),"resume":false,"backward":backward,"endpoint":saved}))?;
        saved
    }else{save_endpoint(output,&mut core,&adam,st,"tiny")?};
    // Wrong policies/cores and legacy loaders fail before dispatch or optimizer.
    let mut wrong=ep.state.clone();wrong.tokenizer=neural::hash(b"wrong tokenizer");assert!(artifact::load_comparison(&ep.path,&wrong,core.device()).is_err());
    wrong=ep.state.clone();wrong.adam_clock+=1;assert!(artifact::load_comparison(&ep.path,&wrong,core.device()).is_err());
    assert!(artifact::load(&ep.path,Device::Cpu,false).is_err());
    let(restored,restored_adam)=load_endpoint(&ep,core.device().clone())?;
    for(n,v)in core.vars(){if v.flatten_all()?.to_vec1::<f32>()?!=restored.vars()[n].flatten_all()?.to_vec1::<f32>()?{return Err(bad("TINY native weights readback"));}}
    for(n,v)in &adam.moments{if v.flatten_all()?.to_vec1::<f32>()?!=restored_adam.moments[n].flatten_all()?.to_vec1::<f32>()?{return Err(bad("TINY native Adam readback"));}}
    if let Core::Gru(model)=&core{
        if resume&&model.restore_state(&std::fs::read(output.join("state-1.r3b"))?,"process-snapshot").is_ok(){return Err(bad("changed weights accepted old hidden"));}
        let mut state=model.state(1,"process-snapshot")?;model.forward_state(&Tensor::new(&[[BOS,8,9]],&model.device)?,None,&mut state,"process-snapshot")?;
        let bytes=model.snapshot(&state,"process-snapshot")?;let restored=model.restore_state(&bytes,"process-snapshot")?;
        if state.positions()!=restored.positions(){return Err(bad("GRU state snapshot"));}
        if model.restore_state(&bytes,"other-request").is_ok(){return Err(bad("GRU stale reset accepted"));}
        replica_v3::codec::publish_new(&output.join(format!("state-{}.r3b",ep.state.committed)),|f,_|{f.write_all(&bytes)?;Ok(())})?;
    }
    publish_confirmed(&output.join(format!("step-{}.r3b",ep.state.committed)),&ep)?;
    println!("TINY core={arm} actual_device={:?} committed={} clock={} weights={} native={} teacher=0 generation=0",core.device().location(),ep.state.committed,ep.state.adam_clock,ep.content,ep.hash);Ok(())
}
fn verify_tiny(continuous:&Path,split:&Path)->Result<()> {
    let a:Endpoint=read_confirmed(&continuous.join("step-2.r3b"))?;let b:Endpoint=read_confirmed(&split.join("step-2.r3b"))?;
    if a.state!=b.state{return Err(bad("continuous/split clock or policy"));}
    let(_,aw,aa)=artifact::load_comparison(&a.path,&a.state,&Device::Cpu)?;let(_,bw,ba)=artifact::load_comparison(&b.path,&b.state,&Device::Cpu)?;
    let mut maximum=0f64;
    for (x,y)in [(&aw,&bw),(&aa,&ba)]{for(n,t)in x{let a=t.flatten_all()?.to_vec1::<f32>()?;let b=y[n].flatten_all()?.to_vec1::<f32>()?;
        let mut d=0.;let mut den=0.;let(mut dot,mut other)=(0.,0.);let mut max=0f64;
        for(&a,&b)in a.iter().zip(&b){let(a,b)=(a as f64,b as f64);d+=(a-b).powi(2);den+=a*a;other+=b*b;dot+=a*b;max=max.max((a-b).abs());}
        let nrmse=(d/den.max(1e-20)).sqrt();maximum=maximum.max(nrmse);
        if den>1e-12&&(nrmse>0.003||dot/(den*other).sqrt().max(1e-20)<0.999)||den<=1e-12&&max>2e-5{return Err(bad(&format!("continuous/split tensor {n} nrmse={nrmse}")));}
    }}
    // Native corruption and foreign core are rejected without optimizer or generation.
    let mut bad_state=a.state.clone();bad_state.core=match &a.state.core{ComparisonCore::Trpp(c)=>ComparisonCore::Gru(gru::Config::matched(c.vocab)),ComparisonCore::Gru(c)=>ComparisonCore::Trpp(Config::tiny(c.vocab))};
    if artifact::load_comparison(&a.path,&bad_state,&Device::Cpu).is_ok(){return Err(bad("wrong core native accepted"));}
    let mut bytes=std::fs::read(&a.path)?;bytes.truncate(bytes.len()-1);let broken=split.join("truncated.r3model");
    replica_v3::codec::publish_new(&broken,|f,_|{f.write_all(&bytes)?;Ok(())})?;
    if artifact::load_comparison(&broken,&a.state,&Device::Cpu).is_ok(){return Err(bad("truncated native accepted"));}
    let result=binary::record!({"continuous":a.hash,"split":b.hash,"max_nrmse":maximum,"clock":2,"native":true,"wrong_core_rejected":true,"truncated_rejected":true,"optimizer":0});
    publish_confirmed(&split.join("parity.r3b"),&result)?;println!("TINY_PROCESS_PARITY {result}");Ok(())
}
fn report(s:&Study)->Result<()> {
    let mut models=vec![];
    for(i,arm)in ARMS.iter().enumerate(){let h=history(s,i)?;let Some(last)=h.last()else{models.push(binary::record!({"core":arm,"status":"NOT_RUN"}));continue;};
        let step=last.endpoint.state.committed;let mut scores=BTreeMap::new();
        if [512,1536,3072].contains(&step){for p in panels(s,step,false)?{let path=s.root.join(arm).join(format!("eval-{step}-{}-score.r3b",p.0));if path.exists(){scores.insert(p.0,read_confirmed::<binary::Value>(&path)?);}}}
        models.push(binary::record!({"core":arm,"endpoint":last.endpoint,"backward":h.iter().map(|v|v.backwards).sum::<usize>(),"generation":h.iter().map(|v|v.generation).sum::<usize>(),"generated_tokens":h.iter().map(|v|v.generated_tokens).sum::<usize>(),
            "active_seconds":h.iter().map(|v|v.seconds).sum::<f64>(),"scores":scores,"stop":last.stop,"quality_acceptance":"NOT_GRANTED","device_total_memory":"UNKNOWN"}));
    }
    let result=binary::record!({"contract":CONTRACT,"policy":digest(s)?,"models":models,"new_artifact_bytes":new_bytes(&s.artifact_root)?,"PRODUCT_INTEGRATION":"NOT_RUN","GOAL1_ACCEPTED":false});
    println!("{result}");Ok(())
}
pub fn run(action:Action)->Result<()>{match action{
    Action::BindingRecovery{action}=>binding_recovery::run(action),
    Action::Prepare{word_root,output,audit_only}=>prepare(&word_root,&output,audit_only),Action::Admit{root,review}=>admit(&root,&review),
    Action::Revise{previous,output,failed_review}=>revise(&previous,&output,&failed_review),
    Action::Execute{root,core,phase,until}=>{let s=study(&root,true)?;let arm=ARMS.iter().position(|v|*v==core).ok_or_else(||bad("core"))?;if phase=="finalize"{finalize(&s,arm)}else{execute(&s,arm,&phase,until)}},
    Action::Report{root}=>report(&study(&root,false)?),Action::Tiny{output,core,until,resume,cancel_after_commit}=>tiny(&output,&core,until,resume,cancel_after_commit),
    Action::VerifyTiny{continuous,split}=>verify_tiny(&continuous,&split),
}}
