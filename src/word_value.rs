//! Training-only four-word selection profile on the existing citation harness.
use super::*;

pub(super) const CONTRACT: &str = "R3-QA-WORD-VALUE-1.0";
const DATA: &str = "qa-word-value-v1";
const WORDS: [&str; 4] = ["왼쪽", "오른쪽", "직진", "대기"];
const KEEP: usize = 6144;
const DEV: usize = 3072;
pub(in super::super::super::super) fn is(p: &Plan) -> bool {
    adapt::is(p) || p.identifiable.as_ref().is_some_and(|o| o.dataset == DATA)
}

// Value is an entire, nonempty string. No vocabulary restriction, trimming or
// correction is used here or in generation. The app citation parser is unchanged.
fn parsed_answer(text: &str) -> Option<(&str, i64)> {
    let (value, suffix) = text.split_once("입니다. [event:")?;
    let digits = suffix.strip_suffix(']')?;
    if value.is_empty() || digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let id = digits.parse::<i64>().ok().filter(|&n| n > 0)?;
    if text != format!("{value}입니다. [event:{id}]") || citations(text).ok()? != [id] {
        return None;
    }
    Some((value, id))
}

type Group = (usize, usize, usize, usize);
fn groups() -> Result<(Vec<Group>, Vec<Group>)> {
    let (mut train, mut dev) = (vec![], vec![]);
    for w0 in 0..4 { for w1 in w0+1..4 {
        let mut keys = vec![];
        for k0 in 0..10 { for k1 in k0+1..10 {
            keys.push((digest(&(CONTRACT, 20260924u64, k0, k1, WORDS[w0], WORDS[w1]))?, (k0,k1,w0,w1)));
        }}
        keys.sort();
        train.extend(keys[..32].iter().map(|x|x.1));
        dev.extend(keys[32..40].iter().map(|x|x.1));
    }}
    Ok((train,dev))
}
fn generated(split: &str, gs: &[Group], versions: usize, used: &mut BTreeSet<i64>) -> Result<(Vec<Episode>,Vec<Meta>)> {
    let mut ids = stream(20260924, &format!("{CONTRACT}/{split}/independent-event-ids"));
    let mut out = vec![]; let mut ms = vec![];
    for version in 0..versions { for (base_index,&(k0,k1,w0,w1)) in gs.iter().enumerate() {
        let mut event = [0i64;2];
        for id in &mut event {
            for _ in 0..10000 {
                let candidate=10_000_000+(ids.next_u64()%90_000_000) as i64;
                if used.insert(candidate) { *id=candidate; break; }
            }
            if *id==0 {return Err(bad("word finite independent ID stream exhausted"));}
        }
        let keys=[format!("장치{k0}"),format!("장치{k1}")];
        // Renaming must preserve order, times, input and answer value.
        let reverse=u64::from_str_radix(&digest(&(CONTRACT,"order",k0,k1,w0,w1,version))?[..16],16).map_err(|_|bad("word order"))?%2==1;
        let base=format!("{DATA}/{split}/{base_index}/id{version}");
        for view in 0..4 {
            let assignment=view/2;let query=view%2;
            let values=if assignment==0 {[WORDS[w0],WORDS[w1]]}else{[WORDS[w1],WORDS[w0]]};
            let mut records=(0..2).map(|j| {
                let mut r=record(&keys[j],"구역0",values[j],event[j],0,"current");
                r.source="s".into();r.recorded_at=0;r
            }).collect::<Vec<_>>();
            if reverse {records.reverse();}
            let request=ModelRequest {request_id:String::new(),system:SYSTEM.into(),
                input:format!("{} 구역0 {}",keys[query],phrases(Intent::Current)[0]),
                evidence:EvidenceBundle{items:records,..Default::default()},
                limits:GenerationLimits{context_tokens:2048,max_tokens:32,timeout_ms:120000}};
            let answer=bridge_resolve(&request)?;
            if parsed_answer(&answer)!=Some((values[query],event[query])) {return Err(bad("word request-only unique label"));}
            let id=format!("{base}/{view}");
            out.push(Episode{id:id.clone(),category:0,family:base.clone(),binding:format!("keys{k0}-{k1}/words{w0}-{w1}"),sequence:base.clone(),request,answer});
            ms.push(Meta{id,base:base.clone(),template:format!("words{w0}-{w1}/id{version}"),bucket:0,view,split:split.into(),entities:keys.to_vec(),source_id:None,query_context:Some("구역0".into())});
        }
    }}
    Ok((out,ms))
}
fn new_tape(parent: &[[usize;8]], tm: &[Meta]) -> Result<Vec<[usize;8]>> {
    if parent.len()!=1536 || tm.len()!=1536 {return Err(bad("word tape exact source counts"));}
    // Alternate ID version/assignment; each word pair contributes 32 groups.
    // Adjacent query pairs use different semantic groups, including cycle seams.
    let mut pairs=vec![];
    for key_rank in 0..32 {for view_pair in 0..4 {for word_pair in 0..6 {
        let version=view_pair%2;let assignment=view_pair/2;let base=word_pair*32+key_rank;
        pairs.push(KEEP+version*768+base*4+assignment*2);
    }}}
    let mut out=vec![];
    for s in 0..3072 {
        let p=&parent[s/2];let offset=(s%2)*4;
        let a=pairs[(2*s)%768];let b=pairs[(2*s+1)%768];
        if (a-KEEP)%768/4==(b-KEEP)%768/4 {return Err(bad("word batch repeats semantic group"));}
        out.push([p[offset],p[offset+1],p[offset+2],p[offset+3],a,a+1,b,b+1]);
    }
    Ok(out)
}
fn inputs(parent:&Path, old:&Plan, tok:&ByteBpe, reserved:&BTreeSet<i64>) -> Result<(data::native::Corpus,Vec<Meta>,Vec<Meta>,Vec<[usize;8]>,binary::Value)> {
    let c=verified_corpus(&parent.join("corpus.r3cor"),&old.corpus)?;
    let (tm,dm,xm)=verified_metadata(parent,old)?;
    if c.train.len()!=KEEP||c.validation.len()!=DEV||tm.len()!=KEEP||dm.len()!=DEV||digest(&dm)?!=digest(&xm)? {return Err(bad("word original bridge data"));}
    let original_train=digest(&c.train)?;let original_dev=digest(&c.validation)?;
    let mut used=reserved.clone();
    for e in c.train.iter().chain(&c.validation) {used.extend(e.request.evidence.items.iter().map(|r|r.event_id));}
    let (train_g,dev_g)=groups()?;
    let (train,train_m)=generated("train",&train_g,2,&mut used)?;
    let (dev,dev_m)=generated("dev",&dev_g,1,&mut used)?;
    let (renamed,renamed_m)=generated("renamed",&dev_g,1,&mut used)?;
    for (a,b) in dev.iter().zip(&renamed) {
        let mut q=b.request.clone();
        for (r,old) in q.evidence.items.iter_mut().zip(&a.request.evidence.items) {r.event_id=old.event_id;}
        if digest(&q)?!=digest(&a.request)?||parsed_answer(&a.answer).map(|v|v.0)!=parsed_answer(&b.answer).map(|v|v.0) {return Err(bad("word renamed changes outside IDs"));}
    }
    let suffix=own(old).rows.get(old.origin_step()..old.config.max_steps).ok_or_else(||bad("word parent tape cursor"))?;
    let fixture_tape=bridge_tape();
    let tape=new_tape(if old.tiny {&fixture_tape}else{suffix},&train_m)?;
    let mut train_all=c.train;train_all.extend(train);let mut train_meta=tm;train_meta.extend(train_m);
    let mut dev_all=c.validation;dev_all.extend(dev);dev_all.extend(renamed);let mut dev_meta=dm;dev_meta.extend(dev_m);dev_meta.extend(renamed_m);
    let train_set=train_g.iter().copied().collect::<BTreeSet<_>>();
    if dev_g.iter().any(|g|train_set.contains(g)) {return Err(bad("word semantic split overlap"));}
    let mut cross=BTreeMap::<String,usize>::new();let mut prompts=BTreeSet::new();
    for (i,e) in train_all.iter().chain(&dev_all).enumerate() {
        let prompt=tok.prepare_with_framing(&e.request,neural::Framing::QuestionEvidence,2048,"word-input-check")?;
        let target=tok.encode(e.answer.as_bytes())?;
        if !prompt.excluded.is_empty()||e.request.evidence.items.len()!=2||tok.decode(&target)?!=e.answer
            ||target.len()+1>32||prompt.token_ids.len()+target.len()+1>256 {return Err(bad(&format!("word complete input bound case={} prompt={} target={}",e.id,prompt.token_ids.len(),target.len()+1)));}
        let sample=samples(std::slice::from_ref(e),tok,256)?;
        if sample[0].tokens[..sample[0].response_start]!=prompt.token_ids {return Err(bad("word train/generation prompt mismatch"));}
        if i<train_all.len(){prompts.insert(digest(&prompt.token_ids)?);}else if prompts.contains(&digest(&prompt.token_ids)?){return Err(bad("word exact train/dev prompt overlap"));}
        if e.id.starts_with(DATA) {
            let (v,id)=parsed_answer(&e.answer).ok_or_else(||bad("word gold grammar"))?;
            let selected=e.request.evidence.items.iter().position(|r|r.event_id==id).ok_or_else(||bad("word support"))?;
            let entity=parsed_record(&e.request.evidence.items[selected])?.0;
            let small=id==e.request.evidence.items.iter().map(|r|r.event_id).min().unwrap();
            let small_key=entity==e.request.evidence.items.iter().map(|r|parsed_record(r).map(|v|v.0)).collect::<Result<Vec<_>>>()?.into_iter().min().unwrap();
            *cross.entry(format!("{}/{v}/{entity}/side{selected}/small-id{small}/small-key{small_key}",if i<train_all.len(){"train"}else{"dev"})).or_default()+=1;
        }
    }
    if digest(&&train_all[..KEEP])?!=original_train||digest(&&dev_all[..DEV])?!=original_dev {return Err(bad("word changed historical episodes"));}
    let costs=qa_token_cost(&train_all,&tape,tok)?;
    if costs["input"].as_u64().is_none_or(|n|n>8_000_000)||costs["target"].as_u64().is_none_or(|n|n>1_000_000){return Err(bad("BLOCKED_BUDGET word token costs"));}
    for (i,count) in costs["counts"].as_array().unwrap().iter().enumerate() {
        if *count!=if i<POOL{2}else if i<3*POOL{1}else if i<KEEP{4}else{8} {return Err(bad("word exposure count"));}
    }
    let mut manifest=c.manifest;manifest.generator=DATA.into();manifest.seed=20260924;
    manifest.split_rule="word-pair hash rank key-pairs 32 train / 8 dev / 5 unused; two train ID versions; common renamed dev".into();
    manifest.train=data::native::split("train",&train_all);manifest.validation=data::native::split("validation",&dev_all);
    let audit=binary::record!({"train_groups":train_g,"dev_groups":dev_g,"unused_groups":30,"counts":[1536,192,192],"cross":cross,"costs":costs,
        "original_train":original_train,"original_dev":original_dev,"reserved_ids":digest(reserved)?,"tape":digest(&tape)?,"parent_suffix":digest(&suffix)?,"maximum_length":256,"normalizer":"examples; EOS included; no prompt/padding","vocabulary_scope":"four seen words, heldout combinations","confirmation_opened":false});
    Ok((data::native::from_episodes(manifest,train_all,dev_all)?,train_meta,dev_meta,tape,audit))
}

// Walk only explicit lineage/source references. Do not discover artifacts or
// open sealed cases; public event reservations contain no question/answer data.
fn reserved_ids(parent:&Path)->Result<(BTreeSet<i64>,BTreeMap<PathBuf,String>)> {
    let mut todo=vec![parent.to_path_buf()];let mut seen=BTreeSet::new();let mut ids=BTreeSet::new();let mut hashes=BTreeMap::new();
    while let Some(root)=todo.pop() {
        if !seen.insert(root.clone()){continue;}if seen.len()>96{return Err(bad("word lineage bound"));}
        for name in ["corpus.r3cor","transfer.r3cor"] {
            let path=root.join(name);if path.is_file(){let c=data::native::read(&path)?;hashes.insert(path.clone(),file_hash(&path)?);
                for e in c.train.iter().chain(&c.validation){ids.extend(e.request.evidence.items.iter().map(|r|r.event_id));}}
        }
        let study=if root.join("selection.r3b").is_file(){root.clone()}else{root.parent().ok_or_else(||bad("word lineage parent"))?.into()};
        for name in ["confirmation-reservation.r3b","used-confirmation-event-ids.r3b"] {
            let path=study.join(name);if path.is_file(){ids.extend(read_ids(&path,128)?);hashes.insert(path.clone(),file_hash(&path)?);}
        }
        let path=study.join("selection.r3b");if path.is_file(){
            let v:binary::Value=read(&path)?;hashes.insert(path.clone(),file_hash(&path)?);
            for key in ["parent","old_qa","balanced"] {if let Some(path)=v[key].as_str(){let path=PathBuf::from(path);if path.is_dir(){todo.push(path);}}}
        }
    }
    Ok((ids,hashes))
}
fn policy(old:&Plan,study:&Path,s:&binary::Value,tape:&[[usize;8]])->Result<Plan> {
    let state:TrainingState=binary::from_value(s["parent_state"].clone())?;
    let mut p=old.clone();let get=|key:&str|s[key].as_str().map(str::to_owned).ok_or_else(||bad("word policy field"));
    p.source=get("source")?;p.binary=get("binary")?;p.initial=get("physical")?;p.initial_weights=get("weights")?;
    p.corpus=get("corpus")?;p.transfer=get("transfer")?;p.metadata=get("metadata")?;
    p.config.seq_len=256;p.config.lr=3e-5;p.config.warmup=0;p.config.max_steps=state.step+if p.tiny{2}else{3072};
    p.config.budget_start_step=state.step;p.config.budget_start_tokens=state.consumed_tokens;p.config.max_tokens=state.consumed_tokens+8_000_000;
    let o=p.identifiable.as_mut().ok_or_else(||bad("word parent profile"))?;o.study=study.into();o.dataset=DATA.into();o.rows.truncate(state.step);
    if o.rows.len()!=state.step{return Err(bad("word parent tape cursor"));}
    o.rows.extend(tape.iter().take(if p.tiny{2}else{3072}).copied());
    if p.tiny {for col in 4..8 {o.rows[state.step+1][col]=o.rows[state.step][col]+2;}}
    p.train_order=digest(&o.rows)?;p.order=vec![(0..7680).collect()];
    let f=p.fork.as_mut().ok_or_else(||bad("word parent fork"))?;f.study=study.into();f.study_hash=file_hash(&study.join("selection.r3b"))?;
    f.parent_policy=digest(old)?;f.parent_state=digest(&state)?;f.parent_adam=get("adam")?;f.origin_step=state.step;f.origin_input=state.consumed_tokens;f.origin_target=state.target_tokens;
    f.original_corpus=p.corpus.clone();f.constant_lr=3e-5;f.target_limit=1_000_000;p.evaluation=super::evaluation(&p);Ok(p)
}
pub(in super::super::super::super) fn prepare(previous:&Path,output:&Path,tiny:bool)->Result<()> {
    if tiny!=cfg!(all(test,feature="test-support")){return Err(bad("explicit word production/TINY boundary"));}
    let previous=previous.canonicalize()?;let parent=previous.join(MEAN_ARMS[1]);let old=historical_plan(&parent)?;
    let(end,d)=close(&parent,&old)?;bridge_diagnostic_admission(&old,&end,&d)?;
    if old.tiny!=tiny||(!tiny&&end.step!=14336)||old.config.lr!=3e-5||!answer_mean(&old)||old.config.first_target_weight!=1.
        ||old.config.microbatch!=8||old.config.accumulation!=1||old.framing()!=neural::Framing::QuestionEvidence {return Err(bad("word exact bridge14336 parent"));}
    if !tiny{verify_review_b(&previous,&comparison(&previous,&old,&end,&d)?)?;}
    let native=parent.join(&end.checkpoint);let physical=file_hash(&native)?;
    if !tiny&&physical!="15015900a3c91a6b8ed87e9c2ecb8f7faf124849e5dcf8f40203cddf267e30d9" {return Err(bad("word registered parent native differs"));}
    let l=checkpoint::load(&native,Device::Cpu,true)?;let state=l.manifest.training.as_ref().ok_or_else(||bad("word missing Adam/state"))?;
    if state.step!=end.step||state.sampler_state!=end.step as u64||state.resume_binding!=Some(old.binding(state,&l.tokenizer)?)
        ||state.resume_binding.as_ref().is_none_or(|b|b.family!=checkpoint::ANSWER_MEAN_FAMILY||b.normalizer!=2)||state.config!=old.config {return Err(bad("word native objective/Adam/clock binding"));}
    let (reserved,mut protected)=reserved_ids(&parent)?;
    let(c,tm,dm,tape,audit)=inputs(&parent,&old,&l.tokenizer,&reserved)?;
    for path in [native.clone(),parent.join("plan.r3b"),terminal_path(&parent,&end)?,parent.join(format!("citation-decision-{:04}.r3b",end.step)),parent.join("tokenizer.r3b"),previous.join("preparation.r3b")] {protected.insert(path.clone(),file_hash(&path)?);}
    if !tiny{let b:binary::Value=read_confirmed(&previous.join("review-b.r3b"))?;for path in [previous.join("review-b.r3b"),PathBuf::from(b["report_path"].as_str().ok_or_else(||bad("word parent B"))?)]{protected.insert(path.clone(),file_hash(&path)?);}}
    let prior:binary::Value=read(&previous.join("selection.r3b"))?;
    let output=std::path::absolute(output)?;std::fs::create_dir(&output)?;let root=output.join(MEAN_ARMS[1]);std::fs::create_dir(&root)?;
    data::native::write(&root.join("corpus.r3cor"),&c,true)?;copy_native(&root.join("corpus.r3cor"),&root.join("transfer.r3cor"))?;
    write(&root.join("metadata.r3b"),&(tm,dm.clone(),dm))?;copy_native(&native,&root.join("initial.r3m"))?;copy_native(&parent.join("tokenizer.r3b"),&root.join("tokenizer.r3b"))?;
    let mut s=binary::record!({"contract":CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"parent":parent,"parent_endpoint":end,"parent_state":state,
        "physical":physical,"weights":l.model.weight_hash()?,"tensor_content":l.model.weights_content_id()?,"adam":optimizer_hash(&l.optimizer)?,"tokenizer":old.tokenizer,
        "parent_protected":protected,"reserved_ids":reserved,"audit":audit,"old_qa":prior["old_qa"],"old_qa_hashes":prior["old_qa_hashes"],
        "historical_resume_unchanged":true,"objective":checkpoint::ANSWER_MEAN_OBJECTIVE,"lr_bits":3e-5f64.to_bits(),"new_updates":if tiny{2}else{3072}});
    for (key,name) in [("corpus","corpus.r3cor"),("transfer","transfer.r3cor"),("metadata","metadata.r3b")] {s[key]=binary::record!(file_hash(&root.join(name))?);}
    write(&output.join("selection.r3b"),&s)?;let p=policy(&old,&output,&s,&tape)?;write(&root.join("plan.r3b"),&p)?;
    verify(&root,&p)?;if !p.parent_entry(&root.join("initial.r3m"),&l)?{return Err(bad("word native parent entry"));}
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"contract":CONTRACT,"source":p.source,"binary":p.binary,"selection":file_hash(&output.join("selection.r3b"))?,
        "arms":{(MEAN_ARMS[1]):{"policy":file_hash(&root.join("plan.r3b"))?,"initial":p.initial,"corpus":p.corpus,"tape":p.train_order}},"optimizer":0,"generation":0,"teacher":0}))?;
    println!("WORD_PREPARED parent={} train7680 dev3456 input={} target={} samples24576 optimizer0 generation0 teacher0 A_PENDING",end.step,s["audit"]["costs"]["input"],s["audit"]["costs"]["target"]);Ok(())
}
pub(super) fn verify(root:&Path,p:&Plan)->Result<()> {
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("word parent path"))?);
    let old:Plan=read(&parent.join("plan.r3b"))?;let reserved:BTreeSet<i64>=binary::from_value(s["reserved_ids"].clone())?;
    let protected:BTreeMap<PathBuf,String>=binary::from_value(s["parent_protected"].clone())?;for(path,hash)in protected{if file_hash(&path)?!=hash{return Err(bad("word original lineage changed"));}}
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let(c,tm,dm,tape,audit)=inputs(parent,&old,&tok,&reserved)?;let actual=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(am,ad,ax)=verified_metadata(root,p)?;
    if !is(p)||s["contract"]!=CONTRACT||*p!=policy(&old,&own(p).study,&s,&tape)?||root!=own(p).study.join(MEAN_ARMS[1])
        ||p.initial!=file_hash(&root.join("initial.r3m"))?||p.tokenizer!=tok.id()||p.tokenizer!=old.tokenizer||file_hash(&root.join("transfer.r3cor"))?!=p.transfer||p.transfer!=p.corpus
        ||digest(&(&c.manifest,&c.train,&c.validation))?!=digest(&(&actual.manifest,&actual.train,&actual.validation))?
        ||digest(&(&tm,&dm,&dm))?!=digest(&(&am,&ad,&ax))?||audit!=s["audit"] {return Err(bad("word frozen owned data/policy/tape mismatch"));}
    Ok(())
}
fn rows_score(es:&[Episode],ms:&[Meta],rows:&[binary::Value],tok:&ByteBpe)->Result<binary::Value> {
    if es.is_empty()||es.len()%4!=0||es.len()!=rows.len()||es.len()!=ms.len(){return Err(bad("word complete four-view panel"));}
    let full=score(rows,es,ms)?;
    let mut j=OrbitScore{total:es.len(),full:full.exact,eos:full.eos,errors:full.errors,query_both:0,swap_both:0,all4:0,
        gold_foil_other_malformed:[0;4],same_across_queries:0,same_across_assignments:0,exact:vec![]};
    let(mut values,mut syntax,mut provided,mut support,mut joint,mut other,mut malformed)=(0,0,0,0,0,0,0);
    let mut slices=BTreeMap::<String,[usize;2]>::new();let mut pair_results=vec![];let mut shared_first=0;
    for ((e,m),r) in es.iter().zip(ms).zip(rows) {
        verify_generated(r,tok)?;
        if e.id!=m.id||e.family!=m.base||bridge_resolve(&e.request)?!=e.answer {return Err(bad("word frozen label/group"));}
        let (gold,id)=parsed_answer(&e.answer).ok_or_else(||bad("word gold grammar"))?;
        let selected=e.request.evidence.items.iter().find(|v|v.event_id==id).ok_or_else(||bad("word selected support"))?;
        if parsed_record(selected)?.2!=gold{return Err(bad("word complete selected value"));}
        let text=r["actual"].as_str();let parsed=text.and_then(parsed_answer);
        let v=text.and_then(|v|v.split_once("입니다. ")).is_some_and(|(v,_)|!v.is_empty()&&v==gold);
        let ids=text.map(individually_valid_ids).unwrap_or_default();
        let c=ids.len()==1&&ids.contains(&id);
        let pr=ids.len()==1&&ids.iter().all(|id|e.request.evidence.items.iter().any(|r|r.event_id==*id));
        values+=usize::from(v);syntax+=usize::from(parsed.is_some());provided+=usize::from(pr);support+=usize::from(c);joint+=usize::from(v&&c);
        other+=usize::from(pr&&!c);malformed+=usize::from(parsed.is_none());
        let good=r["generation_completed"]==true&&r["finish_reason"]=="stop"&&r["error"].is_null()&&r["actual"]==e.answer;
        j.exact.push(good);
        let foil=e.request.evidence.items.iter().find(|r|r.event_id!=id).ok_or_else(||bad("word second record"))?;
        let foil_answer=format!("{}입니다. [event:{}]",parsed_record(foil)?.2,foil.event_id);
        j.gold_foil_other_malformed[if good{0}else if r["actual"]==foil_answer{1}else if parsed.is_some(){2}else{3}]+=1;
        shared_first+=usize::from(tok.encode(gold.as_bytes())?.first()==tok.encode(parsed_record(foil)?.2.as_bytes())?.first());
        for key in [&m.template,&e.binding] {let n=slices.entry(key.clone()).or_default();n[0]+=1;n[1]+=usize::from(good);}
    }
    for (group,(raw,meta)) in j.exact.chunks_exact(4).zip(rows.chunks_exact(4).zip(ms.chunks_exact(4))) {
        if meta.iter().enumerate().any(|(i,m)|m.view!=i||m.base!=meta[0].base){return Err(bad("word ALL4 metadata order"));}
        j.query_both+=usize::from(group[0]&&group[1])+usize::from(group[2]&&group[3]);
        j.swap_both+=usize::from(group[0]&&group[2])+usize::from(group[1]&&group[3]);j.all4+=usize::from(group.iter().all(|v|*v));
        j.same_across_queries+=usize::from(raw[0]["actual"].is_string()&&raw[0]["actual"]==raw[1]["actual"])+usize::from(raw[2]["actual"].is_string()&&raw[2]["actual"]==raw[3]["actual"]);
        j.same_across_assignments+=usize::from(raw[0]["actual"].is_string()&&raw[0]["actual"]==raw[2]["actual"])+usize::from(raw[1]["actual"].is_string()&&raw[1]["actual"]==raw[3]["actual"]);
        pair_results.push(binary::record!({"base":meta[0].base,"query":[group[0]&&group[1],group[2]&&group[3]],"swap":[group[0]&&group[2],group[1]&&group[3]],"all4":group.iter().all(|v|*v)}));
    }
    if j.full!=j.exact.iter().filter(|v|**v).count(){return Err(bad("word full independent recount mismatch"));}
    Ok(binary::record!({"scorer":"strict-word-value-v1","joint":j,"value_correct":values,"citation_syntax_valid":syntax,"citation_provided":provided,
        "citation_support_correct":support,"value_and_citation_correct":joint,"other_provided_id":other,"valid_outside_id":valid_outside_ids(es,rows),"parse_failure_rows":malformed,
        "breakdown":slices,"pairs":pair_results,"shared_first_gold_foil":shared_first,"different_first_gold_foil":es.len()-shared_first}))
}
pub(super) fn read_score(root:&Path,p:&Plan,step:usize,panel:&Panel)->Result<binary::Value> {
    let(name,es,ms)=panel;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
    let summary=audit_panel(root,p,step,name,es,ms,&tok)?;let raw=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
    let mut out=rows_score(es,ms,&raw[1..],&tok)?;
    let teachers=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}-teachers.r3rows")))?;
    let(mut first,mut value_nll,mut suffix_nll,mut eos)=(0.,0.,0.,0.);let(mut nv,mut nc,mut boundary)=(0,0,0);
    for (e,r) in es.iter().zip(&teachers[1..]) {
        let obs=&r["teacher"]["target_token_observation"];let n:Vec<f64>=binary::from_value(obs["nll"].clone())?;
        let mut tokens=tok.encode(e.answer.as_bytes())?;tokens.push(EOS);
        if obs["gold"]!=binary::record!(tokens)||n.len()!=tokens.len()||n.iter().any(|v|!v.is_finite()){return Err(bad("word teacher actual target"));}
        let value_len=parsed_answer(&e.answer).ok_or_else(||bad("word teacher gold"))?.0.len();let mut offset=0;
        first+=n[0];eos+=n[n.len()-1];
        for (&t,&loss) in tokens[..tokens.len()-1].iter().zip(&n) {
            let size=tok.decode_bytes(&[t])?.len();
            if offset<value_len {value_nll+=loss;nv+=1;if offset+size>value_len{boundary+=1;}}
            else {suffix_nll+=loss;nc+=1;}offset+=size;
        }
        if offset!=e.answer.len(){return Err(bad("word teacher byte spans"));}
    }
    out["model"]=binary::record!(summary.model);out["raw"]=binary::record!(summary.raw_hash);out["cases"]=binary::record!(digest(es)?);
    out["teacher"]=binary::record!({"scope":"gold-prefix diagnostic, never normal generation","first_value_nll":first/es.len()as f64,"value_nll":value_nll/nv as f64,"value_tokens":nv,
        "citation_nll":suffix_nll/nc as f64,"citation_tokens":nc,"eos_nll":eos/es.len()as f64,"cross_boundary_value_tokens":boundary,"margin":"NOT_MEASURED"});Ok(out)
}
fn word_pass(s:&binary::Value,fit:bool,tiny:bool)->Result<bool> {
    let j:OrbitScore=binary::from_value(s["joint"].clone())?;
    let(n,full,pair,all4)=if tiny{(4,4,2,1)}else if fit{(1536,1524,756,372)}else{(192,183,88,44)};
    Ok(pass(&j,n,full,pair,all4)&&j.swap_both>=pair&&s["valid_outside_id"]==0&&s["parse_failure_rows"]==0
        &&(fit||(s["value_correct"].as_u64().is_some_and(|n|n>=if tiny{4}else{190})&&s["citation_support_correct"].as_u64().is_some_and(|n|n>=if tiny{4}else{190}))))
}
pub(super) fn base_panels(root:&Path,p:&Plan,step:usize)->Result<Vec<Panel>> {
    if !p.evaluation_due(step){return Err(bad("word unregistered evaluation"));}
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(tm,dm,_)=verified_metadata(root,p)?;let full=full_evaluation(p,step)||p.tiny;
    let mut out=vec![];let n=if p.tiny{4}else if full{512}else{64};
    for (i,name) in ["value","citation","renamed","S1Q0","S0Q1","S1Q1"].iter().enumerate(){if full||[0,1,5].contains(&i){out.push((format!("{name}{n}"),c.validation[i*512..i*512+n].to_vec(),dm[i*512..i*512+n].to_vec()));}}
    let n=if p.tiny{4}else if full{192}else{64};
    for (i,name) in ["word","word-renamed"].iter().enumerate(){out.push((format!("{name}{n}"),c.validation[DEV+i*192..DEV+i*192+n].to_vec(),dm[DEV+i*192..DEV+i*192+n].to_vec()));}
    if full {
        let exposed=own(p).rows[p.origin_step()..step].iter().flatten().copied().filter(|i|*i>=KEEP).collect::<BTreeSet<_>>();
        let n=if p.tiny{4}else{128};let mut indices=vec![];
        for at in (KEEP..KEEP+1536).step_by(4){if (at..at+4).all(|i|exposed.contains(&i)){indices.extend(at..at+4);if indices.len()==n{break;}}}
        if indices.len()!=n{return Err(bad("word train screen not actually exposed"));}
        out.push((format!("word-train{n}"),indices.iter().map(|&i|c.train[i].clone()).collect(),indices.iter().map(|&i|tm[i].clone()).collect()));
    }Ok(out)
}
fn development(scores:&BTreeMap<String,binary::Value>,p:&Plan,step:usize)->Result<(bool,bool)> {
    if !full_evaluation(p,step)&&!p.tiny{return Ok((false,false));}
    let n=if p.tiny{4}else{192};let words=word_pass(&scores[&format!("word{n}")],false,p.tiny)?&&word_pass(&scores[&format!("word-renamed{n}")],false,p.tiny)?;
    Ok((words,!p.tiny&&bridge_dev_pass(scores)?))
}
fn guard(root:&Path,p:&Plan,step:usize)->Result<binary::Value> {
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("word guard parent"))?);
    let old:Plan=read(&parent.join("plan.r3b"))?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let mut before=[0;3];let mut previous=None;
    for at in p.evaluation.train_steps.iter().copied().filter(|&at|at<=step) {
        let panels=base_panels(root,p,at)?;let mut after=[0;3];let mut screens=vec![];let mut hashes=vec![];let mut stop=None;
        for (i,prefix) in ["value","citation","S1Q1"].into_iter().enumerate() {
            let panel=panels.iter().find(|v|v.0.starts_with(prefix)).ok_or_else(||bad("word guard panel"))?;
            let n=if p.tiny{4}else{64};let rawpath=root.join(format!("eval-{at:04}-{}.r3rows",panel.0));let raw=binary::read_value_records(&rawpath)?;
            let original=parent.join(format!("eval-{:04}-{prefix}{}.r3rows",p.origin_step(),if p.tiny{4}else{512}));let baseline=binary::read_value_records(&original)?;
            let score=|rows:&[binary::Value]|->Result<OrbitScore>{if i==0{orbit_score(&panel.1[..n],&panel.2[..n],rows,&tok)}else{Ok(score_citation_profile(&panel.1[..n],&panel.2[..n],rows,&tok,true)?.joint)}};
            let b=score(&baseline[1..n+1])?;let now=score(&raw[1..n+1])?;
            let warning=b.full.saturating_sub(now.full)>4||b.all4.saturating_sub(now.all4)>2;
            after[i]=if warning{before[i]+1}else{0};
            let bad_rows=raw[1..n+1].iter().filter(|r|r["finish_reason"]!="stop"||r["generation_completed"]!=true||!r["error"].is_null()).count();
            if !p.tiny {if b.full.saturating_sub(now.full)>=16||bad_rows>=4{stop=Some("SEVERE_RETENTION_REGRESSION");}else if after[i]>=2&&stop.is_none(){stop=Some("PERSISTENT_RETENTION_REGRESSION");}}
            screens.push(binary::record!({"panel":prefix,"parent":b,"current":now,"non_eos_or_error":bad_rows}));hashes.push((file_hash(&original)?,file_hash(&rawpath)?));
        }
        let g=binary::record!({"before":before,"after":after,"screens":screens,"raw":hashes,"previous":previous,"stop":stop,"parent_policy":digest(&old)?,"origin_counts_as_warning":false});
        if at==step{return Ok(g);}let path=root.join(format!("citation-decision-{at:04}.r3b"));let d:binary::Value=read_confirmed(&path)?;
        if d["guard"]!=g||d["policy"]!=digest(p)?||stop.is_some(){return Err(bad("word guard prior decision mismatch"));}previous=Some(file_hash(&path)?);before=after;
    }Err(bad("word guard unscheduled"))
}
pub(super) fn panels(root:&Path,p:&Plan,step:usize)->Result<Vec<Panel>> {
    let mut out=base_panels(root,p,step)?;
    if (full_evaluation(p,step)||p.tiny)&&out.iter().all(|x|root.join(format!("eval-{step:04}-{}.r3b",x.0)).exists()) {
        let scores=out.iter().map(|panel|Ok((panel.0.clone(),super::read_score(root,p,step,panel)?))).collect::<Result<BTreeMap<_,_>>>()?;
        let(w,r)=development(&scores,p,step)?;
        if !guard(root,p,step)?["stop"].is_string()&&((w&&r)||(!adapt::is(p)&&step==p.config.max_steps)) {
            let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(tm,_,_)=verified_metadata(root,p)?;let n=if p.tiny{4}else{1536};
            out.push((format!("word-fit{n}"),c.train[KEEP..KEEP+n].to_vec(),tm[KEEP..KEEP+n].to_vec()));
        }
    }Ok(out)
}
pub(super) fn decision(root:&Path,p:&Plan,step:usize)->Result<binary::Value> {
    let cases=panels(root,p,step)?;let scores=cases.iter().map(|panel|Ok((panel.0.clone(),super::read_score(root,p,step,panel)?))).collect::<Result<BTreeMap<_,_>>>()?;
    let models=scores.values().map(|s|s["model"].as_str().ok_or_else(||bad("word model identity"))).collect::<Result<BTreeSet<_>>>()?;
    if models.len()!=1{return Err(bad("word mixed endpoint models"));}
    let g=guard(root,p,step)?;let regression=g["stop"].is_string();let (words,retained)=development(&scores,p,step)?;
    let fit_key=format!("word-fit{}",if p.tiny{4}else{1536});let fit=scores.get(&fit_key).map(|s|word_pass(s,true,p.tiny)).transpose()?;
    let eligible=words&&retained&&fit==Some(true)&&!regression;
    let action=if regression{g["stop"].as_str().unwrap().to_owned()}else if eligible{format!("CANDIDATE_FIXED_AT_{step}")}else if fit.is_some(){format!("FINAL_WORD_QUALITY_FAIL_AT_{step}")}else if adapt::is(p)&&step==p.config.max_steps{format!("FINAL_ADAPTER_WORD_QUALITY_FAIL_AT_{step}")}else{"CONTINUE_WITHIN_REGISTERED_CAP".into()};
    let extend=action=="CONTINUE_WITHIN_REGISTERED_CAP";
    if extend&&step==p.config.max_steps{return Err(bad("word final missing fit/decision"));}
    Ok(binary::record!({"policy":digest(p)?,"step":step,"model":models.into_iter().next(),"panels":scores,"guard":g,"word_development":words,"retention_joint":retained,
        "development":words&&retained,"fit":fit,"eligible":eligible,"regression":regression,"action":action,"extend":extend,"stop":(!extend).then_some(action),"S4":"NOT_OPENED","GOAL1_ACCEPTED":false}))
}
pub(super) fn parent_cases(root:&Path,p:&Plan,name:&str)->Result<(Vec<Episode>,Vec<Meta>,Option<Vec<binary::Value>>)> {
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(_,ms,_)=verified_metadata(root,p)?;
    if name=="dev"||name=="renamed" {let at=DEV+if name=="renamed"{192}else{0};let n=if p.tiny{4}else{192};return Ok((c.validation[at..at+n].to_vec(),ms[at..at+n].to_vec(),None));}
    if name!="parity"{return Err(bad("word parent panel"));}
    let s:binary::Value=read(&own(p).study.join("selection.r3b"))?;let parent=Path::new(s["parent"].as_str().ok_or_else(||bad("word parity parent"))?);
    let mut es=vec![];let mut meta=vec![];let mut expected=vec![];
    for (i,prefix) in ["value","citation","renamed","S1Q0","S0Q1","S1Q1"].iter().enumerate() {
        let n=if p.tiny{4}else{512};let raw=binary::read_value_records(&parent.join(format!("eval-{:04}-{prefix}{n}.r3rows",p.origin_step())))?;
        let count=if p.tiny{2}else{[4,2,2,2,2,4][i]};let mut pairs=(0..n).step_by(2).collect::<Vec<_>>();
        // Coverage first: include each panel's first failed pair when present,
        // then deterministic metadata order. Never select easy correct cases.
        pairs.sort_by_key(|&at|(![at,at+1].into_iter().any(|j|raw[j+1]["actual"]!=c.validation[i*512+j].answer||raw[j+1]["finish_reason"]!="stop"||!raw[j+1]["error"].is_null()),ms[i*512+at].base.clone()));
        for at in pairs.into_iter().take(count/2){for j in at..at+2 {es.push(c.validation[i*512+j].clone());meta.push(ms[i*512+j].clone());expected.push(raw[j+1].clone());}}
    }Ok((es,meta,Some(expected)))
}
pub(in super::super::super::super) fn parent_observe(study:&Path,name:&str)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;if !is(&p){return Err(bad("word parent profile"));}expansion_review(&p)?;
    if adapt::is(&p){return Err(bad("adapter permits only registered zero-delta parity"));}
    let(es,ms,expected)=parent_cases(&root,&p,name)?;let label=format!("word-parent-{name}");
    orbit_observe(&study,&label,&root.join("initial.r3m"),&es,&binary::record!({"policy":digest(&p)?,"checkpoint":p.initial,"selection":"six-panel metadata coverage and first failed pairs"}),expected.as_deref(),observation_control(&p,es.len(),0)?)?;
    let rows=observed(&study,&label,&p,&p.initial,&es,expected.is_some())?;
    if expected.is_none(){let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let score=rows_score(&es,&ms,&rows,&tok)?;println!("WORD_PARENT {name} score={score}");}
    println!("WORD_PARENT panel={name} generation={} teacher0 optimizer0",es.len());Ok(())
}
pub(super) fn authorize(root:&Path,p:&Plan)->Result<()> {
    verify(root,p)?;expansion_review(p)?;let prep:binary::Value=read_confirmed(&own(p).study.join("preparation.r3b"))?;
    if prep["selection"]!=file_hash(&own(p).study.join("selection.r3b"))?{return Err(bad("word A selection binding"));}
    if !p.tiny {let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let mut pass=true;
        for name in ["parity","dev","renamed"] {let(es,ms,expected)=parent_cases(root,p,name)?;let rows=observed(&own(p).study,&format!("word-parent-{name}"),p,&p.initial,&es,expected.is_some())?;
            if name!="parity"{pass&=word_pass(&rows_score(&es,&ms,&rows,&tok)?,false,false)?;}}
        if pass{return Err(bad("NO_TRAINING_NEEDED_IN_THIS_SCOPE"));}
    }Ok(())
}
pub(in super::super::super::super) fn report(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;
    if !is(&p){return Err(bad("word report profile"));}let h=history(&root,&p)?;let end=h.last().ok_or_else(||bad("word NOT_RUN"))?;
    for step in p.evaluation.train_steps.iter().copied().filter(|&n|n<=end.step) {
        if step==end.step&&end.phase.as_deref()==Some("EvaluationPending"){continue;}
        let d=decision(&root,&p,step)?;if d!=read_confirmed::<binary::Value>(&root.join(format!("citation-decision-{step:04}.r3b")))?{return Err(bad("word raw decision mismatch"));}
        for(name,s)in d["panels"].as_object().unwrap(){println!("WORD_PANEL step={step} {name} full={} QB={} SB={} ALL4={} value={} support={} outside={} parse={} EOS={} errors={}",s["joint"]["full"],s["joint"]["query_both"],s["joint"]["swap_both"],s["joint"]["all4"],s["value_correct"],s["citation_support_correct"],s["valid_outside_id"],s["parse_failure_rows"],s["joint"]["eos"],s["joint"]["errors"]);}
    }
    let t=trace(&root,&p,end)?;println!("WORD_TRACE updates={} input={} target={} usage={:?} durable={} step={} resume={} stop={} Goal1=false",t["updates"],t["input"],t["target"],work(&p)?,end.checkpoint_hash,end.step,end.resume,end.stop);
    if !end.resume&&end.phase.as_deref()==Some("Finished"){let(e,d)=close(&root,&p)?;println!("WORD_COMPARISON {}",comparison(&study,&p,&e,&d)?);}Ok(())
}
fn diagnostic_admission(p:&Plan,end:&Segment,d:&binary::Value)->Result<()> {
    if !is(p)||end.resume||end.phase.as_deref()!=Some("Finished")||(!p.tiny&&![p.origin_step()+1536,p.origin_step()+3072].contains(&end.step))
        ||d["step"]!=end.step||d["stop"]!=end.stop||d["extend"]!=false||d["regression"]!=false
        ||!(end.stop==format!("FINAL_WORD_QUALITY_FAIL_AT_{}",end.step)||end.stop==format!("CANDIDATE_FIXED_AT_{}",end.step)) {return Err(bad("word diagnostic requires normal complete scheduled endpoint"));}
    let n=if p.tiny{4}else{1536};if d["panels"][&format!("word-fit{n}")]["joint"]["total"]!=n{return Err(bad("word diagnostic final fit missing"));}Ok(())
}
fn review_cases(root:&Path,p:&Plan,step:usize)->Result<(Vec<Episode>,Vec<binary::Value>)> {
    let mut es=vec![];let mut expected=vec![];let mut errors=vec![];
    for(name,cases,ms) in panels(root,p,step)? {
        let raw=binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?;
        for at in (0..cases.len()).step_by(2) {
            if ms[at].base!=ms[at+1].base{return Err(bad("word reviewer complete query pair"));}
            let failed=(at..at+2).any(|i|raw[i+1]["actual"]!=cases[i].answer||raw[i+1]["finish_reason"]!="stop"||!raw[i+1]["error"].is_null());
            if at<2 {for i in at..at+2{es.push(cases[i].clone());expected.push(raw[i+1].clone());}}
            else if failed&&errors.len()<32 {for i in at..at+2{errors.push((cases[i].clone(),raw[i+1].clone()));}}
        }
    }
    for(e,r)in errors.into_iter().take(64-es.len()){es.push(e);expected.push(r);}Ok((es,expected))
}
pub(in super::super::super::super) fn review(study:&Path)->Result<()> {
    let study=study.canonicalize()?;let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;let(end,_)=close(&root,&p)?;
    if !is(&p){return Err(bad("word review profile"));}let(es,raw)=review_cases(&root,&p,end.step)?;
    orbit_observe(&study,"word-review",&root.join(&end.checkpoint),&es,&binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"selection":"one fixed query pair per panel plus complete failed pairs; capped64"}),Some(&raw),observation_control(&p,es.len(),0)?)?;
    observed(&study,"word-review",&p,&end.checkpoint_hash,&es,true)?;println!("WORD_REVIEW generation={} matched={} teacher0 optimizer0",es.len(),es.len());Ok(())
}
pub(in super::super::super::super) fn qa(study:&Path,transfer:bool)->Result<()> {
    let study=study.canonicalize()?;let lock=std::fs::File::open(&study)?;lock.try_lock().map_err(|_|bad("word diagnostic already running"))?;
    let root=study.join(MEAN_ARMS[1]);let p=historical_plan(&root)?;if adapt::is(&p){return Err(bad("adapter QA not authorized"));}let(end,d)=close(&root,&p)?;diagnostic_admission(&p,&end,&d)?;
    verify_review_b(&study,&comparison(&study,&p,&end,&d)?)?;let(es,_)=review_cases(&root,&p,end.step)?;observed(&study,"word-review",&p,&end.checkpoint_hash,&es,true)?;
    let panel=bridge_diagnostic_cases(&study,&p,transfer)?;let name=if transfer{"bridge-qa-transfer"}else{"bridge-qa-primary"};let s:binary::Value=read(&study.join("selection.r3b"))?;
    let identity=binary::record!({"policy":digest(&p)?,"checkpoint":end.checkpoint_hash,"mode":"DIAGNOSTIC_ONLY","old_qa":s["old_qa_hashes"],"generation_limit":128,"generation_count":panel.1.len(),"teacher_limit":0,"context":2048,"candidate_authority":false,"S4_authority":false});
    bridge_diagnostic_output(&study,&root,&p,&end,&panel,&ByteBpe::load(&root.join("tokenizer.r3b"))?,name,&identity,bridge_diagnostic_prior_usage(&p,name)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parent()->Result<PathBuf>{Ok(PathBuf::from(std::env::var("R3_WORD_PARENT").map_err(|_|bad("explicit preserved parent path required"))?))}
    #[test]
    #[ignore = "reads only the registered plan/terminal; zero model calls"]
    fn word_scope_never_confirmation()->Result<()> {
        let root=parent()?;let mut p:Plan=read(&root.join("plan.r3b"))?;
        let end=history(&root,&p)?.last().cloned().ok_or_else(||bad("word scope fixture endpoint"))?;
        p.identifiable.as_mut().unwrap().dataset=DATA.into();
        for tiny in [false,true] {p.tiny=tiny;for eligible in [false,true] {
            assert!(!confirmation_admitted(&root,&p,&end,&binary::record!({"eligible":eligible}))?);
        }}
        println!("WORD_CONFIRMATION denied for SMALL/TINY, positive/negative quality, before opening a seal; model_calls0");Ok(())
    }
    #[test]
    #[ignore = "explicit preserved corpus/tokenizer, no model calls"]
    fn word_data_tape_boundaries()->Result<()> {
        let root=parent()?;let p:Plan=read(&root.join("plan.r3b"))?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let(ids,paths)=reserved_ids(&root)?;let(c,tm,dm,tape,audit)=inputs(&root,&p,&tok,&ids)?;
        assert_eq!((c.train.len(),c.validation.len(),tm.len(),dm.len(),tape.len()),(7680,3456,7680,3456,3072));
        for cycle in tape.chunks_exact(384){let mut counts=vec![0;1536];for row in cycle {for &i in &row[4..]{counts[i-KEEP]+=1;}}assert!(counts.iter().all(|&n|n==1));}
        let suffix=&own(&p).rows[p.origin_step()..p.config.max_steps];
        for (i,row) in suffix.iter().enumerate(){assert_eq!(&tape[2*i][..4],&row[..4]);assert_eq!(&tape[2*i+1][..4],&row[4..]);}
        let mut altered=c.train[KEEP].clone();altered.request.input=altered.request.input.replace("장치", "미등록");assert!(bridge_resolve(&altered.request).is_err());
        let reserved_count=ids.len();let frames=samples(&c.train,&tok,256)?;let ids=tape[0];let b=batch(&frames,&ids,&Device::Cpu)?;let width=b.input.dim(1)?;let vocab=tok.vocab_size();
        let z=candle_core::Var::from_vec((0..8*width*vocab).map(|i|((i%19)as f32-9.)/13.).collect::<Vec<_>>(),(8,width,vocab),&Device::Cpu)?;
        let(_,loss,n,den)=crate::training::response_objective(z.as_tensor(),&b,1.,true)?;assert_eq!(den,8);assert_eq!(n,ids.iter().map(|&i|frames[i].tokens.len()-frames[i].response_start).sum::<usize>());
        let grad=loss.backward()?.get(&z).unwrap().flatten_all()?.to_vec1::<f32>()?;let mut sum=vec![0f32;grad.len()];let mut objective=0.;
        for offset in [0,4] {let mb=batch(&frames,&ids[offset..offset+4],&Device::Cpu)?;let logits=z.narrow(0,offset,4)?.narrow(1,0,mb.input.dim(1)?)?;
            let(_,l,_,d)=crate::training::response_objective(&logits,&mb,1.,true)?;assert_eq!(d,4);objective+=f64::from(l.to_scalar::<f32>()?)/2.;
            for(a,b)in sum.iter_mut().zip((l*0.5)?.backward()?.get(&z).unwrap().flatten_all()?.to_vec1::<f32>()?){*a+=b;}}
        assert!((objective-f64::from(loss.to_scalar::<f32>()?)).abs()<2e-5);assert!(grad.iter().zip(sum).all(|(a,b)|(a-b).abs()<2e-6));
        let mask=b.mask.to_vec2::<f32>()?;for row in 0..8 {let s=&frames[ids[row]];assert_eq!(s.tokens.last(),Some(&EOS));assert_eq!(mask[row][s.tokens.len()-2],1.);
            for pos in 0..width{if mask[row][pos]==0.{assert!(grad[(row*width+pos)*vocab..(row*width+pos+1)*vocab].iter().all(|x|*x==0.));}}}
        println!("WORD_ANSWER_GRADIENT actual review4/word4 batch8 vs4+4, actual mask denominator/EOS, synthetic backward3, native optimizer0 generation0 teacher0");
        println!("WORD_DATA train7680 dev3456 groups192/48 unused30 token_input={} token_target={} length={} reserved_ids={} lineage_files={} optimizer0 generation0 teacher0",audit["costs"]["input"],audit["costs"]["target"],audit["costs"]["maximum_sequence"],reserved_count,paths.len());Ok(())
    }
    #[test]
    #[ignore = "actual native tokenizer, typed record reader; zero model calls"]
    fn word_reader_score_gate()->Result<()> {
        let root=parent()?;let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;let mut used=BTreeSet::new();let(gs,_)=groups()?;
        let(es,ms)=generated("fixture",&gs[..48],1,&mut used)?;
        let gold=es.iter().map(|e|super::super::tests::gold(e,&tok)).collect::<Result<Vec<_>>>()?;
        let dir=tempfile::tempdir()?;
        for mode in ["gold","wrong-value","other-provided","outside","mixed","no-eos","utf8"] {
            let mut rows=gold.clone();let e=&es[0];
            if mode!="gold" {
                let(v,id)=parsed_answer(&e.answer).unwrap();let foil=e.request.evidence.items.iter().find(|r|r.event_id!=id).unwrap();
                let text=match mode {"wrong-value"=>format!("{}입니다. [event:{id}]",parsed_record(foil)?.2),"other-provided"=>format!("{v}입니다. [event:{}]",foil.event_id),"outside"=>format!("{v}입니다. [event:1]"),"mixed"=>format!("{v}입니다. [event:oops] [event:1]"),_=>e.answer.clone()};
                let mut output=e.clone();output.answer=text;rows[0]=super::super::tests::gold(&output,&tok)?;rows[0]["expected"]=binary::record!(e.answer);rows[0]["exact_match"]=binary::record!(false);
                if mode=="no-eos" {let ids=tok.encode(e.answer.as_bytes())?;rows[0]["raw_tokens"]=binary::record!(ids);rows[0]["finish_reason"]=binary::record!("length");rows[0]["generation"]["finish"]=binary::record!("length");rows[0]["generation"]["generated"]=binary::record!(ids.len());}
                if mode=="utf8" {let ids=tok.encode(&[0xff])?;rows[0]["raw_tokens"]=binary::record!([ids.clone(),vec![EOS]].concat());rows[0]["generation"]["tokens"]=binary::record!(ids);rows[0]["generation"]["generated"]=binary::record!(ids.len()+1);rows[0]["actual"]=binary::Value::Null;rows[0]["error"]=binary::record!("strict UTF-8");rows[0]["error_class"]=binary::record!("strict_utf8");}
            }
            let path=dir.path().join(format!("{mode}.r3rows"));let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(&path)?;for row in &rows{append_row(&mut f,row)?;}drop(f);
            let got=rows_score(&es,&ms,&binary::read_value_records(&path)?,&tok)?;
            assert_eq!(got["joint"]["full"],if mode=="gold"{192}else{191});
            if mode=="gold"{assert!(word_pass(&got,false,false)?);}else if ["outside","mixed","no-eos","utf8"].contains(&mode){assert!(!word_pass(&got,false,false)?);}
            if mode=="wrong-value"{assert_eq!(got["value_correct"],191);assert_eq!(got["citation_support_correct"],192);}
            if mode=="other-provided"{assert_eq!(got["value_correct"],192);assert_eq!(got["other_provided_id"],1);}
            if mode=="mixed"{assert_eq!(got["value_correct"],192);assert_eq!(got["valid_outside_id"],1);assert_eq!(got["parse_failure_rows"],1);}
        }
        let p:Plan=read(&root.join("plan.r3b"))?;let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(_,dm,_)=verified_metadata(&root,&p)?;
        let raw=binary::read_value_records(&root.join(format!("eval-{:04}-citation512.r3rows",p.config.max_steps)))?;
        let numeric=score_citation(&c.validation[512..516],&dm[512..516],&raw[1..5],&tok)?;assert_eq!(numeric.value_correct,4);assert_eq!(numeric.joint.full,4);
        assert_eq!(value("왼쪽입니다. [event:1]"),None);assert_eq!(parsed_answer("입니다. [event:1]"),None);
        println!("WORD_READER_SCORE_GATE seven cases typed writer/reader/scorer/gate; numeric4 unchanged; optimizer0 generation0 teacher0");Ok(())
    }
    #[test]
    #[ignore = "typed synthetic endpoint records, zero model calls"]
    fn word_raw_endpoint_gates()->Result<()> {
        let parent=parent()?;let old:Plan=read(&parent.join("plan.r3b"))?;let tok=ByteBpe::load(&parent.join("tokenizer.r3b"))?;
        let(ids,_)=reserved_ids(&parent)?;let(c,tm,dm,tape,_)=inputs(&parent,&old,&tok,&ids)?;
        let base=PathBuf::from(std::env::var("R3_WORD_GATE_ROOT").map_err(|_|bad("new gate evidence path required"))?);std::fs::create_dir(&base)?;let base=base.canonicalize()?;
        let end=history(&parent,&old)?.last().cloned().ok_or_else(||bad("fixture parent endpoint"))?;
        for label in ["early-pass","early-dev-fail","early-fit-fail","final-dev-fail"] {
            let root=base.join(label);std::fs::create_dir(&root)?;copy_native(&parent.join("tokenizer.r3b"),&root.join("tokenizer.r3b"))?;
            data::native::write(&root.join("corpus.r3cor"),&c,true)?;write(&root.join("metadata.r3b"),&(tm.clone(),dm.clone(),dm.clone()))?;
            write(&root.join("selection.r3b"),&binary::record!({"parent":parent}))?;
            let mut p=old.clone();p.tiny=false;p.corpus=file_hash(&root.join("corpus.r3cor"))?;p.metadata=file_hash(&root.join("metadata.r3b"))?;p.order=vec![(0..7680).collect()];
            p.config.max_steps=14336+3072;p.config.lr=3e-5;p.config.seq_len=256;p.config.warmup=0;p.fork.as_mut().unwrap().origin_step=14336;
            let o=p.identifiable.as_mut().unwrap();o.study=root.clone();o.dataset=DATA.into();o.rows.truncate(14336);o.rows.extend(&tape);p.train_order=digest(&o.rows)?;
            let step=14336+if label=="final-dev-fail"{3072}else{1536};p.evaluation=evaluation(&p);p.evaluation.train_steps=vec![step];
            let mut l=checkpoint::load(&parent.join(&end.checkpoint),Device::Cpu,true)?;
            let state=l.manifest.training.as_mut().unwrap();state.config=p.config.clone();state.step=step;state.sampler_state=step as u64;state.resume_binding=Some(p.binding(state,&l.tokenizer)?);
            let native=root.join("synthetic-step-fixture.r3m");checkpoint::save(&native,&l.model,&l.tokenizer,l.manifest,&l.optimizer)?;drop(l.model);
            for panel in base_panels(&root,&p,step)? {
                let wrong=if label.ends_with("dev-fail")&&panel.0=="word192"{12}else{0};
                super::super::tests::precision_panel_fixture(&root,&p,step,&native,&panel,wrong)?;
            }
            let all=panels(&root,&p,step)?;let fit=all.iter().find(|p|p.0=="word-fit1536");
            assert_eq!(fit.is_some(),label!="early-dev-fail");
            if let Some(panel)=fit {assert!(decision(&root,&p,step).is_err());super::super::tests::precision_panel_fixture(&root,&p,step,&native,panel,if label=="early-fit-fail"{13}else{0})?;}
            let d=decision(&root,&p,step)?;assert_eq!(d["eligible"],label=="early-pass");assert_eq!(d["extend"],label=="early-dev-fail");
            let mut endpoint=end.clone();endpoint.step=step;endpoint.resume=false;endpoint.phase=Some("Finished".into());endpoint.stop=d["stop"].as_str().unwrap_or("CONTINUE").into();endpoint.checkpoint_hash=file_hash(&native)?;
            if label=="early-dev-fail" {assert!(diagnostic_admission(&p,&endpoint,&d).is_err());}else{
                diagnostic_admission(&p,&endpoint,&d)?;let cmp=comparison_fixture(&p,&endpoint,&d)?;
                assert!(verify_review_b(&root,&cmp).is_err());let report=root.join("fixture-b.txt");std::fs::write(&report,b"synthetic boundary fixture only; no independent approval")?;
                publish_confirmed(&root.join("preparation.r3b"),&binary::record!({"fixture":"no model calls"}))?;
                publish_confirmed(&root.join("review-b.r3b"),&binary::record!({"verdict":"PASS","preparation":file_hash(&root.join("preparation.r3b"))?,"endpoints":cmp["endpoints"],"report_path":report,"report_hash":file_hash(&report)?}))?;
                verify_review_b(&root,&cmp)?;let mut wrong=cmp.clone();wrong["endpoints"]["ANSWER-MEAN"]["checkpoint"]=binary::record!("different endpoint");assert!(verify_review_b(&root,&wrong).is_err());
            }
            publish_confirmed(&root.join("fixture-decision.r3b"),&d)?;assert_eq!(read_confirmed::<binary::Value>(&root.join("fixture-decision.r3b"))?,d);
        }
        println!("WORD_RAW_GATE early-pass/early-continue/early-fit-fail/final-dev-fail-fit-once/B-required model_calls0");Ok(())
    }
    fn comparison_fixture(p:&Plan,e:&Segment,d:&binary::Value)->Result<binary::Value>{Ok(binary::record!({"endpoints":{"ANSWER-MEAN":{"policy":digest(p)?,"checkpoint":e.checkpoint_hash,"step":e.step,"decision":d}}}))}
    #[test]
    #[ignore = "preserved TINY parent, real separate-process native training/evaluation"]
    fn word_native_process()->Result<()> {
        const TEST:&str="training::fresh::identifiable::binding::citation::word::tests::word_native_process";
        if let Ok(path)=std::env::var("R3_WORD_CHILD") {return run(Path::new(&path),std::env::var("R3_WORD_ACTION").as_deref()!=Ok("one"));}
        if !cfg!(feature="test-support"){return Err(bad("word process test-support required"));}
        let parent=parent()?;let old=historical_plan(&parent)?;assert!(old.tiny);
        let base=PathBuf::from(std::env::var("R3_WORD_PROCESS_ROOT").map_err(|_|bad("new TINY evidence root required"))?);std::fs::create_dir(&base)?;let base=base.canonicalize()?;
        let child=|root:&Path,mode:&str,fault:Option<&str>,label:&str|->Result<()> {
            let mut cmd=std::process::Command::new(std::env::current_exe()?);cmd.args(["--ignored","--exact",TEST,"--nocapture","--test-threads=1"])
                .env("R3_WORD_CHILD",root).env("R3_WORD_ACTION",mode).env("VECLIB_MAXIMUM_THREADS","1").env("RAYON_NUM_THREADS","1");
            if let Some(f)=fault{cmd.env("R3_FRESH_CALL_STOP",f);}let out=cmd.output()?;
            std::fs::write(base.join(format!("{label}.stdout")),&out.stdout)?;std::fs::write(base.join(format!("{label}.stderr")),&out.stderr)?;
            assert!(out.status.success(),"{label}: {} {}",String::from_utf8_lossy(&out.stdout),String::from_utf8_lossy(&out.stderr));assert!(String::from_utf8_lossy(&out.stdout).contains("1 passed"));Ok(())};
        let mut natives=vec![];let mut outputs=vec![];let mut totals=[0usize;3];
        for mode in ["continuous","split","eval-only"] {
            let study=base.join(mode);prepare(parent.parent().unwrap(),&study,true)?;super::super::super::tests::fixture_review(&study)?;
            let root=study.join(MEAN_ARMS[1]);let p=plan_read(&root)?;let step=p.config.max_steps;
            for which in 0..4 {let mut bad=p.clone();match which{0=>bad.config.lr=3e-4,1=>bad.config.seq_len=512,2=>bad.identifiable.as_mut().unwrap().dataset=BRIDGE_COMPLETION_DATA.into(),_=>bad.identifiable.as_mut().unwrap().rows[p.origin_step()][0]+=1};assert!(super::super::verify_plan(&root,&bad).is_err());}
            let fault=format!("eval-{step:04}-word-fit4/generation/1/fresh_panel_row_durable");
            child(&root,if mode=="split"{"one"}else{"run"},(mode=="eval-only").then_some(fault.as_str()),&format!("{mode}-0"))?;
            let prefix_path=root.join(format!("eval-{step:04}-word-fit4.r3rows"));
            let prefix=if mode=="eval-only" {let h=history(&root,&p)?;assert_eq!(h.last().unwrap().phase.as_deref(),Some("EvaluationPending"));assert_eq!(h.last().unwrap().step,step);Some(binary::read_value_records(&prefix_path)?)}else{None};
            if mode!="continuous"{child(&root,"run",None,&format!("{mode}-1"))?;}
            if let Some(prefix)=prefix{assert_eq!(&binary::read_value_records(&prefix_path)?[..prefix.len()],prefix.as_slice());let h=history(&root,&p)?;assert_eq!(read::<binary::Value>(&root.join(format!("segment-{:04}/train-control.r3b",h.len()-1)))?["optimizer_calls"],0);}
            let(end,d)=close(&root,&p)?;assert_eq!(end.step,step);assert_eq!(end.stop,format!("FINAL_WORD_QUALITY_FAIL_AT_{step}"));diagnostic_admission(&p,&end,&d)?;
            for reason in ["CANCELLED","UNKNOWN","SEVERE_RETENTION_REGRESSION","INTEGRITY_FAIL"]{let mut e=end.clone();e.stop=reason.into();assert!(diagnostic_admission(&p,&e,&d).is_err());}
            let mut missing=d.clone();missing["panels"]["word-fit4"]["joint"]["total"]=binary::record!(3);assert!(diagnostic_admission(&p,&end,&missing).is_err());assert!(run(&root,true).is_err());assert!(qa(&study,false).is_err());
            let l=checkpoint::load(&root.join(&end.checkpoint),Device::Cpu,true)?;let state=l.manifest.training.as_ref().unwrap();natives.push((l.model.weights_content_id()?,optimizer_hash(&l.optimizer)?,state.step,state.sampler_state,state.consumed_tokens,state.target_tokens));
            let mut output=vec![];for (name,_,_) in panels(&root,&p,step)? {for r in binary::read_value_records(&root.join(format!("eval-{step:04}-{name}.r3rows")))?.into_iter().skip(1){output.push(binary::record!({"actual":r["actual"],"tokens":r["raw_tokens"],"finish":r["finish_reason"],"error":r["error"]}));}}
            outputs.push(output);for(i,e)in history(&root,&p)?.iter().enumerate(){totals[0]+=read::<binary::Value>(&root.join(format!("segment-{i:04}/train-control.r3b")))?["optimizer_calls"].as_u64().unwrap()as usize;totals[1]+=e.generations;totals[2]+=e.teachers;}
        }
        assert!(natives.windows(2).all(|w|w[0]==w[1]));assert!(outputs.windows(2).all(|w|w[0]==w[1]));assert_eq!(totals,[6,120,120]);
        write(&base.join("usage.r3b"),&binary::record!({"optimizer":totals[0],"generation":totals[1],"teacher":totals[2]}))?;
        println!("WORD_NATIVE continuous2/split1+1/evaluation-only2+0 weights/Adam/clock/raw SAME optimizer{} generation{} teacher{} evidence={}",totals[0],totals[1],totals[2],base.display());Ok(())
    }
}
