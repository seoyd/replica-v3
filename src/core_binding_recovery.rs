//! Training-only, parent-bound answer/branch study. No resolver enters inference.
use super::*;
use std::collections::BTreeSet;

const CONTRACT:&str="R3-TRPP-BINDING-RECOVERY-1.0";
const PARENT_HASH:&str="f58e6564134fccf7926df1f2d077396fe08aac67cf84ad8bbb88324872a7440f";
const RULE:&str="FULL_WORD_OTHER_PROVIDED_VALUE_OR_EVENT_FIRST_LCP_V1";
const LAMBDA:f64=0.1;
const WORD_START:usize=6144;
const WORD_COUNT:usize=1536;
// Four already executed TINY optimizer steps, one rejected init, and their
// process/validation time belong to this study. Reserve above observed ~16 s.
const PRIOR_TINY_MODEL_SECONDS:f64=20.;

#[derive(Subcommand)]
pub enum Action {
    Prepare {#[arg(long)] parent:PathBuf,#[arg(long)] output:PathBuf},
    Inspect {#[arg(long)] root:PathBuf},
    Admit {#[arg(long)] root:PathBuf,#[arg(long)] review:PathBuf},
    Diagnose {#[arg(long)] root:PathBuf},
    Train {#[arg(long)] root:PathBuf,#[arg(long,value_parser=["C","T"])] arm:String,
        #[arg(long)] until:usize},
    Evaluate {#[arg(long)] root:PathBuf,#[arg(long,value_parser=["C","T"])] arm:String,
        #[arg(long)] step:usize},
    Confirm {#[arg(long)] root:PathBuf,#[arg(long,value_parser=["C","T"])] arm:String},
    ConfirmationReport {#[arg(long)] root:PathBuf},
    Review {#[arg(long)] root:PathBuf,#[arg(long,value_parser=["C","T"])] arm:String},
    Report {#[arg(long)] root:PathBuf},
    Tiny {#[arg(long)] root:PathBuf,#[arg(long)] until:usize,#[arg(long)] resume:bool},
    VerifyTiny {#[arg(long)] continuous:PathBuf,#[arg(long)] split:PathBuf},
}
#[derive(Clone,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
struct Branch {position:usize,positive:u32,negative:u32}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    contract:String, root:PathBuf, source:String, parent_root:PathBuf, parent_plan_hash:String,
    parent_endpoint:Endpoint, parent_segment_hash:String, tokenizer:String, corpus:String,
    runtime:RuntimeProfile,
    tape:Vec<[usize;8]>, pairs:Vec<Vec<Branch>>, confirmation_hash:String, lr:f64,
    diagnostic:binary::Value, budget:binary::Value,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Run {
    policy:String, arm:String, before:Endpoint, after:Endpoint, attempted:usize,
    committed:usize, input:u64, target:u64, seconds:f64, stop:String,
    trace:PathBuf, trace_hash:String, control:binary::Value,
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct EvalRun {policy:String,arm:String,step:usize,native:String,seconds:f64,calls:usize,tokens:usize,stop:String,control:binary::Value}
fn source()->Result<String>{digest(&(source_digest()?,neural::hash(include_bytes!("core_binding_recovery.rs"))))}
fn arm_policy(s:&Study,arm:&str)->Result<String>{
    digest(&(CONTRACT,&s.source,&s.parent_plan_hash,&s.parent_endpoint.hash,&s.corpus,
        &s.tokenizer,&s.tape,&s.confirmation_hash,&s.runtime,s.lr,RULE,LAMBDA,"per-answer EOS-included; review D=0",arm))
}
fn word_answer(e:&Episode)->Result<(String,i64)> {
    let(v,rest)=e.answer.split_once("입니다. [event:").ok_or_else(||bad("word answer grammar"))?;
    let digits=rest.strip_suffix(']').ok_or_else(||bad("word answer suffix"))?;
    if v.is_empty()||digits.is_empty()||!digits.bytes().all(|c|c.is_ascii_digit()) {return Err(bad("word answer parts"));}
    let id=digits.parse::<i64>().map_err(|_|bad("word event ID"))?;
    if id<=0||format!("{v}입니다. [event:{id}]")!=e.answer {return Err(bad("word canonical answer"));}
    Ok((v.into(),id))
}
fn first_branch(tok:&ByteBpe,sample:&Sample,alternative:&str)->Result<Option<Branch>> {
    if sample.response_start==0||sample.response_start>=sample.tokens.len()||sample.tokens.last()!=Some(&EOS){return Err(bad("branch positive boundary"));}
    let mut other=tok.encode(alternative.as_bytes())?;other.push(EOS);
    let gold=&sample.tokens[sample.response_start..];
    let Some(j)=gold.iter().zip(&other).position(|(a,b)|a!=b) else {
        if gold==other{return Ok(None)}else{return Err(bad("branch target prefix identical but length differs"))}
    };
    let position=sample.response_start+j-1;
    if position>=sample.tokens.len()-1||gold[j] as usize>=tok.vocab_size()||other[j] as usize>=tok.vocab_size(){return Err(bad("branch token bounds"));}
    Ok(Some(Branch{position,positive:gold[j],negative:other[j]}))
}
fn word_branches(e:&Episode,sample:&Sample,tok:&ByteBpe)->Result<Vec<Branch>> {
    if e.request.evidence.items.len()!=2||e.request.input.is_empty(){return Err(bad("word two provided records"));}
    let (value,id)=word_answer(e)?;
    let selected=e.request.evidence.items.iter().position(|r|r.event_id==id).ok_or_else(||bad("positive support absent"))?;
    let own=parsed_record(&e.request.evidence.items[selected])?;
    if own.2!=value||identifiable::binding::citation::bridge_resolve(&e.request)?!=e.answer{return Err(bad("word label/provided record mismatch"));}
    let other=&e.request.evidence.items[1-selected];let other_value=parsed_record(other)?.2;
    let mut pairs=BTreeSet::new();
    if other_value!=value {
        if let Some(b)=first_branch(tok,sample,&format!("{other_value}입니다. [event:{id}]"))? {pairs.insert((b.position,b.positive,b.negative));}
    }
    if other.event_id!=id {
        if let Some(b)=first_branch(tok,sample,&format!("{value}입니다. [event:{}]",other.event_id))? {pairs.insert((b.position,b.positive,b.negative));}
    }
    if pairs.len()>2{return Err(bad("more than two deduplicated word branches"));}
    Ok(pairs.into_iter().map(|(position,positive,negative)|Branch{position,positive,negative}).collect())
}
fn parent(root:&Path)->Result<(StudyParent,Endpoint)> {
    let root=root.canonicalize()?;
    let p:StudyParent=read_confirmed(&root.join("plan.r3b"))?;
    if p.contract!=super::CONTRACT||p.root!=root||p.cores[0]!=ComparisonCore::Trpp(Config::small(ByteBpe::load(&p.tokenizer)?.vocab_size()))
        ||p.tape.len()!=3072 {return Err(bad("closed FULL comparison parent"));}
    let h=history(&p,0)?;
    let last=h.last().ok_or_else(||bad("parent history empty"))?;
    let ep=&last.endpoint;
    if ep.state.committed!=3072||ep.state.adam_clock!=3072||ep.state.objective!=checkpoint::ANSWER_MEAN_FAMILY
        ||ep.hash!=PARENT_HASH||file_hash(&ep.path)?!=PARENT_HASH||last.stop!="COMPLETED" {return Err(bad("parent3072 native/terminal identity"));}
    Ok((p,ep.clone()))
}
fn compatible_parent_runtime(parent:&RuntimeProfile,current:&RuntimeProfile)->Result<()> {
    let mut expected=current.clone();expected.binary=parent.binary.clone();
    if &expected!=parent{return Err(bad("parent runtime changed beyond binary identity"));}Ok(())
}
// A type alias keeps the historical plan untouched and makes the parent boundary explicit.
type StudyParent=super::Study;
fn last_lr(p:&StudyParent)->Result<f64>{
    let h=history(p,0)?;
    for segment in h.iter().rev(){
        if segment.phase!="train"{continue;}
        let rows=binary::read_value_records(&segment.trace)?;
        for row in rows.iter().rev(){if row["commit"].as_u64()==Some(3072){
            let lr=row["lr"].as_f64().ok_or_else(||bad("parent LR missing"))?;
            if !lr.is_finite()||lr<=0. {return Err(bad("parent LR nonpositive/nonfinite"));}return Ok(lr);
        }}
    }Err(bad("parent last applied LR not found"))
}
fn fixed_manifest(p:&StudyParent,tm:&[Meta],dm:&[Meta],c:&data::native::Corpus)->Result<binary::Value>{
    let check_orbits=|ms:&[Meta],episodes:&[Episode],start:usize,n:usize|->Result<Vec<String>>{
        let mut bases=vec![];
        for at in (start..start+n).step_by(4){let group=&ms[at..at+4];
            if group.iter().enumerate().any(|(v,m)|m.base!=group[0].base||m.view!=v||m.id!=episodes[at+v].id){
                return Err(bad("fixed diagnostic incomplete four-view orbit"));}
            bases.push(group[0].base.clone());
        }Ok(bases)
    };
    let train=check_orbits(tm,&c.train,WORD_START,64)?;
    let word=check_orbits(dm,&c.validation,3072,64)?;
    let renamed=check_orbits(dm,&c.validation,3264,64)?;
    let qa=identifiable::binding::citation::bridge_old_qa(&p.old_qa,false)?;
    let mut picked=vec![];let mut counts=[0usize;8];
    for (i,m)in qa.2.iter().enumerate(){if m.bucket<8&&counts[m.bucket]<8 {picked.push(i);counts[m.bucket]+=1;}}
    if counts!=[8;8]{return Err(bad("fixed QA A-H eight each"));}
    Ok(binary::record!({"train_indices":(WORD_START..WORD_START+64).collect::<Vec<_>>(),"train_bases":train,
        "word_indices":(3072..3136).collect::<Vec<_>>(),"word_bases":word,
        "renamed_indices":(3264..3328).collect::<Vec<_>>(),"renamed_bases":renamed,
        "qa_indices":picked,"qa_buckets":counts,"metadata_only":true,"model_calls":0}))
}
fn prepare(parent_root:&Path,output:&Path)->Result<()> {
    let (p,ep)=parent(parent_root)?;
    let runtime=RuntimeProfile::capture(Backend::Metal0,&Backend::Metal0.open()?)?;
    compatible_parent_runtime(&ep.state.runtime,&runtime)?;
    let tiny_continuous=PathBuf::from("artifacts/trpp-binding-tiny-20260926-c2").canonicalize()?;
    let tiny_split=PathBuf::from("artifacts/trpp-binding-tiny-20260926-s2").canonicalize()?;
    verify_tiny(&tiny_continuous,&tiny_split)?;
    let tiny_continuous_hash=file_hash(&tiny_continuous.join("segment-2.r3b"))?;
    let tiny_split_hash=file_hash(&tiny_split.join("segment-2.r3b"))?;
    let (c,tm,dm,tok,samples)=inputs(&p)?;
    if c.train.len()!=7680||c.validation.len()!=3456||samples.len()!=7680{return Err(bad("FULL dimensions"));}
    let mut answers=BTreeMap::<String,String>::new();
    let(mut min_input,mut max_input,mut min_target,mut max_target)=(usize::MAX,0,usize::MAX,0);
    for(e,sample)in c.train.iter().zip(&samples){
        let key=digest(&e.request)?;
        if answers.insert(key.clone(),e.answer.clone()).is_some_and(|prior|prior!=e.answer){
            return Err(bad("same FULL request has conflicting labels"));
        }
        if sample.response_start==0||sample.response_start>=sample.tokens.len()||sample.tokens.last()!=Some(&EOS){
            return Err(bad("FULL supervised answer/EOS boundary"));
        }
        let input=sample.tokens.len()-1;let target=sample.tokens.len()-sample.response_start;
        min_input=min_input.min(input);max_input=max_input.max(input);
        min_target=min_target.min(target);max_target=max_target.max(target);
    }
    let mut all=Vec::with_capacity(WORD_COUNT);let mut coverage=0;
    for i in WORD_START..WORD_START+WORD_COUNT{
        let pairs=word_branches(&c.train[i],&samples[i],&tok)?;
        coverage+=usize::from(!pairs.is_empty());all.push(pairs);
    }
    let fixed=fixed_manifest(&p,&tm,&dm,&c)?;
    let used_ids=c.train.iter().chain(&c.validation).flat_map(|e|e.request.evidence.items.iter().map(|r|r.event_id)).collect::<BTreeSet<_>>();
    let confirmation=identifiable::binding::citation::word::sealed_confirmation(&used_ids)?;
    let train_bases=tm.iter().map(|m|m.base.as_str()).collect::<BTreeSet<_>>();
    let dev_bases=dm.iter().map(|m|m.base.as_str()).collect::<BTreeSet<_>>();
    if confirmation.1.iter().any(|m|train_bases.contains(m.base.as_str())||dev_bases.contains(m.base.as_str())){
        return Err(bad("confirmation base overlaps train/development"));
    }
    let lr=last_lr(&p)?;
    let tape=p.tape[..512].to_vec();
    let(mut input,mut target)=(0u64,0u64);let mut exposure=vec![0u16;c.train.len()];
    for row in &tape{for &i in row{let s=samples.get(i).ok_or_else(||bad("recovery tape sample bound"))?;
        exposure[i]=exposure[i].checked_add(1).ok_or_else(||bad("FULL exposure overflow"))?;
        input+=(s.tokens.len()-1)as u64;target+=(s.tokens.len()-s.response_start)as u64;}}
    if coverage*10<WORD_COUNT*9||input>2_000_000||target>300_000 {return Err(bad("negative coverage or per-arm token budget"));}
    let original_parent_plan=parent_root.canonicalize()?.join("plan.r3b");
    let root=output.to_path_buf();std::fs::create_dir(&root)?;let root=root.canonicalize()?;
    let confirmation_path=root.join("sealed-confirmation.r3b");
    publish_confirmed(&confirmation_path,&confirmation)?;
    let s=Study{contract:CONTRACT.into(),root:root.clone(),source:source()?,parent_root:p.root.clone(),parent_plan_hash:file_hash(&original_parent_plan)?,
        parent_endpoint:ep.clone(),parent_segment_hash:file_hash(&p.root.join("TRPP/segment-006-returned.r3b"))?,
        tokenizer:p.tokenizer_id.clone(),corpus:file_hash(&p.word_root.join("corpus.r3cor"))?,runtime,tape,pairs:all,
        confirmation_hash:file_hash(&confirmation_path)?,lr,
        diagnostic:fixed,budget:binary::record!({"small_commit_per_arm":512,"small_backward_per_arm":528,"small_input_per_arm":2_000_000,
            "small_target_per_arm":300_000,"tiny_commit_total":8,"tiny_backward_total":16,"teacher":1024,
            "generation":4096,"generated_tokens":524288,"model_work_seconds":3600,"train_cutoff_seconds":2400,
            "artifact_bytes":1073741824u64,"target_growth_bytes":1073741824u64,"expected_input_per_arm":input,"expected_target_per_arm":target,
            "prior_tiny_optimizer":4,"prior_tiny_backward":4,"prior_tiny_observed_model_seconds":15.803,
            "prior_tiny_reserved_model_seconds":PRIOR_TINY_MODEL_SECONDS,"prior_tiny_continuous":tiny_continuous,
            "prior_tiny_split":tiny_split,"prior_tiny_continuous_hash":tiny_continuous_hash,"prior_tiny_split_hash":tiny_split_hash,
            "prior_failed_diagnosis_observed_seconds":86.667,"prior_failed_diagnosis_reserved_seconds":90.0,
            "unique_train_requests":answers.len(),"conflicting_train_labels":0,"min_input":min_input,"max_input":max_input,
            "min_target_including_eos":min_target,"max_target_including_eos":max_target,
            "first512_exposure_hash":digest(&exposure)?,"first512_review_exposures":exposure[..WORD_START].iter().map(|&x|x as usize).sum::<usize>(),
            "first512_word_exposures":exposure[WORD_START..].iter().map(|&x|x as usize).sum::<usize>()})};
    publish_confirmed(&root.join("plan.r3b"),&s)?;
    publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"contract":CONTRACT,"policy":digest(&s)?,
        "parent_native":ep.hash,"parent_clock":ep.state.adam_clock,"parent_plan":s.parent_plan_hash,
        "training_word_coverage":coverage,"training_word_total":WORD_COUNT,"negative_rule":RULE,"lambda":LAMBDA,
        "diagnostic":s.diagnostic,"budget":s.budget,"source":s.source,"independent_A":"PENDING",
        "optimizer":0,"generation":0,"teacher":0}))?;
    println!("BINDING_PREPARED policy={} parent={} word_negative_coverage={}/{} input_per_arm={input} target_per_arm={target} lr={lr} A_PENDING",digest(&s)?,ep.hash,coverage,WORD_COUNT);
    Ok(())
}
fn inspect(root:&Path)->Result<()> {
    let s=checked(root)?;
    println!("BINDING_INSPECT policy={} parent={} clock={} A_PENDING",digest(&s)?,s.parent_endpoint.hash,s.parent_endpoint.state.adam_clock);Ok(())
}
fn checked(root:&Path)->Result<Study>{
    let s:Study=read_confirmed(&root.join("plan.r3b"))?;
    if s.contract!=CONTRACT||s.root!=root.canonicalize()?||s.source!=source()?||s.tape.len()!=512||s.pairs.len()!=WORD_COUNT{return Err(bad("binding study identity"));}
    if file_hash(&s.root.join("sealed-confirmation.r3b"))?!=s.confirmation_hash{return Err(bad("binding sealed confirmation changed"));}
    for (path_key,hash_key) in [("prior_tiny_continuous","prior_tiny_continuous_hash"),("prior_tiny_split","prior_tiny_split_hash")] {
        let dir=Path::new(s.budget[path_key].as_str().ok_or_else(||bad("tiny accounting path"))?);
        if file_hash(&dir.join("segment-2.r3b"))?!=s.budget[hash_key].as_str().ok_or_else(||bad("tiny accounting hash"))?{
            return Err(bad("tiny evidence changed"));
        }
    }
    if s.budget["prior_tiny_optimizer"]!=4||s.budget["prior_tiny_reserved_model_seconds"]!=PRIOR_TINY_MODEL_SECONDS{
        return Err(bad("tiny common budget accounting"));
    }
    s.runtime.verify(&Backend::Metal0.open()?)?;
    compatible_parent_runtime(&s.parent_endpoint.state.runtime,&s.runtime)?;
    let(p,ep)=parent(&s.parent_root)?;
    if ep!=s.parent_endpoint||file_hash(&s.parent_root.join("plan.r3b"))?!=s.parent_plan_hash
        ||file_hash(&s.parent_root.join("TRPP/segment-006-returned.r3b"))?!=s.parent_segment_hash
        ||file_hash(&p.word_root.join("corpus.r3cor"))?!=s.corpus||p.tokenizer_id!=s.tokenizer||p.tape[..512]!=s.tape
        ||last_lr(&p)?!=s.lr{return Err(bad("binding parent/data/tape/LR changed"));}
    Ok(s)
}
fn admitted(s:&Study)->Result<()> {
    let a:binary::Value=read_confirmed(&s.root.join("review-a.r3b"))?;
    let report=Path::new(a["path"].as_str().ok_or_else(||bad("independent A path"))?);
    let r:binary::Value=read(report)?;
    if a["policy"]!=digest(s)?||a["hash"]!=file_hash(report)?
        ||r["contract"]!=CONTRACT||r["policy"]!=digest(s)?||r["source"]!=s.source||r["verdict"]!="PASS" {
        return Err(bad("independent A missing/mismatched"));
    }Ok(())
}
fn indices(v:&binary::Value,key:&str,n:usize)->Result<Vec<usize>> {
    let out=v[key].as_array().ok_or_else(||bad("diagnostic manifest indices"))?.iter().map(|x|x.as_u64().map(|n|n as usize).ok_or_else(||bad("diagnostic index"))).collect::<Result<Vec<_>>>()?;
    if out.len()!=n{return Err(bad("diagnostic manifest cardinality"));}Ok(out)
}
fn diagnostic_cases(s:&Study,p:&StudyParent,tok:&ByteBpe)->Result<Vec<(String,Episode,Sample,binary::Value)>> {
    let(c,_,_,_,train)=inputs(p)?;
    let qa=identifiable::binding::citation::bridge_old_qa(&p.old_qa,false)?;
    let qa_samples=samples_with_framing(&qa.1,tok,2048,Framing::QuestionEvidence)?;
    let mut out=Vec::with_capacity(256);
    // Keep the original panel order, but select only frozen metadata positions.
    let dev=samples_with_framing(&c.validation,tok,2048,Framing::QuestionEvidence)?;
    for (name,es,ss,ix,panel,shift) in [
        ("train",c.train.as_slice(),train.as_slice(),indices(&s.diagnostic,"train_indices",64)?,"train192",WORD_START),
        ("word",c.validation.as_slice(),dev.as_slice(),indices(&s.diagnostic,"word_indices",64)?,"word",3072),
        ("renamed",c.validation.as_slice(),dev.as_slice(),indices(&s.diagnostic,"renamed_indices",64)?,"renamed",3264),
        ("qa",qa.1.as_slice(),qa_samples.as_slice(),indices(&s.diagnostic,"qa_indices",64)?,"qa-old_qa-primary",0),
    ] {
        let rows=binary::read_value_records(&s.parent_root.join("TRPP").join(format!("eval-3072-{panel}.r3rows")))?;
        for i in ix{let e=es.get(i).ok_or_else(||bad("diagnostic case index"))?;
            let sample=ss.get(i).ok_or_else(||bad("diagnostic sample index"))?;
            let raw=rows.get(i-shift+1).ok_or_else(||bad("diagnostic parent raw missing"))?;
            if raw["id"]!=e.id||raw["expected"]!=e.answer {return Err(bad("diagnostic raw identity"));}
            out.push((name.into(),e.clone(),sample.clone(),raw.clone()));
        }
    }
    if out.len()!=256{return Err(bad("diagnostic total"));}Ok(out)
}
fn nll(row:&[f32],gold:usize)->Result<f64>{
    if gold>=row.len()||row.iter().any(|v|!v.is_finite()){return Err(bad("teacher nonfinite/token bound"));}
    let max=row.iter().copied().fold(f32::NEG_INFINITY,f32::max) as f64;
    let denom=row.iter().map(|v|((*v as f64)-max).exp()).sum::<f64>().ln();
    Ok(max+denom-row[gold] as f64)
}
fn diagnostic_role(answer:&str,tok:&ByteBpe,gold:&[u32],j:usize)->Result<&'static str>{
    if j+1==gold.len(){return Ok("EOS");}
    let Some((value,after))=answer.split_once("입니다. [event:") else{return Ok("OTHER")};
    let id=after.strip_suffix(']').ok_or_else(||bad("diagnostic answer suffix"))?;
    let prefix=tok.decode_bytes(&gold[..j])?.len();let end=tok.decode_bytes(&gold[..=j])?.len();
    let v=value.len();let id_start=v+"입니다. [event:".len();let id_end=id_start+id.len();
    Ok(if end<=v{"VALUE"}else if prefix>=id_start&&end<=id_end{"ID"}
        else if prefix>=v&&end<=id_start||prefix>=id_end{"FORMAT"}else{"MIXED"})
}
fn greedy_diagnostic(e:&Episode,raw:&binary::Value,gold:&[u32],tok:&ByteBpe)->Result<binary::Value>{
    verify_generated(raw,tok)?;
    let emitted:Vec<u32>=binary::from_value(raw["raw_tokens"].clone())?;
    let first=(0..gold.len().max(emitted.len())).find(|&j|gold.get(j)!=emitted.get(j));
    let text=raw["actual"].as_str();
    let outside=text.is_some_and(|v|identifiable::binding::citation::individually_valid_ids(v).iter()
        .any(|id|!e.request.evidence.items.iter().any(|record|record.event_id==*id)));
    let completed=raw["generation_completed"]==true;
    let eos=completed&&raw["finish_reason"]=="stop"&&emitted.last()==Some(&EOS);
    let length=raw["finish_reason"]=="length";
    let runtime=!raw["error"].is_null()&&raw["error_class"]!="strict_utf8";
    let utf8=raw["error_class"]=="strict_utf8";
    let expects_citation=e.answer.contains("[event:");
    let parsed=text.and_then(|t|t.split_once("입니다. [event:")).and_then(|(v,rest)|rest.strip_suffix(']')
        .and_then(|id|id.parse::<i64>().ok().map(|id|(v,id))));
    let malformed=text.is_none()||expects_citation&&parsed.is_none();
    let wrong_kind=if let(Ok((gold_value,gold_id)),Some((value,id)))=(word_answer(e),parsed){
        if value!=gold_value {"VALUE"}else if id!=gold_id {"SUPPORT"}else{"OTHER"}
    }else{"OTHER"};
    let kind=if runtime{"RUNTIME"}else if utf8{"UTF8"}else if length{"LENGTH"}else if !eos{"EOS"}
        else if outside{"OUTSIDE_ID"}else if malformed{"MALFORMED"}else if text==Some(&e.answer){"CORRECT"}else{wrong_kind};
    Ok(binary::record!({"first_wrong_token":first,"gold_token":first.and_then(|j|gold.get(j).copied()),
        "emitted_token":first.and_then(|j|emitted.get(j).copied()),"type":kind,"outside_id":outside,
        "malformed":malformed,"eos":eos,"length":length,"runtime":runtime,"strict_utf8":utf8,
        "raw_tokens":emitted.len(),"generation_completed":completed}))
}
fn cache_parity(m:&Transformer,sample:&Sample,full:&[Vec<f32>],device:&Device,scope:&str)->Result<f64>{
    let mut cache=m.cache(scope);let prompt=&sample.tokens[..sample.response_start];
    let mut current=m.forward_cached(&Tensor::from_vec(prompt.to_vec(),(1,prompt.len()),device)?,&mut cache,scope)?;
    let mut max=0f64;let gold=&sample.tokens[sample.response_start..];
    for (j,&token) in gold.iter().enumerate(){
        let last=current.dim(1)?-1;
        let got=current.narrow(1,last,1)?.reshape((current.dim(2)?,))?.to_vec1::<f32>()?;
        let reference=&full[sample.response_start+j-1];
        if got.len()!=reference.len(){return Err(bad("cached vocab dimension"));}
        for(&a,&b)in got.iter().zip(reference){max=max.max((a as f64-b as f64).abs());}
        if j+1<gold.len(){current=m.forward_cached(&Tensor::from_vec(vec![token],(1,1),device)?,&mut cache,scope)?;}
    }
    Ok(max)
}
fn diagnose(root:&Path)->Result<()> {
    let command_started=Instant::now();
    let s=checked(root)?;admitted(&s)?;
    let output=s.root.join("diagnostic.r3b");if output.exists(){return Err(bad("diagnostic already completed; no duplicate teacher"));}
    let p:StudyParent=read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let tok=ByteBpe::load(&p.tokenizer)?;let cases=diagnostic_cases(&s,&p,&tok)?;
    let device=Backend::Metal0.open()?;s.runtime.verify(&device)?;
    compatible_parent_runtime(&s.parent_endpoint.state.runtime,&s.runtime)?;
    let(core,_)=load_endpoint(&s.parent_endpoint,device.clone())?;
    let Core::Tr(m)=core else{return Err(bad("diagnostic TR parent"))};
    let cancel=Arc::new(AtomicBool::new(false));let flag=cancel.clone();
    ctrlc::set_handler(move||flag.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let remaining=900f64-command_started.elapsed().as_secs_f64();
    if remaining<=0.{return Err(bad("diagnostic model-work command budget"));}
    let mut ctl=RunControl::new(cancel,Duration::from_secs_f64(remaining),16*1024*1024)?;ctl.set_call_limits(0,1024);
    let binding=binary::record!({"policy":digest(&s)?,"parent":s.parent_endpoint.hash,"cases":digest(&cases.iter().map(|(_,e,_,_)|e.id.clone()).collect::<Vec<_>>())?,"teacher_only":true});
    let path=s.root.join("diagnostic.r3rows");let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&path)?;
    append_row(&mut file,&binding)?;let mut observed=0usize;let mut higher=0usize;let mut max_cache=0f64;
    let(mut token_sum,mut token_count)=(0f64,0usize);let mut greedy_types=BTreeMap::<String,usize>::new();
    for (i,(name,e,sample,raw))in cases.iter().enumerate(){
        ctl.check("before_binding_teacher")?;
        let entered=prepare_call(&s.root,"diagnostic","teacher",&binding,e,i)?;
        ctl.begin_teacher()?;
        let b=batch(std::slice::from_ref(sample),&[0],&device)?;
        let logits=m.forward(&b.input,Some(&b.valid))?;device.synchronize()?;
        let matrix=logits.to_vec3::<f32>()?.remove(0);let gold=&sample.tokens[sample.response_start..];
        let mut sum=0f64;let mut roles=BTreeMap::<String,(f64,usize)>::new();let mut first=None;
        for (j,&token)in gold.iter().enumerate(){let row=&matrix[sample.response_start+j-1];let loss=nll(row,token as usize)?;sum+=loss;
            let role=diagnostic_role(&e.answer,&tok,gold,j)?.to_string();let x=roles.entry(role).or_default();x.0+=loss;x.1+=1;
            let top=row.iter().enumerate().max_by(|a,b|a.1.total_cmp(b.1)).ok_or_else(||bad("empty logits"))?.0 as u32;
            if top!=token&&first.is_none(){first=Some(j);}
        }
        let pairs=if name=="train"||name=="word"||name=="renamed"{word_branches(e,sample,&tok)?}else{vec![]};
        let margins=pairs.iter().map(|b|{let r=&matrix[b.position];r[b.positive as usize]-r[b.negative as usize]}).collect::<Vec<_>>();
        let branches=pairs.iter().map(|b|{let r=&matrix[b.position];binary::record!({"position":b.position,
            "positive_token":b.positive,"negative_token":b.negative,"positive_logit":r[b.positive as usize],
            "negative_logit":r[b.negative as usize],"margin":r[b.positive as usize]-r[b.negative as usize]})}).collect::<Vec<_>>();
        if (name=="train"||name=="word")&&raw["actual"]!=e.answer && margins.iter().any(|&v|v<0.){higher+=1;}
        if (64..80).contains(&i){max_cache=max_cache.max(cache_parity(&m,sample,&matrix,&device,&format!("binding-cache-{i}"))?);}
        let greedy=greedy_diagnostic(e,raw,gold,&tok)?;
        *greedy_types.entry(greedy["type"].as_str().ok_or_else(||bad("greedy diagnostic type"))?.into()).or_default()+=1;
        token_sum+=sum;token_count+=gold.len();
        let mut row=binary::record!({"index":i,"id":e.id,"group":name,"raw_hash":digest(raw)?,"answer_ce":sum/gold.len()as f64,
            "token_ce_sum":sum,"target_tokens":gold.len(),"roles":roles,"first_teacher_error":first,
            "branch_margins":margins,"branches":branches,"greedy":greedy,"gold_prefix_only":true});
        row["attempt"]=binary::record!(entered.file_name().unwrap().to_string_lossy());
        append_row(&mut file,&row)?;resolve_call(&entered,Some(&row),&ctl)?;observed+=1;
        ctl.stop_result()?;
    }
    if max_cache>5e-4{return Err(bad("gold-prefix cached/full logit tolerance"));}
    ctl.seal_completed_no_call()?;
    publish_confirmed(&output,&binary::record!({"policy":digest(&s)?,"parent":s.parent_endpoint.hash,"raw":file_hash(&path)?,
        "teacher":observed,"generation":0,"optimizer":0,"first_branch_higher_on_failed":higher,"minimum":16,
        "token_weighted_ce":token_sum/token_count as f64,"greedy_types":greedy_types,
        "max_cached_difference":max_cache,"tolerance":5e-4,"seconds":command_started.elapsed().as_secs_f64(),"control":ctl.receipt(),
        "hypothesis_targeted":higher>=16}))?;
    println!("BINDING_DIAGNOSTIC teacher={observed}/256 new_generation=0 optimizer=0 first_branch_higher={higher} cached_max={max_cache} seconds={:.3}",command_started.elapsed().as_secs_f64());Ok(())
}
fn admit(root:&Path,review:&Path)->Result<()> {
    let s=checked(root)?;let r:binary::Value=read(review)?;
    if r["contract"]!=CONTRACT||r["policy"]!=digest(&s)?||r["source"]!=s.source||r["verdict"]!="PASS"{
        return Err(bad("independent A not accepted"));
    }
    publish_confirmed(&s.root.join("review-a.r3b"),&binary::record!({"policy":digest(&s)?,"path":review.canonicalize()?,"hash":file_hash(review)?}))
}
fn arm_state(s:&Study,arm:&str)->Result<ComparisonState>{
    let mut st=s.parent_endpoint.state.clone();
    st.runtime=s.runtime.clone();
    st.policy=arm_policy(s,arm)?;st.config.lr=s.lr;st.config.warmup=0;st.config.max_steps=3584;
    st.objective=if arm=="C"{checkpoint::ANSWER_MEAN_FAMILY}else{8};
    Ok(st)
}
fn initial(s:&Study,arm:&str)->Result<Endpoint>{
    let path=s.root.join(arm).join("initial.r3b");
    if path.exists(){let ep:Endpoint=read_confirmed(&path)?;
        if ep.state!=arm_state(s,arm)?||file_hash(&ep.path)?!=ep.hash{return Err(bad("binding fork initial identity"));}return Ok(ep);
    }
    let dir=s.root.join(arm);std::fs::create_dir(&dir)?;
    let device=Backend::Metal0.open()?;s.runtime.verify(&device)?;
    compatible_parent_runtime(&s.parent_endpoint.state.runtime,&s.runtime)?;
    let(mut core,adam)=load_endpoint(&s.parent_endpoint,device)?;
    if !matches!(core,Core::Tr(_)){return Err(bad("binding parent must be TR++"));}
    let ep=save_endpoint(&dir,&mut core,&adam,arm_state(s,arm)?,"initial")?;
    publish_confirmed(&path,&ep)?;Ok(ep)
}
fn runs(s:&Study,arm:&str)->Result<Vec<Run>>{
    let mut out=vec![];
    let initial_path=s.root.join(arm).join("initial.r3b");
    if !initial_path.exists(){return Ok(out);}
    let mut previous=initial(s,arm)?;
    for index in 0..16 {let path=s.root.join(arm).join(format!("segment-{index:03}-returned.r3b"));
        let entered=s.root.join(arm).join(format!("segment-{index:03}-entered.r3b"));
        if !path.exists(){if entered.exists()||s.root.join(arm).join(format!("segment-{index:03}-trace.r3rows")).exists(){
            return Err(bad("UNKNOWN training segment has no returned usage receipt"));}break;}
        let r:Run=read_confirmed(&path)?;
        let marker:binary::Value=read_confirmed(&entered)?;
        if marker["policy"]!=arm_policy(s,arm)?||marker["before"]!=binary::record!(&r.before)||marker["index"]!=index{
            return Err(bad("training entry/return mismatch"));
        }
        if r.policy!=arm_policy(s,arm)?||r.arm!=arm||r.before!=previous||r.after.state.policy!=r.policy
            ||r.after.state.adam_clock!=r.after.state.committed||r.after.state.committed!=previous.state.committed+r.committed
            ||r.attempted<r.committed||r.attempted>528||r.committed>512
            ||r.trace_hash!=file_hash(&r.trace)?||r.after.hash!=file_hash(&r.after.path)?||!r.seconds.is_finite()||r.seconds<0.
            ||!["COMPLETED","TIME_BUDGET"].contains(&r.stop.as_str())
            ||r.stop=="TIME_BUDGET"&&r.control["observed_conditions"]!=binary::record!(["TIME_BUDGET"]) {return Err(bad("binding run segment integrity"));}
        previous=r.after.clone();out.push(r);
    }Ok(out)
}
fn model_seconds(s:&Study)->Result<f64>{
    let a:binary::Value=read_confirmed(&s.root.join("review-a.r3b"))?;
    let report:binary::Value=read(Path::new(a["path"].as_str().ok_or_else(||bad("A report path"))?))?;
    let independent=report["model_work_seconds"].as_f64().ok_or_else(||bad("A model-work seconds absent"))?;
    let diagnostic=if s.root.join("diagnostic.r3b").exists(){
        let d:binary::Value=read_confirmed(&s.root.join("diagnostic.r3b"))?;
        d["seconds"].as_f64().ok_or_else(||bad("diagnostic seconds absent"))?
    }else{0.};
    let runs=runs(s,"C")?.into_iter().chain(runs(s,"T")?).map(|r|r.seconds).sum::<f64>();
    let evaluations=[("C",256),("C",512),("T",256),("T",512),("C",999),("T",999),("C",998),("T",998)].into_iter()
        .map(|(a,n)|eval_runs(s,a,n).map(|v|v.into_iter().map(|x|x.seconds).sum::<f64>()))
        .sum::<Result<f64>>()?;
    let prior_tiny=s.budget["prior_tiny_reserved_model_seconds"].as_f64().ok_or_else(||bad("prior tiny model-work seconds"))?;
    let prior_failed=s.budget["prior_failed_diagnosis_reserved_seconds"].as_f64().ok_or_else(||bad("failed diagnosis model-work seconds"))?;
    let total=prior_tiny+prior_failed+independent+diagnostic+runs+evaluations;
    if !total.is_finite()||total<0.{return Err(bad("model-work time ledger"));}Ok(total)
}
fn eval_runs(s:&Study,arm:&str,step:usize)->Result<Vec<EvalRun>>{
    let mut out=vec![];
    for i in 0..16 {let path=s.root.join(arm).join(format!("eval-{step}-segment-{i:03}-returned.r3b"));
        let entered=s.root.join(arm).join(format!("eval-{step}-segment-{i:03}-entered.r3b"));
        if !path.exists(){if entered.exists(){return Err(bad("UNKNOWN evaluation segment has no returned usage receipt"));}break;}
        let r:EvalRun=read_confirmed(&path)?;
        let marker:binary::Value=read_confirmed(&entered)?;
        if marker["policy"]!=arm_policy(s,arm)?||marker["native"]!=r.native||marker["step"]!=step||marker["index"]!=i{
            return Err(bad("evaluation entry/return mismatch"));
        }
        if r.policy!=arm_policy(s,arm)?||r.arm!=arm||r.step!=step||r.native.len()!=64||!r.seconds.is_finite()||r.seconds<0.
            ||r.calls>4096||r.tokens>524288||!["COMPLETED","TIME_BUDGET"].contains(&r.stop.as_str())
            ||r.stop=="TIME_BUDGET"&&r.control["observed_conditions"]!=binary::record!(["TIME_BUDGET"]) {
            return Err(bad("binding evaluation receipt"));
        }out.push(r);
    }Ok(out)
}
fn eval_panels(s:&Study,p:&StudyParent)->Result<Vec<Panel>>{
    let mut out=vec![];
    let all=panels(p,3072,false)?;
    for (name,n)in [("word",192),("renamed",192),("train192",192),("value",64),("citation",64),("S1Q1",64)]{
        let source=all.iter().find(|v|v.0==name).ok_or_else(||bad("parent final panel missing"))?;
        out.push((name.into(),source.1[..n].to_vec(),source.2[..n].to_vec()));
    }
    let qa=all.iter().find(|v|v.0=="qa-old_qa-primary").ok_or_else(||bad("parent QA panel missing"))?;
    let ix=indices(&s.diagnostic,"qa_indices",64)?;
    out.push(("qa-old_qa-primary64".into(),ix.iter().map(|&i|qa.1[i].clone()).collect(),ix.iter().map(|&i|qa.2[i].clone()).collect()));
    if out.iter().map(|p|p.1.len()).sum::<usize>()!=832{return Err(bad("binding evaluation832"));}
    Ok(out)
}
fn generation_usage(s:&Study)->Result<(usize,usize)>{
    let mut calls=0;let mut tokens=0;
    for arm in ["C","T"]{for step in [256,512,999,998]{for r in eval_runs(s,arm,step)?{calls+=r.calls;tokens+=r.tokens;}}}
    Ok((calls,tokens))
}
fn scored(s:&Study,arm:&str,step:usize,name:&str)->Result<binary::Value>{
    let p=s.root.join(arm).join(format!("eval-{step}-{name}-score.r3b"));
    let score:binary::Value=read_confirmed(&p)?;
    if score["policy"]!=arm_policy(s,arm)?{return Err(bad("binding score policy"));}
    Ok(score)
}
fn score_count(score:&binary::Value,key:&str)->Result<usize>{
    score[key].as_u64().map(|n|n as usize).ok_or_else(||bad(&format!("binding score {key}")))
}
fn joint_count(score:&binary::Value,key:&str)->Result<usize>{score_count(&score["joint"],key)}
fn invalid_citation_rows(name:&str,score:&binary::Value)->Result<usize>{
    if name=="citation"||name=="S1Q1"{score_count(score,"valid_outside_id")}
    else if name.starts_with("qa-"){score_count(score,"outside_id")}else{Ok(0)}
}
fn paired_counts(c:&binary::Value,t:&binary::Value,key:&str)->Result<(usize,usize)>{
    let a=c[key].as_array().ok_or_else(||bad("paired C exact"))?;
    let b=t[key].as_array().ok_or_else(||bad("paired T exact"))?;
    if a.len()!=b.len(){return Err(bad("paired exact denominator"));}
    Ok((a.iter().zip(b).filter(|(x,y)|**x==false&&**y==true).count(),
        a.iter().zip(b).filter(|(x,y)|**x==true&&**y==false).count()))
}
fn report(root:&Path)->Result<()> {
    let s=checked(root)?;admitted(&s)?;
    let mut common=0usize;
    for step in [256,512]{if ["C","T"].iter().all(|arm|s.root.join(arm).join(format!("eval-{step}-complete.r3b")).exists()){
        common=step;
    }}
    if common==0{return Err(bad("no common complete scheduled endpoint"));}
    let(mut scores,mut signal)=(BTreeMap::new(),common==512);
    for name in ["word","renamed","train192","value","citation","S1Q1","qa-old_qa-primary64"]{
        let c=scored(&s,"C",common,name)?;let t=scored(&s,"T",common,name)?;
        let (cf,tf)=if name.starts_with("qa-"){(score_count(&c,"full")?,score_count(&t,"full")?)}
            else{(joint_count(&c,"full")?,joint_count(&t,"full")?)};
        let invalid_citation=invalid_citation_rows(name,&t)?;
        signal &= invalid_citation==0;
        let paired=if name=="word"||name=="renamed"{
            let(gain,loss)=paired_counts(&c["joint"],&t["joint"],"exact")?;
            let co=joint_count(&c,"all4")?;let to=joint_count(&t,"all4")?;
            let co_out=score_count(&c,"valid_outside_id")?;let to_out=score_count(&t,"valid_outside_id")?;
            let mal=score_count(&t,"parse_failure_rows")?;
            signal &= tf>=cf+20&&to>=co+4&&to_out<=co_out&&mal==0;
            binary::record!({"gain":gain,"loss":loss,"C_all4":co,"T_all4":to,
                "C_outside_id":co_out,"T_outside_id":to_out,"T_malformed":mal})
        }else{signal &= if name=="train192"{tf>=cf}else{tf+1>=cf};binary::record!(null)};
        scores.insert(name.to_string(),binary::record!({"C_full":cf,"T_full":tf,"paired":paired,
            "T_invalid_citation_rows":invalid_citation,
            "C_hash":file_hash(&s.root.join("C").join(format!("eval-{common}-{name}-score.r3b")))?,
            "T_hash":file_hash(&s.root.join("T").join(format!("eval-{common}-{name}-score.r3b")))?}));
    }
    let(calls,tokens)=generation_usage(&s)?;let seconds=model_seconds(&s)?;
    signal &= calls<=4096&&tokens<=524288&&seconds<=3600.&&new_bytes(&s.root)?<=1073741824;
    let result=binary::record!({"contract":CONTRACT,"policy":digest(&s)?,"common_step":common,"full_study_complete":common==512,
        "screening_only":true,"FOLLOWUP_SIGNAL":signal,"MODEL_QUALITY_ACCEPTED":false,"GOAL1_ACCEPTED":false,
        "scores":scores,"generation_calls":calls,"generated_tokens":tokens,"model_work_seconds":seconds,
        "artifact_bytes":new_bytes(&s.root)?});
    let path=s.root.join(format!("report-{common}.r3b"));
    if path.exists(){if read_confirmed::<binary::Value>(&path)?!=result{return Err(bad("binding report changed"));}}
    else{publish_confirmed(&path,&result)?;}
    println!("BINDING_REPORT common={common} followup_signal={signal} calls={calls} tokens={tokens} seconds={seconds:.3}");Ok(())
}
fn evaluate(root:&Path,arm:&str,step:usize)->Result<()> {
    let command_started=Instant::now();let s=checked(root)?;admitted(&s)?;
    if !["C","T"].contains(&arm)||![256,512,998,999].contains(&step){return Err(bad("binding eval endpoint"));}
    let confirmation=step==999;let reproduce=step==998;
    if confirmation{
        let decision:binary::Value=read_confirmed(&s.root.join("report-512.r3b"))?;
        if decision["policy"]!=digest(&s)?||decision["FOLLOWUP_SIGNAL"]!=true||decision["common_step"]!=512{
            return Err(bad("confirmation requires same-512 followup signal"));
        }
    }
    if reproduce{let decision:binary::Value=read_confirmed(&s.root.join("report-512.r3b"))?;
        if decision["policy"]!=digest(&s)?||decision["common_step"]!=512{return Err(bad("B reproduction requires completed same-512 comparison"));}}
    let lock=std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(s.root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("binding heavy process already active"))?;
    let rs=runs(&s,arm)?;let ep=rs.last().ok_or_else(||bad("evaluation before training"))?.after.clone();
    if ep.state.committed!=3072+if confirmation||reproduce{512}else{step}{return Err(bad("evaluation unscheduled/unequal checkpoint"));}
    let dir=s.root.join(arm);if dir.join(format!("eval-{step}-complete.r3b")).exists(){return Err(bad("evaluation already completed"));}
    let prior=eval_runs(&s,arm,step)?;let(index,used)=(prior.len(),model_seconds(&s)?);
    let(calls,tokens)=generation_usage(&s)?;
    if used>=3600.||calls>=4096||tokens>=524288||new_bytes(&s.root)?>1073741824{return Err(bad("binding evaluation budget exhausted"));}
    let p:StudyParent=read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let cases=if confirmation{
        let (es,ms):(Vec<Episode>,Vec<Meta>)=read_confirmed(&s.root.join("sealed-confirmation.r3b"))?;
        vec![("confirmation".into(),es,ms)]
    }else if reproduce{let word=eval_panels(&s,&p)?.into_iter().find(|p|p.0=="word").ok_or_else(||bad("B word panel"))?;
        vec![("review-word8".into(),word.1[..8].to_vec(),word.2[..8].to_vec())]
    }else{eval_panels(&s,&p)?};let tok=ByteBpe::load(&p.tokenizer)?;
    publish_confirmed(&dir.join(format!("eval-{step}-segment-{index:03}-entered.r3b")),
        &binary::record!({"policy":arm_policy(&s,arm)?,"native":ep.hash,"step":step,"index":index,
            "budget_calls_before":calls,"budget_tokens_before":tokens,"budget_seconds_before":used}))?;
    let device=Backend::Metal0.open()?;ep.state.runtime.verify(&device)?;
    let(core,_)=load_endpoint(&ep,device)?;
    let cancel=Arc::new(AtomicBool::new(false));let flag=cancel.clone();
    ctrlc::set_handler(move||flag.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let remaining=(3600.-used-command_started.elapsed().as_secs_f64()).min(900.);
    if remaining<=0.{return Err(bad("binding evaluation no model-work time"));}
    let mut ctl=RunControl::new(cancel,Duration::from_secs_f64(remaining),16*1024*1024)?;
    ctl.set_call_limits(4096-calls,0);let mut generated=0usize;
    let result=(||->Result<()> {for panel in &cases {
        let label=format!("eval-{step}-{}",panel.0);
        let original_hash=if reproduce{Some(file_hash(&dir.join("eval-512-word.r3rows"))?)}else{None};
        let binding=binary::record!({"policy":arm_policy(&s,arm)?,"arm":arm,"native":ep.hash,"model":ep.content,
            "step":step,"cases":digest(&panel.1)?,"metadata":digest(&panel.2)?,"tokenizer":s.tokenizer,
            "source_raw":original_hash,"accuracy_denominator":!reproduce});
        let rows=collect(&core,&tok,&dir,&label,&panel.1,&binding,&mut ctl,&mut generated,524288-tokens)?;
        if reproduce{
            let original=binary::read_value_records(&dir.join("eval-512-word.r3rows"))?;
            if original.len()!=193{return Err(bad("B source raw incomplete"));}
            for (a,b)in rows.iter().zip(original.iter().skip(1)){
                for field in ["raw_tokens","raw_bytes","actual","finish_reason","error","generation_completed"]{
                    if a[field]!=b[field]{return Err(bad("B fresh-process raw mismatch"));}
                }
            }
        }
        let raw=file_hash(&dir.join(format!("{label}.r3rows")))?;
        let mut score=panel_score(panel,&rows,&tok,&ep.content,&raw)?;
        score["policy"]=binary::record!(arm_policy(&s,arm)?);
        let path=dir.join(format!("{label}-score.r3b"));
        if path.exists(){if read_confirmed::<binary::Value>(&path)?!=score{return Err(bad("binding score mismatch"));}}
        else{publish_confirmed(&path,&score)?;}
    }ctl.seal_completed_no_call()})();
    if let Err(e)=&result{ctl.classify_error(e);}
    let stop=ctl.reason().unwrap_or("COMPLETED").to_string();
    let r=EvalRun{policy:arm_policy(&s,arm)?,arm:arm.into(),step,native:ep.hash.clone(),seconds:command_started.elapsed().as_secs_f64(),
        calls:ctl.receipt()["generation_calls"].as_u64().unwrap_or(0)as usize,tokens:generated,stop:stop.clone(),control:ctl.receipt()};
    publish_confirmed(&dir.join(format!("eval-{step}-segment-{index:03}-returned.r3b")),&r)?;
    if result.is_ok(){publish_confirmed(&dir.join(format!("eval-{step}-complete.r3b")),&binary::record!({"policy":arm_policy(&s,arm)?,
        "native":ep.hash,"panels":cases.len(),"rows":if confirmation{128}else if reproduce{8}else{832},
        "accuracy_denominator":!reproduce,"stop":"COMPLETED","generation_calls":generation_usage(&s)?.0}))?;}
    println!("BINDING_EVAL arm={arm} step={step} new_calls={} new_tokens={} seconds={:.3} stop={stop}",r.calls,r.tokens,r.seconds);
    result
}
fn confirmation_report(root:&Path)->Result<()> {
    let s=checked(root)?;let decision:binary::Value=read_confirmed(&s.root.join("report-512.r3b"))?;
    if decision["policy"]!=digest(&s)?||decision["FOLLOWUP_SIGNAL"]!=true{return Err(bad("unqualified confirmation"));}
    for arm in ["C","T"]{read_confirmed::<binary::Value>(&s.root.join(arm).join("eval-999-complete.r3b"))?;}
    let c=scored(&s,"C",999,"confirmation")?;let t=scored(&s,"T",999,"confirmation")?;
    let(cf,tf)=(joint_count(&c,"full")?,joint_count(&t,"full")?);
    let(co,to)=(joint_count(&c,"all4")?,joint_count(&t,"all4")?);
    let(cout,tout)=(score_count(&c,"valid_outside_id")?,score_count(&t,"valid_outside_id")?);
    let consistent=tf>=cf+8&&to>=co+2&&tout<=cout;
    let result=binary::record!({"policy":digest(&s)?,"C_full":cf,"T_full":tf,"C_all4":co,"T_all4":to,
        "C_outside_id":cout,"T_outside_id":tout,"CONFIRM_DIRECTION_CONSISTENT":consistent,
        "MODEL_QUALITY_ACCEPTED":false,"GOAL1_ACCEPTED":false,"generation":generation_usage(&s)?,"model_seconds":model_seconds(&s)?});
    publish_confirmed(&s.root.join("confirmation-report.r3b"),&result)?;
    println!("BINDING_CONFIRMATION full={cf}/{tf} all4={co}/{to} outside={cout}/{tout} consistent={consistent}");Ok(())
}
fn tiny(root:&Path,until:usize,resume:bool)->Result<()> {
    if ![1,2].contains(&until){return Err(bad("tiny bound2"));}
    let root=if resume{root.canonicalize()?}else{std::fs::create_dir(root)?;root.canonicalize()?};
    let device=Backend::Metal0.open()?;let runtime=RuntimeProfile::capture(Backend::Metal0,&device)?;
    let policy=digest(&(CONTRACT,source()?,RULE,LAMBDA,"TINY_V1"))?;
    let tok=neural::hash(b"binding-recovery-tiny-token-scope");
    let mut cfg=super::config();cfg.lr=3e-4;cfg.warmup=0;cfg.max_steps=2;cfg.seq_len=64;cfg.microbatch=1;
    let mut core;let mut adam;let mut st;
    if resume {
        let index=if root.join("segment-1.r3b").exists(){1}else{return Err(bad("tiny missing step1"))};
        let ep:Endpoint=read_confirmed(&root.join(format!("segment-{index}.r3b")))?;
        if ep.state.policy!=policy||ep.state.tokenizer!=tok||ep.state.committed!=index||ep.state.objective!=8||ep.state.runtime!=runtime{return Err(bad("tiny resume policy/runtime/clock"));}
        (core,adam)=load_endpoint(&ep,device.clone())?;st=ep.state;
    }else{
        let m=Transformer::init(Config::tiny(264),20260926,device.clone())?;core=Core::Tr(m);core.bind(&tok)?;
        adam=Adam::new(core.vars())?;
        st=ComparisonState{schema:1,core:core.descriptor(),policy:policy.clone(),tokenizer:tok.clone(),framing:Framing::QuestionEvidence.digest(),runtime,
            objective:8,optimizer:"FRESH_ADAMW_ALL_V1".into(),config:cfg.clone(),committed:0,adam_clock:0,input_tokens:0,target_tokens:0};
        let ep=save_endpoint(&root,&mut core,&adam,st.clone(),"initial")?;
        publish_confirmed(&root.join("initial.r3b"),&ep)?;
    }
    if until<=st.committed{return Err(bad("tiny next step"));}
    let sample=Sample{tokens:vec![BOS,10,11,12,EOS],response_start:3,curriculum:false};
    let samples=[sample];let pairs=vec![vec![Branch{position:2,positive:12,negative:13}]];
    let mut rows=vec![];
    while st.committed<until {
        let b=batch(&samples,&[0],&device)?;let logits=core.forward(&b)?;
        let(_,base,n,_)=response_objective(&logits,&b,1.,true)?;
        let loss=(base+branch_loss(&logits,&[WORD_START],&pairs,LAMBDA)?)?;
        let value=loss.to_scalar::<f32>()?;let graph=loss.backward()?;let mut grads=BTreeMap::new();
        for(name,var)in core.vars(){grads.insert(name.clone(),graph.get(var).ok_or_else(||bad("tiny missing gradient"))?.detach());}
        let clock=st.committed+1;adam.apply_admitted_step(core.vars(),&grads,&st.config,clock,cfg.lr,|_,_,_,_|Ok(()))?;
        device.synchronize()?;st.committed=clock;st.adam_clock=clock;st.input_tokens+=b.tokens as u64;st.target_tokens+=n as u64;
        rows.push(binary::record!({"clock":clock,"loss":value,"input":b.tokens,"target":n}));
        let ep=save_endpoint(&root,&mut core,&adam,st.clone(),&format!("segment-{clock}"))?;
        publish_confirmed(&root.join(format!("segment-{clock}.r3b")),&ep)?;
    }
    publish_confirmed(&root.join(format!("trace-{until}.r3b")),&binary::record!(rows))?;
    println!("BINDING_TINY clock={} input={} target={} native={}",st.committed,st.input_tokens,st.target_tokens,root.display());Ok(())
}
fn verify_tiny(continuous:&Path,split:&Path)->Result<()> {
    let a:Endpoint=read_confirmed(&continuous.join("segment-2.r3b"))?;
    let b:Endpoint=read_confirmed(&split.join("segment-2.r3b"))?;
    if a.state!=b.state||a.content!=b.content{return Err(bad("tiny state/weight mismatch"));}
    for wrong in ["policy","objective","tokenizer","clock"]{
        let mut expected=a.state.clone();
        match wrong{"policy"=>expected.policy=neural::hash(b"wrong policy"),"objective"=>expected.objective=checkpoint::ANSWER_MEAN_FAMILY,
            "tokenizer"=>expected.tokenizer=neural::hash(b"wrong tokenizer"),_=>expected.adam_clock+=1}
        if artifact::load_comparison(&a.path,&expected,&Device::Cpu).is_ok(){return Err(bad("tiny mismatched native accepted"));}
    }
    let(_,aa)=load_endpoint(&a,Device::Cpu)?;let(_,bb)=load_endpoint(&b,Device::Cpu)?;
    if aa.moments.len()!=bb.moments.len(){return Err(bad("tiny Adam names"));}
    for(name,t)in &aa.moments{if t.flatten_all()?.to_vec1::<f32>()?!=bb.moments[name].flatten_all()?.to_vec1::<f32>()?{return Err(bad("tiny Adam mismatch"));}}
    let c:binary::Value=read_confirmed(&continuous.join("trace-2.r3b"))?;
    let first:binary::Value=read_confirmed(&split.join("trace-1.r3b"))?;
    let second:binary::Value=read_confirmed(&split.join("trace-2.r3b"))?;
    let mut merged=first.as_array().ok_or_else(||bad("tiny first trace"))?.clone();
    merged.extend_from_slice(second.as_array().ok_or_else(||bad("tiny second trace"))?);
    if c.as_array()!=Some(&merged){return Err(bad("tiny continuous/restart loss trace mismatch"));}
    println!("BINDING_TINY_PARITY updates=2+1+1 weights=EXACT Adam=EXACT clock=2");Ok(())
}
fn train(root:&Path,arm:&str,until:usize)->Result<()> {
    let command_started=Instant::now();
    let s=checked(root)?;admitted(&s)?;
    let diagnosis:binary::Value=read_confirmed(&s.root.join("diagnostic.r3b"))?;
    if diagnosis["policy"]!=digest(&s)?||diagnosis["parent"]!=s.parent_endpoint.hash||diagnosis["teacher"]!=256
        ||diagnosis["hypothesis_targeted"]!=true{return Err(bad("fixed diagnosis did not support C/T study"));}
    if !["C","T"].contains(&arm)||![256,512].contains(&until){return Err(bad("binding arm/endpoint"));}
    let lock=std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(s.root.join("writer.lock"))?;
    lock.try_lock().map_err(|_|bad("binding heavy process already active"))?;
    let run=runs(&s,arm)?;let before=run.last().map_or(initial(&s,arm),|r|Ok(r.after.clone()))?;
    let start_step=before.state.committed-3072;
    if until<=start_step||until>512||start_step<256&&until!=256||start_step>=256&&until!=512{return Err(bad("binding next scheduled update"));}
    if start_step==256 {
        let score=s.root.join(arm).join("eval-256-complete.r3b");
        read_confirmed::<binary::Value>(&score)?;
    }
    let attempted_before=run.iter().map(|r|r.attempted).sum::<usize>();
    let elapsed_before=model_seconds(&s)?;
    if attempted_before>=528||elapsed_before>=2400. {return Err(bad("binding training budget exhausted"));}
    let parent_plan:StudyParent=read_confirmed(&s.parent_root.join("plan.r3b"))?;
    let(_,_,_,_,samples)=inputs(&parent_plan)?;
    let index=run.len();let dir=s.root.join(arm);
    publish_confirmed(&dir.join(format!("segment-{index:03}-entered.r3b")),
        &binary::record!({"policy":arm_policy(&s,arm)?,"before":before,"index":index,
            "attempted_before":attempted_before,"model_seconds_before":elapsed_before,"until":until}))?;
    let device=Backend::Metal0.open()?;s.runtime.verify(&device)?;
    compatible_parent_runtime(&s.parent_endpoint.state.runtime,&s.runtime)?;
    let(mut core,mut adam)=load_endpoint(&before,device)?;
    let cancel=Arc::new(AtomicBool::new(false));let flag=cancel.clone();
    ctrlc::set_handler(move||flag.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let remaining=(2400.-elapsed_before-command_started.elapsed().as_secs_f64()).min(900.);
    if remaining<=0.{return Err(bad("binding model-work training cutoff"));}
    let mut ctl=RunControl::new(cancel,Duration::from_secs_f64(remaining),16*1024*1024)?;
    ctl.set_call_limits(0,0);
    let trace=dir.join(format!("segment-{index:03}-trace.r3rows"));
    let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&trace)?;
    append_row(&mut file,&binary::record!({"policy":arm_policy(&s,arm)?,"before":before,"until":until,"index":index}))?;
    let mut st=before.state.clone();let mut backward=0usize;let mut in_optimizer=false;
    let result=(||->Result<()> {while st.committed<3072+until {
        ctl.check("before_binding_update")?;
        if attempted_before+backward>=528{return Err(bad("binding backward cap"));}
        let row=&s.tape[st.committed-3072];let b=batch(&samples,row,core.device())?;
        if st.input_tokens-s.parent_endpoint.state.input_tokens+b.tokens as u64>2_000_000{return Err(bad("binding input token cap"));}
        let logits=core.forward(&b)?;let(plain,base,n,_)=response_objective(&logits,&b,1.,true)?;
        if st.target_tokens-s.parent_endpoint.state.target_tokens+n as u64>300_000{return Err(bad("binding target token cap"));}
        let loss=if arm=="T"{(base+branch_loss(&logits,row,&s.pairs,LAMBDA)?)?}else{base};
        let value=loss.to_scalar::<f32>()?;if !value.is_finite(){return Err(bad("binding nonfinite loss"));}
        backward+=1;let graph=loss.backward()?;let mut grads=BTreeMap::new();
        for(name,var)in core.vars(){grads.insert(name.clone(),graph.get(var).ok_or_else(||bad("missing binding gradient"))?.detach());}
        core.device().synchronize()?;ctl.check("before_binding_optimizer")?;
        let clock=st.committed+1;in_optimizer=true;
        let(norm,delta)=adam.apply_admitted_step(core.vars(),&grads,&st.config,clock,s.lr,|_,_,_,_|Ok(()))?;
        core.device().synchronize()?;st.committed=clock;st.adam_clock=clock;st.input_tokens+=b.tokens as u64;st.target_tokens+=n as u64;in_optimizer=false;
        append_row(&mut file,&binary::record!({"commit":clock,"local":clock-3072,"input":b.tokens,"target":n,"loss":value,
            "plain_ce":plain.to_scalar::<f32>()?,"lr":s.lr,"gradient_norm":norm,"update_l2":delta,"batch":row}))?;
    }Ok(())})();
    if let Err(e)=&result{ctl.classify_error(e);}
    let stop=ctl.reason().unwrap_or("COMPLETED").to_string();
    let mut after=before.clone();
    if !in_optimizer&&st.committed>before.state.committed {
        after=save_endpoint(&dir,&mut core,&adam,st.clone(),&format!("segment-{index:03}"))?;
    }
    append_row(&mut file,&binary::record!({"phase":"RETURNED","after":after,"stop":stop}))?;
    let r=Run{policy:arm_policy(&s,arm)?,arm:arm.into(),before:before.clone(),after:after.clone(),attempted:backward,
        committed:after.state.committed-before.state.committed,input:after.state.input_tokens-before.state.input_tokens,
        target:after.state.target_tokens-before.state.target_tokens,seconds:command_started.elapsed().as_secs_f64(),stop:stop.clone(),
        trace:trace.clone(),trace_hash:file_hash(&trace)?,control:ctl.receipt()};
    publish_confirmed(&dir.join(format!("segment-{index:03}-returned.r3b")),&r)?;
    println!("BINDING_TRAIN arm={arm} local={} absolute={} backward={} input={} target={} seconds={:.3} stop={stop} native={} hash={}",
        after.state.committed-3072,after.state.committed,backward,r.input,r.target,r.seconds,after.path.display(),after.hash);
    result
}
// The existing gold-prefix forward supplies both logits. Neither alternative
// is forwarded or supplied to product generation.
fn branch_loss(logits:&Tensor,rows:&[usize],pairs:&[Vec<Branch>],lambda:f64)->Result<Tensor>{
    if !lambda.is_finite()||lambda<0.||rows.len()!=logits.dim(0)?{return Err(bad("branch loss batch/weight"));}
    let(_,len,vocab)=logits.dims3()?;let zero=Tensor::zeros((),DType::F32,logits.device())?;
    let mut per_row=Vec::with_capacity(rows.len());
    for(row,&index)in rows.iter().enumerate(){
        if index<WORD_START {per_row.push(zero.clone());continue;}
        let b=pairs.get(index-WORD_START).ok_or_else(||bad("branch word index"))?;
        if b.len()>2{return Err(bad("branch pair count"));}
        let mut seen=BTreeSet::new();let mut each=Vec::new();
        for pair in b {if pair.position>=len||pair.positive as usize>=vocab||pair.negative as usize>=vocab
            ||pair.positive==pair.negative||!seen.insert((pair.position,pair.positive,pair.negative)){return Err(bad("branch pair bounds/duplicate"));}
            let at=|token:u32| logits.narrow(0,row,1)?.narrow(1,pair.position,1)?.narrow(2,token as usize,1)?.reshape(());
            let d=(at(pair.negative)?-at(pair.positive)?)?;
            each.push(Tensor::stack(&[&zero,&d],0)?.log_sum_exp(0)?);
        }
        per_row.push(if each.is_empty(){zero.clone()}else{Tensor::stack(&each,0)?.mean_all()?});
    }
    Ok((Tensor::stack(&per_row,0)?.mean_all()?*lambda)?)
}
pub fn run(action:Action)->Result<()>{match action{
    Action::Prepare{parent,output}=>prepare(&parent,&output),
    Action::Inspect{root}=>inspect(&root),
    Action::Admit{root,review}=>admit(&root,&review),
    Action::Diagnose{root}=>diagnose(&root),
    Action::Train{root,arm,until}=>train(&root,&arm,until),
    Action::Evaluate{root,arm,step}=>evaluate(&root,&arm,step),
    Action::Confirm{root,arm}=>evaluate(&root,&arm,999),
    Action::ConfirmationReport{root}=>confirmation_report(&root),
    Action::Review{root,arm}=>evaluate(&root,&arm,998),
    Action::Report{root}=>report(&root),
    Action::Tiny{root,until,resume}=>tiny(&root,until,resume),
    Action::VerifyTiny{continuous,split}=>verify_tiny(&continuous,&split),
}}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn branch_loss_first_divergence_and_mean() -> Result<()> {
        let logits=Tensor::from_vec(vec![2f32,0.,0.,0., 0.,0.,2.,0.],(2,1,4),&Device::Cpu)?;
        let p=vec![vec![Branch{position:0,positive:0,negative:1}],vec![]];
        let l=branch_loss(&logits,&[WORD_START,WORD_START+1],&p,0.1)?.to_scalar::<f32>()? as f64;
        let expected=(1.+(-2f64).exp()).ln()*0.05;
        assert!((l-expected).abs()<1e-7);
        assert_eq!(branch_loss(&logits,&[WORD_START,WORD_START+1],&p,0.)?.to_scalar::<f32>()?,0.);
        assert!(branch_loss(&logits,&[WORD_START],&p,0.1).is_err());
        assert!(branch_loss(&logits,&[WORD_START,WORD_START+1],&[vec![p[0][0].clone(),p[0][0].clone()],vec![]],0.1).is_err());
        Ok(())
    }
    #[test]
    fn branch_loss_gradient_matches_independent_f64_oracle() -> Result<()> {
        let shape=(3,2,4);
        let mut values=(0..24).map(|i|(i as f32-11.)/5.).collect::<Vec<_>>();
        values[6]=1.5;values[7]=-0.5;
        let rows=[WORD_START,WORD_START+1,0];
        let pairs=vec![
            vec![Branch{position:1,positive:2,negative:3},Branch{position:0,positive:1,negative:0}],
            vec![Branch{position:1,positive:0,negative:2}],
        ];
        let oracle=|v:&[f32]|->f64{
            let mut total=0.;
            for (row,group) in pairs.iter().enumerate(){
                let mut answer=0.;
                for b in group{let at=|token:u32|v[row*8+b.position*4+token as usize] as f64;
                    let d=at(b.negative)-at(b.positive);
                    answer+=d.max(0.)+(-d.abs()).exp().ln_1p();
                }
                total+=answer/group.len() as f64;
            }
            total*0.1/rows.len() as f64
        };
        let x=Var::from_vec(values.clone(),shape,&Device::Cpu)?;
        let loss=branch_loss(x.as_tensor(),&rows,&pairs,0.1)?;
        assert!((loss.to_scalar::<f32>()? as f64-oracle(&values)).abs()<2e-7);
        let grad=loss.backward()?.get(&x).ok_or_else(||bad("missing branch gradient"))?.flatten_all()?.to_vec1::<f32>()?;
        for coordinate in [0,1,6,7,8,12,14,22]{
            let h=1e-3f32;
            values[coordinate]+=h;let upper=oracle(&values);
            values[coordinate]-=2.*h;let lower=oracle(&values);
            values[coordinate]+=h;
            let numerical=(upper-lower)/(2.*h as f64);
            assert!((grad[coordinate] as f64-numerical).abs()<2e-5,"coordinate {coordinate}");
        }
        assert!(grad[6]<0.&&grad[7]>0.,"descent must raise gold and lower foil");
        let no_branch=branch_loss(x.as_tensor(),&rows,&pairs,0.)?;
        assert_eq!(no_branch.to_scalar::<f32>()?,0.);
        let wrong=vec![vec![Branch{position:2,positive:2,negative:3}],vec![]];
        assert!(branch_loss(x.as_tensor(),&rows,&wrong,0.1).is_err());
        Ok(())
    }
    #[test]
    fn high_full_score_never_hides_outside_citation() -> Result<()> {
        let word=binary::record!({"joint":{"full":511},"valid_outside_id":1});
        let qa=binary::record!({"full":63,"outside_id":1});
        assert_eq!(invalid_citation_rows("citation",&word)?,1);
        assert_eq!(invalid_citation_rows("S1Q1",&word)?,1);
        assert_eq!(invalid_citation_rows("qa-old_qa-primary64",&qa)?,1);
        assert_eq!(invalid_citation_rows("value",&word)?,0);
        Ok(())
    }
    #[test]
    fn parent_runtime_fork_changes_binary_only() -> Result<()> {
        let old=RuntimeProfile{backend:Backend::Metal0,actual_device:"Metal(0)".into(),dtype:"F32".into(),
            accumulator:"F32".into(),patch:"same".into(),patch_source:"patch".into(),lock:"lock".into(),
            binary:"old-binary".into(),os_build:"same-OS".into(),fast_math:"UNKNOWN".into()};
        let mut current=old.clone();current.binary="new-binary".into();
        compatible_parent_runtime(&old,&current)?;
        current.patch_source="changed-patch".into();
        assert!(compatible_parent_runtime(&old,&current).is_err());
        Ok(())
    }
}
