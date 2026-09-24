//! One parent-bound q/v adaptation policy on the existing word execution path.
use super::*;
use neural::{artifact,transformer::QvAdapter};

pub(super) const CONTRACT:&str="R3-PROTECTED-ADAPTATION-PRECISION-1.0";
const DATA:&str="qa-word-adapter-v1";
const SEED:u64=20260924;
pub(in super::super::super::super) fn is(p:&Plan)->bool {p.identifiable.as_ref().is_some_and(|o|o.dataset==DATA)}
fn policy(old:&Plan,study:&Path,s:&binary::Value)->Result<Plan>{
    let state:TrainingState=binary::from_value(s["initial_state"].clone())?;
    let get=|k:&str|s[k].as_str().map(str::to_owned).ok_or_else(||bad("adapter policy field"));
    let mut p=old.clone();p.source=get("source")?;p.binary=get("binary")?;p.initial=get("initial")?;p.initial_weights=get("weights")?;
    p.config=state.config.clone();let o=p.identifiable.as_mut().ok_or_else(||bad("adapter word profile"))?;
    o.study=study.into();o.dataset=DATA.into();o.rows.truncate(p.config.max_steps);p.train_order=digest(&o.rows)?;
    let f=p.fork.as_mut().ok_or_else(||bad("adapter parent fork"))?;f.study=study.into();f.study_hash=file_hash(&study.join("selection.r3b"))?;
    f.parent_policy=hex(&state.resume_binding.as_ref().ok_or_else(||bad("adapter initial scope"))?.policy);
    f.parent_state=digest(&state)?;f.parent_adam=get("adam")?;f.origin_step=state.step;f.origin_input=state.consumed_tokens;f.origin_target=state.target_tokens;
    f.constant_lr=3e-4;f.target_limit=400_000;p.evaluation=super::evaluation(&p);Ok(p)
}
pub(in super::super::super::super) fn prepare(previous:&Path,output:&Path,tiny:bool)->Result<()> {
    if tiny!=cfg!(all(test,feature="test-support")){return Err(bad("adapter production/TINY boundary"));}
    let previous=previous.canonicalize()?;let old_root=previous.join(MEAN_ARMS[1]);let old=historical_plan(&old_root)?;
    if !word::is(&old)||is(&old)||old.tiny!=tiny {return Err(bad("adapter requires original word study"));}
    let prior:binary::Value=read(&previous.join("selection.r3b"))?;
    let parent=PathBuf::from(prior["parent"].as_str().ok_or_else(||bad("adapter original bridge parent"))?);
    let base_end:Segment=binary::from_value(prior["parent_endpoint"].clone())?;
    let parent_plan=historical_plan(&parent)?;let(end,d)=close(&parent,&parent_plan)?;bridge_diagnostic_admission(&parent_plan,&end,&d)?;
    if digest(&end)?!=digest(&base_end)? {return Err(bad("adapter original parent endpoint"));}
    let base_path=parent.join(&end.checkpoint).canonicalize()?;let physical=file_hash(&base_path)?;
    if !tiny&&(end.step!=14336||physical!="15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9") {return Err(bad("adapter exact base14336"));}
    // Inference load deliberately does not allocate the historical full Adam.
    let mut l=checkpoint::load(&base_path,Device::Cpu,false)?;
    let mut state=l.manifest.training.clone().ok_or_else(||bad("adapter parent state"))?;
    if state.step!=old.origin_step()||state.config!=parent_plan.config||state.resume_binding!=Some(parent_plan.binding(&state,&l.tokenizer)?) {return Err(bad("adapter base native lineage"));}
    let c=verified_corpus(&old_root.join("corpus.r3cor"),&old.corpus)?;let _=verified_metadata(&old_root,&old)?;
    let limit=if tiny{2}else{1024};let tape=own(&old).rows.get(state.step..state.step+limit).ok_or_else(||bad("adapter exact tape prefix"))?;
    let costs=qa_token_cost(&c.train,tape,&l.tokenizer)?;
    if c.train.len()!=7680||c.validation.len()!=3456||costs["input"].as_u64().is_none_or(|n|n>4_000_000)||costs["target"].as_u64().is_none_or(|n|n>400_000){return Err(bad("adapter data/token cap"));}
    let base=artifact::AdapterBase{path:base_path,physical,content:l.model.weights_content_id()?,architecture:l.model.config.semantic_id()?,tokenizer:l.tokenizer.id(),framing:l.manifest.framing()?.digest(),step:state.step};
    let descriptor=QvAdapter::registered(SEED);l.model.initialize_adapter(descriptor.clone())?;artifact::bind_adapter_base(&mut l.model,base.clone())?;
    state.config=old.config.clone();state.config.lr=3e-4;state.config.weight_decay=0.;state.config.warmup=0;
    state.config.max_steps=state.step+limit;state.config.budget_start_step=state.step;state.config.budget_start_tokens=state.consumed_tokens;state.config.max_tokens=state.consumed_tokens+4_000_000;
    state.parent_checkpoint_hash=Some(base.physical.clone());state.corpus_hash=c.manifest.train.sha256.clone();state.validation_hash=c.manifest.validation.sha256.clone();
    if state.corpus_hash!=l.tokenizer.train_hash&&!state.previous_corpora.contains(&l.tokenizer.train_hash){state.previous_corpora.push(l.tokenizer.train_hash.clone());}
    state.train_loss=None;state.validation_loss=None;
    let scope=binary::record!({"contract":CONTRACT,"source":source_digest()?,"base":base,"descriptor":descriptor,"tape":digest(&tape)?,"data":old.corpus,"config":state.config});
    let mut binding=checkpoint::ResumeBinding::default_for(&state,&l.tokenizer);binding.family=checkpoint::ANSWER_MEAN_FAMILY;binding.normalizer=2;binding.execution=1;binding.framing=base.framing;
    binding.policy=checkpoint::ResumeBinding::digest_bytes(&binary::to_vec(&scope)?);binding.provenance=binding.policy;
    binding.train_order=checkpoint::ResumeBinding::digest_bytes(digest(&tape)?.as_bytes());state.resume_binding=Some(binding);
    let mut adam=BTreeMap::new();for(n,v)in &l.model.vars {for prefix in ["adam.m.","adam.v."]{adam.insert(format!("{prefix}{n}"),Tensor::zeros(v.shape(),candle_core::DType::F32,&Device::Cpu)?);}}
    let output=std::path::absolute(output)?;std::fs::create_dir(&output)?;let root=output.join(MEAN_ARMS[1]);std::fs::create_dir(&root)?;
    let mut references=BTreeMap::new();for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b"] {
        let path=old_root.join(name).canonicalize()?;references.insert(name,(path.clone(),file_hash(&path)?));std::os::unix::fs::symlink(path,root.join(name))?;
    }
    l.manifest.source_id=source_digest()?;l.manifest.training=Some(state.clone());l.manifest.status="TRAINING".into();
    checkpoint::save(&root.join("initial.r3m"),&l.model,&l.tokenizer,l.manifest,&adam)?;
    let mut protected:BTreeMap<PathBuf,String>=binary::from_value(prior["parent_protected"].clone())?;
    for path in [old_root.join("plan.r3b"),previous.join("selection.r3b"),previous.join("preparation.r3b"),base.path.clone()]{protected.insert(path.clone(),file_hash(&path)?);}
    let s=binary::record!({"contract":CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"parent":parent,"parent_endpoint":end,
        "previous_word":old_root,"references":references,"parent_protected":protected,"base":base,"descriptor":descriptor,"initial_scope":scope,"initial_state":state,
        "initial":file_hash(&root.join("initial.r3m"))?,"weights":l.model.weight_hash()?,"adapter_content":l.model.weights_content_id()?,"adam":optimizer_hash(&adam)?,
        "trainable_parameters":l.model.vars.values().map(|v|v.elem_count()).sum::<usize>(),"adapter_bytes":l.model.vars.values().map(|v|v.elem_count()*4).sum::<usize>(),
        "adam_bytes":adam.values().map(|v|v.elem_count()*4).sum::<usize>(),"costs":costs,"tape_prefix":digest(&tape)?,"updates":limit,
        "artifact_limit_bytes":1073741824u64,"build_limit_bytes":8589934592u64,"historical_resume_unchanged":true,"historical_comparison":"different parameter space, Adam, LR and decay; not one-factor A/B"});
    write(&output.join("selection.r3b"),&s)?;let p=policy(&old,&output,&s)?;write(&root.join("plan.r3b"),&p)?;verify(&root,&p)?;
    let initial=checkpoint::load(&root.join("initial.r3m"),Device::Cpu,true)?;
    if !p.parent_entry(&root.join("initial.r3m"),&initial)?{return Err(bad("adapter explicit initial entry"));}
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":CONTRACT,"source":p.source,"binary":p.binary,"selection":file_hash(&output.join("selection.r3b"))?,
        "arms":{(MEAN_ARMS[1]):{"policy":file_hash(&root.join("plan.r3b"))?,"initial":p.initial,"corpus":p.corpus,"tape":p.train_order}},"optimizer":0,"generation":0,"teacher":0}))?;
    println!("ADAPTER_PREPARED base_step={} adapter_updates0 parameters={} weights_bytes={} Adam_bytes={} input={} target={} A_PENDING",p.origin_step(),s["trainable_parameters"],s["adapter_bytes"],s["adam_bytes"],s["costs"]["input"],s["costs"]["target"]);Ok(())
}
pub(super) fn verify(root:&Path,p:&Plan)->Result<()> {
    let study=&own(p).study;let s:binary::Value=read(&study.join("selection.r3b"))?;
    let old_root=Path::new(s["previous_word"].as_str().ok_or_else(||bad("adapter word reference"))?);let old:Plan=read(&old_root.join("plan.r3b"))?;
    let protected:BTreeMap<PathBuf,String>=binary::from_value(s["parent_protected"].clone())?;
    for(path,h)in protected{if file_hash(&path)?!=h{return Err(bad("adapter original changed"));}}
    let refs:BTreeMap<String,(PathBuf,String)>=binary::from_value(s["references"].clone())?;
    for name in ["corpus.r3cor","transfer.r3cor","metadata.r3b","tokenizer.r3b"]{let(path,h)=refs.get(name).ok_or_else(||bad("adapter missing reference"))?;
        if root.join(name).canonicalize()?!=*path||file_hash(path)?!=*h{return Err(bad("adapter frozen reference mismatch"));}}
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let _=verified_metadata(root,p)?;
    let tape=&own(p).rows[p.origin_step()..p.config.max_steps];let init=checkpoint::load(&root.join("initial.r3m"),Device::Cpu,true)?;
    let base:artifact::AdapterBase=binary::from_value(s["base"].clone())?;let descriptor:QvAdapter=binary::from_value(s["descriptor"].clone())?;
    if !is(p)||s["contract"]!=CONTRACT||*p!=policy(&old,study,&s)?||root!=study.join(MEAN_ARMS[1])||p.initial!=file_hash(&root.join("initial.r3m"))?
        ||p.tokenizer!=tok.id()||p.tokenizer!=old.tokenizer||p.corpus!=old.corpus||p.transfer!=old.transfer||p.metadata!=old.metadata
        ||s["costs"]!=qa_token_cost(&c.train,tape,&tok)?||s["tape_prefix"]!=digest(&tape)?||init.model.adapter()!=Some(&descriptor)
        ||descriptor!=QvAdapter::registered(SEED)||artifact::adapter_base(&init.model)!=Some(&base)||!p.parent_entry(&root.join("initial.r3m"),&init)? {
        return Err(bad("adapter native/policy/data/tape mismatch"));
    }Ok(())
}
pub(in super::super::super::super) fn parity(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;if !is(&p){return Err(bad("adapter parity profile"));}expansion_review(&p)?;
    let(es,_,expected)=word::parent_cases(&root,&p,"parity")?;
    orbit_observe(&study,"adapter-zero-parity",&root.join("initial.r3m"),&es,&binary::record!({"policy":digest(&p)?,"checkpoint":p.initial,"selection":"fixed prior six-panel parity cases; zero B"}),expected.as_deref(),observation_control(&p,es.len(),0)?)?;
    observed(&study,"adapter-zero-parity",&p,&p.initial,&es,true)?;println!("ADAPTER_ZERO_PARITY generation{} teacher0 optimizer0",es.len());Ok(())
}
pub(super) fn authorize(root:&Path,p:&Plan)->Result<()> {
    verify(root,p)?;expansion_review(p)?;let prep:binary::Value=read_confirmed(&own(p).study.join("preparation.r3b"))?;
    if prep["selection"]!=file_hash(&own(p).study.join("selection.r3b"))?{return Err(bad("adapter A preparation mismatch"));}
    if !p.tiny {let(es,_,_)=word::parent_cases(root,p,"parity")?;observed(&own(p).study,"adapter-zero-parity",p,&p.initial,&es,true)?;}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST:&str="training::fresh::identifiable::binding::citation::adapt::tests::adapter_native_process";
    #[test]
    #[ignore="registered native TINY fixtures, actual isolated child processes"]
    fn adapter_native_process()->Result<()> {
        if let Ok(root)=std::env::var("R3_ADAPTER_NUMERIC_CHILD") {let root=Path::new(&root);return numeric_updates(root,&root.join("one.r3b"),&root.join("split.r3b"),1);}
        if let Ok(path)=std::env::var("R3_ADAPTER_CHILD"){return run(Path::new(&path),std::env::var("R3_ADAPTER_ACTION").as_deref()!=Ok("one"));}
        let previous=PathBuf::from(std::env::var("R3_ADAPTER_WORD").map_err(|_|bad("explicit preserved TINY word study"))?);
        let out=PathBuf::from(std::env::var("R3_ADAPTER_TEST_ROOT").map_err(|_|bad("new adapter test root"))?);std::fs::create_dir(&out)?;let out=out.canonicalize()?;
        let child=|root:&Path,action:&str,fault:Option<&str>,label:&str|->Result<()> {
            let mut cmd=std::process::Command::new(std::env::current_exe()?);cmd.args(["--ignored","--exact",TEST,"--nocapture","--test-threads=1"])
                .env("R3_ADAPTER_CHILD",root).env("R3_ADAPTER_ACTION",action).env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1");
            if let Some(f)=fault{cmd.env("R3_FRESH_CALL_STOP",f);}let result=cmd.output()?;
            std::fs::write(out.join(format!("{label}.stdout")),&result.stdout)?;std::fs::write(out.join(format!("{label}.stderr")),&result.stderr)?;
            assert!(result.status.success(),"{label}: {} {}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));
            assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));Ok(())
        };
        let mut identities=vec![];let mut outputs=vec![];let mut totals=[0usize;3];
        for mode in ["continuous","split"] {
            let study=out.join(mode);prepare(&previous,&study,true)?;super::super::super::tests::fixture_review(&study)?;
            let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;let step=p.config.max_steps;
            malformed(&root.join("initial.r3m"),&study)?;
            let init=checkpoint::load(&root.join("initial.r3m"),Device::Cpu,true)?;
            assert_eq!(init.model.vars.len(),init.model.config.layers*4);assert_eq!(init.optimizer.len(),init.model.vars.len()*2);
            let base=artifact::adapter_base(&init.model).unwrap().clone();
            // This preserved process fixture has zero output projections and a
            // deliberately EOS-biased tensor. q/v gradients are consequently
            // zero. A separate random native test below verifies nonzero updates.
            let base_vars=init.model.base_tensors();assert_eq!(base_vars["layer.0.o"].sqr()?.sum_all()?.to_scalar::<f32>()?,0.);drop(init);
            let fault=format!("eval-{step:04}-word4/generation/1/fresh_panel_row_durable");
            if mode=="split"{child(&root,"one",None,"split-update1")?;let h=history(&root,&p)?;assert_eq!(h.last().unwrap().step,p.origin_step()+1);}
            child(&root,"run",(mode=="split").then_some(fault.as_str()),&format!("{mode}-run"))?;
            if mode=="split" {
                let h=history(&root,&p)?;assert_eq!(h.last().unwrap().phase.as_deref(),Some("EvaluationPending"));assert_eq!(h.last().unwrap().step,step);
                let path=root.join(format!("eval-{step:04}-word4.r3rows"));let prefix=binary::read_value_records(&path)?;
                child(&root,"run",None,"split-eval-only")?;assert_eq!(&binary::read_value_records(&path)?[..prefix.len()],prefix.as_slice());
                let h=history(&root,&p)?;assert_eq!(read::<binary::Value>(&root.join(format!("segment-{:04}/train-control.r3b",h.len()-1)))?["optimizer_calls"],0);
            }
            let(end,d)=close(&root,&p)?;assert_eq!(end.step,step);assert_eq!(d["extend"],false);assert_eq!(end.stop,format!("FINAL_ADAPTER_WORD_QUALITY_FAIL_AT_{step}"));
            assert!(run(&root,true).is_err());assert!(word::qa(&study,false).is_err());assert!(word::parent_observe(&study,"dev").is_err());
            let l=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,true)?;assert_eq!(l.model.base_content_id()?,base.content);assert_eq!(file_hash(&base.path)?,base.physical);
            let state=l.manifest.training.as_ref().unwrap();identities.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?,state.step,state.sampler_state,state.consumed_tokens,state.target_tokens));
            let mut raw=vec![];for(name,_,_)in word::panels(&root,&p,step)? {for r in binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?.into_iter().skip(1) {raw.push(binary::record!({"actual":r["actual"],"tokens":r["raw_tokens"],"finish":r["finish_reason"],"error":r["error"]}));}}
            outputs.push(raw);for(i,e)in history(&root,&p)?.iter().enumerate(){totals[0]+=read::<binary::Value>(&root.join(format!("segment-{i:04}/train-control.r3b")))?["optimizer_calls"].as_u64().unwrap()as usize;totals[1]+=e.generations;totals[2]+=e.teachers;}
        }
        assert_eq!(identities[0],identities[1]);assert_eq!(outputs[0],outputs[1]);assert_eq!(totals,[4,72,72]);
        random_resume(&out)?;totals[0]+=4;
        write(&out.join("usage.r3b"),&binary::record!({"optimizer":totals[0],"generation":totals[1],"teacher":totals[2]}))?;
        println!("ADAPTER_NATIVE EOS process fixture2 vs1+1+eval-only0 raw SAME; random native2 vs1+1 nonzero gradient/update SAME; base UNCHANGED optimizer8 generation72 teacher72 numeric_backward4");Ok(())
    }
    fn random_resume(out:&Path)->Result<()> {
        let root=out.join("continuous/ANSWER-MEAN");let template=checkpoint::load(&root.join("initial.r3m"),Device::Cpu,true)?;
        let model=Transformer::init(template.model.config.clone(),17,Device::Cpu)?;let initial=model.weight_hash()?;
        let mut m=template.manifest.clone();m.initial_weight_hash=initial.clone();m.training.as_mut().unwrap().initial_weight_hash=initial;
        let mut base_moments=BTreeMap::new();for(n,v)in &model.vars{for prefix in ["adam.m.","adam.v."]{base_moments.insert(format!("{prefix}{n}"),Tensor::zeros(v.shape(),candle_core::DType::F32,&Device::Cpu)?);}}
        let base_path=out.join("numeric-base.r3m");checkpoint::save(&base_path,&model,&template.tokenizer,m.clone(),&base_moments)?;drop(base_moments);
        let base=artifact::AdapterBase{path:base_path.clone(),physical:file_hash(&base_path)?,content:model.weights_content_id()?,architecture:model.config.semantic_id()?,tokenizer:template.tokenizer.id(),framing:m.framing()?.digest(),step:m.trained_steps};
        let mut model=model;model.initialize_adapter(QvAdapter::registered(SEED))?;artifact::bind_adapter_base(&mut model,base.clone())?;
        m.training.as_mut().unwrap().parent_checkpoint_hash=Some(base.physical.clone());
        let start=out.join("numeric-start.r3b");checkpoint::save(&start,&model,&template.tokenizer,m,&template.optimizer)?;
        numeric_updates(out,&start,&out.join("two.r3b"),2)?;numeric_updates(out,&start,&out.join("one.r3b"),1)?;
        let result=std::process::Command::new(std::env::current_exe()?).args(["--ignored","--exact",TEST,"--nocapture","--test-threads=1"]).env("R3_ADAPTER_NUMERIC_CHILD",out).env("VECLIB_MAXIMUM_THREADS","1").output()?;
        std::fs::write(out.join("numeric-child.stdout"),&result.stdout)?;std::fs::write(out.join("numeric-child.stderr"),&result.stderr)?;
        assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
        let a=checkpoint::load(&out.join("two.r3b"),Device::Cpu,true)?;let b=checkpoint::load(&out.join("split.r3b"),Device::Cpu,true)?;
        assert_eq!(a.model.weight_hash()?,b.model.weight_hash()?);assert_ne!(a.model.weight_hash()?,model.weight_hash()?);assert_eq!(optimizer_hash(&a.optimizer)?,optimizer_hash(&b.optimizer)?);
        assert_eq!(digest(&a.manifest.training)?,digest(&b.manifest.training)?);assert_eq!(a.model.base_content_id()?,base.content);assert_eq!(file_hash(&base_path)?,base.physical);Ok(())
    }
    fn numeric_updates(root:&Path,input:&Path,output:&Path,steps:usize)->Result<()> {
        let mut l=checkpoint::load(input,Device::Cpu,true)?;let mut adam=crate::training::Adam{moments:l.optimizer};
        let p:Plan=read(&root.join("continuous/ANSWER-MEAN/plan.r3b"))?;let corpus=verified_corpus(&root.join("continuous/ANSWER-MEAN/corpus.r3cor"),&p.corpus)?;
        let ss=samples(&corpus.train,&l.tokenizer,p.config.seq_len)?;let state=l.manifest.training.as_mut().unwrap();
        for _ in 0..steps {let ids=&own(&p).rows[state.step];let b=batch(&ss,ids,&Device::Cpu)?;let z=l.model.forward(&b.input,Some(&b.valid))?;
            let(_,loss,target,den)=crate::training::response_objective(&z,&b,1.,true)?;assert_eq!(den,8);let g=loss.backward()?;
            let grads=l.model.vars.iter().map(|(n,v)|Ok((n.clone(),g.get(v).ok_or_else(||bad("random adapter gradient missing"))?.clone()))).collect::<Result<BTreeMap<_,_>>>()?;
            let clock=state.step-artifact::adapter_base(&l.model).unwrap().step+1;
            if steps==2&&clock==1 {
                let mut pieces=BTreeMap::<String,Tensor>::new();let mut value=0.;
                for at in [0,4]{let mb=batch(&ss,&ids[at..at+4],&Device::Cpu)?;let mz=l.model.forward(&mb.input,Some(&mb.valid))?;
                    let(_,ml,_,md)=crate::training::response_objective(&mz,&mb,1.,true)?;assert_eq!(md,4);value+=ml.to_scalar::<f32>()?as f64/2.;let mg=(ml*0.5)?.backward()?;
                    for(n,v)in &l.model.vars{let x=mg.get(v).unwrap();let next=match pieces.remove(n){Some(old)=>(old+x)?,None=>x.clone()};pieces.insert(n.clone(),next);}}
                assert!((value-loss.to_scalar::<f32>()?as f64).abs()<2e-5);
                for(n,g)in &grads{let a=g.flatten_all()?.to_vec1::<f32>()?;let b=pieces[n].flatten_all()?.to_vec1::<f32>()?;assert!(a.iter().zip(b).all(|(a,b)|(a-b).abs()<2e-5),"microbatch {n}");}
                println!("ADAPTER_ANSWER same actual8 vs4+4 example-normalized gradient PASS numeric_backward3");
            }
            let(norm,delta)=adam.step_constant(&l.model.vars,&grads,&state.config,clock,3e-4)?;assert!(norm>0.&&delta>0.);
            if clock==1 {assert!(grads.iter().filter(|(n,_)|n.ends_with(".a")).all(|(_,g)|g.sqr().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap()==0.));}
            else{assert!(grads.iter().filter(|(n,_)|n.ends_with(".a")).any(|(_,g)|g.sqr().unwrap().sum_all().unwrap().to_scalar::<f32>().unwrap()>0.));}
            state.step+=1;state.sampler_state=state.step as u64;state.consumed_tokens+=b.tokens as u64;state.target_tokens+=target as u64;state.train_loss=Some(loss.to_scalar::<f32>()?as f64);
        }
        checkpoint::save(output,&l.model,&l.tokenizer,l.manifest,&adam.moments)?;Ok(())
    }
    fn malformed(initial:&Path,study:&Path)->Result<()> {
        let good:binary::Value=read(initial)?;
        let loaded=checkpoint::load(initial,Device::Cpu,true)?;let output=study.join("generic-must-not-start");
        let result=crate::training::train(crate::training::Run{checkpoint:initial,corpus:None,output:&output,resume:true,numeric_probe:false,
            config:loaded.manifest.training.as_ref().unwrap().config.clone(),stop_after:None,measure_rss:false,extend_steps:None,
            extend_microbatch:None,extend_sample_group_size:None,extend_curriculum_steps:None,extend_first_target_weight:None,extend_lr:None,extend_warmup:None,source_id:None,replace_corpus:false},std::sync::Arc::new(AtomicBool::new(false)));
        assert!(result.is_err());assert!(!output.exists());
        for case in ["rank","scale","base","tokenizer","framing","objective","mask","clock","lineage","version","status","source","loss","missing","trailing","truncated"] {
            let mut r=good.clone();match case {
                "rank"=>r["descriptor"]["rank"]=binary::record!(7),"scale"=>r["descriptor"]["alpha_bits"]=binary::record!(7f64.to_bits()),
                "base"=>r["base"]["physical"]=binary::record!("0".repeat(64)),"tokenizer"=>r["base"]["tokenizer"]=binary::record!("0".repeat(64)),
                "framing"=>r["base"]["framing"]=binary::to_value([0u8;32])?,"objective"=>r["manifest"]["training"]["resume_binding"]["family"]=binary::record!(1),
                "mask"=>r["manifest"]["tensors"]=binary::record!({}),"clock"=>r["manifest"]["trained_steps"]=binary::record!(0),
                "lineage"=>{r["manifest"]["initial_weight_hash"]=binary::record!("0".repeat(64));r["manifest"]["training"]["initial_weight_hash"]=binary::record!("0".repeat(64));},
                "version"=>r["manifest"]["version"]=binary::record!(0),"status"=>r["manifest"]["status"]=binary::record!("invented"),"source"=>r["manifest"]["source_id"]=binary::record!("x"),
                "loss"=>r["manifest"]["training"]["train_loss"]=binary::record!(f64::INFINITY),"missing"=>r["tensors"]=binary::record!([]),_=>{}
            }
            let path=study.join(format!("malformed-{case}.r3b"));let mut bytes=binary::to_storage_vec(&r)?;
            if case=="trailing"{bytes.push(0);}if case=="truncated"{bytes.pop();}std::fs::write(&path,bytes)?;
            assert!(checkpoint::load(&path,Device::Cpu,true).is_err(),"accepted {case}");
        }Ok(())
    }
}
