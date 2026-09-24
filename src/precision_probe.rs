//! Experimental E2M1 storage, dequantized into the ordinary F32 inference path.
//! Neither MXFP4 nor NVFP4; no GPU kernel, calibration or optimizer.
use super::*;
use serde::{Serialize,Deserialize};
const FORMAT:&str="R3-FP4-E2M1-B32-F32S-EXPERIMENTAL";
const PARENT:&str="c47e34c7ac4f88dd08b5e719bf4d0364f139ac5002bf4572fe55b37dac823925";
const LEVELS:[f32;8]=[0.,0.5,1.,1.5,2.,3.,4.,6.];
fn decoded(code:u8)->f32 {let value=LEVELS[(code&7)as usize];if code&8!=0{-value}else{value}}
fn nearest(v:f64)->u8 {
    let sign=if v.is_sign_negative(){8}else{0};let a=v.abs();let mut best=0;let mut distance=f64::INFINITY;
    for (i,&x) in LEVELS.iter().enumerate(){let d=(a-x as f64).abs();if d<distance||(d==distance&&i%2==0){best=i;distance=d;}}
    best as u8|sign
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Packed {shape:Vec<usize>,scales:binary::Value,nibbles:binary::Value,elements:usize,padding:usize,clipped:usize,tail:usize}
fn bytes(v:&binary::Value)->Result<&[u8]>{if let binary::Value::Bytes(v)=v{Ok(v)}else{Err(bad("FP4 raw byte payload required"))}}
fn pack(values:&[f32],shape:&[usize])->Result<Packed>{
    if shape.len()!=2||shape.contains(&0)||shape.iter().try_fold(1usize,|a,b|a.checked_mul(*b))!=Some(values.len())||values.iter().any(|v|!v.is_finite()) {return Err(bad("FP4 shape/finite input"));}
    let mut scales=vec![];let mut payload=vec![];let mut clipped=0;
    for row in values.chunks_exact(shape[1]){for block in row.chunks(32){
        let max=block.iter().map(|x|x.abs()).fold(0f32,f32::max);
        let scale=if max==0.{1.}else{(max/6.).max(f32::from_bits(1))};scales.extend(scale.to_le_bytes());
        let mut codes=[0u8;32];for (i,&v)in block.iter().enumerate(){let normalized=v as f64/scale as f64;if normalized.abs()>6.{clipped+=1;}codes[i]=nearest(normalized);}
        payload.extend(codes.chunks_exact(2).map(|p|p[0]|p[1]<<4));
    }}
    Ok(Packed{shape:shape.into(),scales:binary::Value::Bytes(scales),nibbles:binary::Value::Bytes(payload),elements:values.len(),padding:shape[0]*shape[1].div_ceil(32)*32-values.len(),clipped,tail:((shape[1]-1)%32)+1})
}
fn unpack(p:&Packed)->Result<Vec<f32>>{
    if p.shape.len()!=2||p.shape.contains(&0)||p.shape.iter().any(|n|*n>4096){return Err(bad("FP4 shape bound"));}
    let(rows,cols)=(p.shape[0],p.shape[1]);let blocks=cols.div_ceil(32);let s=bytes(&p.scales)?;let n=bytes(&p.nibbles)?;
    if p.elements!=rows*cols||p.padding!=rows*blocks*32-p.elements||p.tail!=((cols-1)%32)+1||p.clipped>p.elements||s.len()!=rows*blocks*4||n.len()!=rows*blocks*16 {return Err(bad("FP4 length/row/scale/tail"));}
    let mut out=Vec::with_capacity(p.elements);
    for block in 0..rows*blocks{let scale=f32::from_le_bytes(s[block*4..block*4+4].try_into().unwrap());
        if !scale.is_finite()||scale<=0.{return Err(bad("FP4 nonpositive/nonfinite scale"));}
        for at in 0..32{let code=(n[block*16+at/2]>>(4*(at%2)))&15;
            if (block%blocks)*32+at<cols{let value=decoded(code)*scale;if !value.is_finite(){return Err(bad("FP4 dequant nonfinite"));}out.push(value);}
            else if code!=0{return Err(bad("FP4 nonzero padding"));}
        }
    }Ok(out)
}
fn quantized(name:&str)->bool{name.starts_with("layer.")&&["q","k","v","o","gate","up","down"].contains(&name.rsplit('.').next().unwrap_or(""))}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry{name:String,shape:Vec<usize>,packed:Option<Packed>,f32_bytes:Option<binary::Value>,digest:String}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Record{format:String,parent:PathBuf,parent_physical:String,parent_content:String,config:Config,tokenizer:String,framing:[u8;32],tensors:Vec<Entry>}
fn entry_digest(e:&Entry)->Result<String>{digest(&(&e.name,&e.shape,&e.packed,&e.f32_bytes))}
fn decode(r:&Record)->Result<checkpoint::Loaded>{
    if r.format!=FORMAT||r.parent_physical!=PARENT||file_hash(&r.parent)?!=r.parent_physical{return Err(bad("FP4 registered base/format"));}
    let mut l=checkpoint::load(&r.parent,Device::Cpu,false)?;
    if l.model.weights_content_id()?!=r.parent_content||l.model.config!=r.config||l.tokenizer.id()!=r.tokenizer||l.manifest.framing()?.digest()!=r.framing{return Err(bad("FP4 base identity"));}
    let expected=r.config.shapes();let mut tensors=BTreeMap::new();
    for e in &r.tensors{
        if expected.get(&e.name)!=Some(&e.shape)||tensors.contains_key(&e.name)||entry_digest(e)?!=e.digest{return Err(bad("FP4 tensor registry/checksum"));}
        let values=if quantized(&e.name){if e.f32_bytes.is_some(){return Err(bad("FP4 duplicate F32 tensor"));}let p=e.packed.as_ref().ok_or_else(||bad("FP4 missing packed"))?;if p.shape!=e.shape{return Err(bad("FP4 packed shape"));}unpack(p)?}
            else{if e.packed.is_some(){return Err(bad("FP4 untouched tensor quantized"));}let b=bytes(e.f32_bytes.as_ref().ok_or_else(||bad("FP4 missing F32"))?)?;
                if b.len()!=e.shape.iter().product::<usize>()*4{return Err(bad("FP4 F32 length"));}let v=b.chunks_exact(4).map(|b|f32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
                let original=l.model.vars[&e.name].flatten_all()?.to_vec1::<f32>()?;if v.iter().zip(&original).any(|(a,b)|a.to_bits()!=b.to_bits()){return Err(bad("FP4 embedding/norm changed"));}v};
        tensors.insert(e.name.clone(),Tensor::from_vec(values,e.shape.clone(),&Device::Cpu)?);
    }
    if tensors.len()!=expected.len(){return Err(bad("FP4 missing tensor"));}
    l.model=Transformer::from_tensors(r.config.clone(),tensors,Device::Cpu)?;
    l.manifest.weights_sha256=l.model.weight_hash()?;l.manifest.model_content_digest=l.model.weights_content_id()?;l.manifest.training=None;l.optimizer.clear();Ok(l)
}
fn cases(root:&Path,p:&Plan)->Result<(Vec<Episode>,Vec<Meta>,Vec<binary::Value>)>{
    let c=verified_corpus(&root.join("corpus.r3cor"),&p.corpus)?;let(_,dm,_)=verified_metadata(root,p)?;
    let mut es=vec![];let mut ms=vec![];let mut raw=vec![];let(end,_)=close(root,p)?;
    if end.step!=11264||end.checkpoint_hash!=PARENT{return Err(bad("FP4 accepted11264 endpoint"));}
    for(i,name)in ["value","citation"].iter().enumerate(){let r=binary::read_value_records(&root.join(format!("eval-11264-{name}512.r3rows")))?;
        es.extend_from_slice(&c.validation[i*512..i*512+128]);ms.extend_from_slice(&dm[i*512..i*512+128]);raw.extend_from_slice(&r[1..129]);}
    let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;for row in &raw{verify_generated(row,&tok)?;}
    let _=orbit_score(&es[..128],&ms[..128],&raw[..128],&tok)?;let _=score_citation(&es[128..],&ms[128..],&raw[128..],&tok)?;
    Ok((es,ms,raw))
}
pub(in super::super::super::super) fn prepare(parent:&Path,output:&Path)->Result<()> {
    let parent=parent.canonicalize()?;let p=historical_plan(&parent)?;let(end,_)=close(&parent,&p)?;let _=cases(&parent,&p)?;
    let path=parent.join(&end.checkpoint).canonicalize()?;if file_hash(&path)?!=PARENT{return Err(bad("FP4 source"));}
    let l=checkpoint::load(&path,Device::Cpu,false)?;let mut tensors=vec![];let mut stats=vec![];let(mut qp,mut bytes_total)=(0usize,0usize);
    for(name,var)in &l.model.vars {let values=var.flatten_all()?.to_vec1::<f32>()?;let packed=if quantized(name){qp+=values.len();Some(pack(&values,var.dims())?)}else{None};
        let f32_bytes=packed.is_none().then(||binary::Value::Bytes(values.iter().flat_map(|v|v.to_le_bytes()).collect()));
        let mut e=Entry{name:name.clone(),shape:var.dims().into(),packed,f32_bytes,digest:String::new()};e.digest=entry_digest(&e)?;
        if let Some(p)=&e.packed{let dq=unpack(p)?;let max=values.iter().zip(&dq).map(|(a,b)|(a-b).abs()).fold(0f32,f32::max);let mse=values.iter().zip(&dq).map(|(a,b)|(*a as f64-*b as f64).powi(2)).sum::<f64>()/values.len()as f64;
            bytes_total+=bytes(&p.scales)?.len()+bytes(&p.nibbles)?.len();stats.push(binary::record!({"name":name,"shape":p.shape,"elements":p.elements,"padding":p.padding,"clipped":p.clipped,"max_abs":max,"mse":mse,"packed_bytes":bytes(&p.nibbles)?.len(),"scale_bytes":bytes(&p.scales)?.len()}));}
        tensors.push(e);
    }
    let record=Record{format:FORMAT.into(),parent:path,parent_physical:PARENT.into(),parent_content:l.model.weights_content_id()?,config:l.model.config.clone(),tokenizer:l.tokenizer.id(),framing:l.manifest.framing()?.digest(),tensors};
    let output=std::path::absolute(output)?;std::fs::create_dir(&output)?;
    publish_confirmed(&output.join("model.r3b"),&record)?;let decoded:Record=read_confirmed(&output.join("model.r3b"))?;let _=decode(&decoded)?;
    publish_confirmed(&output.join("preparation.r3b"),&binary::record!({"format":FORMAT,"backend":"FP4 storage + F32 CPU Accelerate emulated inference","source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"parent_root":parent,"parent":PARENT,
        "artifact":file_hash(&output.join("model.r3b"))?,"artifact_bytes":std::fs::metadata(output.join("model.r3b"))?.len(),"quantized_parameters":qp,"total_parameters":l.model.config.parameters(),"packed_scale_bytes":bytes_total,"tensors":stats,"optimizer":0,"teacher":0,"generation_cap":272,"seconds_cap":900}))?;
    println!("FP4_PREPARED {FORMAT} quantized_params={qp}/{} file_bytes={} optimizer0 teacher0 generation0",l.model.config.parameters(),std::fs::metadata(output.join("model.r3b"))?.len());Ok(())
}
pub(in super::super::super::super) fn observe(study:&Path,parity:bool)->Result<()> {
    let study=study.canonicalize()?;let prep:binary::Value=read_confirmed(&study.join("preparation.r3b"))?;
    if prep["source"]!=source_digest()?||prep["binary"]!=file_hash(&std::env::current_exe()?)?||prep["artifact"]!=file_hash(&study.join("model.r3b"))?{return Err(bad("FP4 frozen source/artifact"));}
    let root=Path::new(prep["parent_root"].as_str().ok_or_else(||bad("FP4 parent"))?);let p=historical_plan(root)?;let(es,ms,raw)=cases(root,&p)?;
    let record:Record=read_confirmed(&study.join("model.r3b"))?;let mut elapsed=0.;
    if !parity {let prior:binary::Value=read_confirmed(&study.join("float-parity-finished.r3b"))?;
        if prior["matched"]!=16||prior["control"]["generation_calls"]!=16||prior["control"]["teacher_calls"]!=0||prior["control"]["terminal_reason"]!="COMPLETED"||!prior["error"].is_null(){return Err(bad("FP4 native parity incomplete"));}
        elapsed=prior["control"]["elapsed_seconds"].as_f64().ok_or_else(||bad("FP4 unknown elapsed"))?;
    }
    if elapsed>=900.{return Err(bad("FP4 active budget exhausted"));}
    let cancel=std::sync::Arc::new(AtomicBool::new(false));let signal=cancel.clone();ctrlc::set_handler(move||signal.store(true,Ordering::Relaxed)).map_err(|e|bad(&e.to_string()))?;
    let mut control=recovery::RunControl::new(cancel,std::time::Duration::from_secs_f64(900.-elapsed),12*1024*1024)?;control.set_call_limits(if parity{16}else{256},0);
    let indices=if parity{(0..8).chain(128..136).collect::<Vec<_>>()}else{(0..256).collect()};
    let selected=indices.iter().map(|&i|es[i].clone()).collect::<Vec<_>>();let reference=indices.iter().map(|&i|raw[i].clone()).collect::<Vec<_>>();
    let identity=binary::record!({"format":FORMAT,"parent":PARENT,"cases":digest(&selected)?,"source_raw":digest(&raw)?,"mode":if parity{"native parity16"}else{"dequantized F32 256"}});
    if parity {orbit_observe(&study,"float-parity",&record.parent,&selected,&identity,Some(&reference),control)?;}
    else {
        let started=std::time::Instant::now();let mut dequant_ms=0.;
        let rows=orbit_observe_loaded(&study,"fp4-f32",&study.join("model.r3b"),&selected,&identity,None,control,||{let at=std::time::Instant::now();let l=decode(&record)?;dequant_ms=at.elapsed().as_secs_f64()*1000.;Ok(l)})?;
        let tok=ByteBpe::load(&root.join("tokenizer.r3b"))?;
        let float_v=orbit_score(&es[..128],&ms[..128],&raw[..128],&tok)?;let float_c=score_citation(&es[128..],&ms[128..],&raw[128..],&tok)?;
        let quant_v=orbit_score(&es[..128],&ms[..128],&rows[..128],&tok)?;let quant_c=score_citation(&es[128..],&ms[128..],&rows[128..],&tok)?;
        publish_confirmed(&study.join("result.r3b"),&binary::record!({"format":FORMAT,"execution":"FP4 storage + F32 emulated inference","reference_value":float_v,"reference_citation":float_c,"quantized_value":quant_v,"quantized_citation":quant_c,
            "dequant_and_base_verification_ms":dequant_ms,"total_ms":started.elapsed().as_secs_f64()*1000.,"rss_kib":rss_kib()?,"calls":272,"optimizer":0,"teacher":0,"S6":"NOT_ACCEPTED","GOAL1_READY":false}))?;
        println!("FP4_F32 value={}/128 citation={}/128 reference={}/{} generation256 parity16 teacher0 optimizer0",quant_v.full,quant_c.joint.full,float_v.full,float_c.joint.full);
    }Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn fp4_scalar_matrix_codec()->Result<()> {
        for code in 0..16{assert_eq!(nearest(decoded(code)as f64),code);}
        assert_eq!(decoded(8).to_bits(),(-0f32).to_bits());assert_eq!(nearest(0.25),0);assert_eq!(nearest(0.75),2);assert_eq!(nearest(-0.25),8);assert_eq!(nearest(100.),7);
        for values in [vec![0.;65],vec![f32::from_bits(1);65],(0..130).map(|i|(i as f32-63.)/17.).collect()] {
            let shape=if values.len()==130{vec![2,65]}else{vec![1,65]};let p=pack(&values,&shape)?;let restored=unpack(&p)?;assert_eq!(restored.len(),values.len());
            let wire=binary::to_vec(&p)?;assert_eq!(unpack(&binary::from_slice::<Packed>(&wire)?)?,restored);
            let mut trailing=wire.clone();trailing.push(0);assert!(binary::from_slice::<Packed>(&trailing).is_err());let mut damaged=wire.clone();let at=damaged.len()/2;damaged[at]^=1;assert!(binary::from_slice::<Packed>(&damaged).is_err());
            for mode in 0..5{let mut badp=p.clone();match mode{0=>badp.tail=0,1=>badp.shape[1]+=1,2=>badp.scales=binary::Value::Bytes(vec![0;bytes(&p.scales)?.len()]),3=>badp.nibbles=binary::Value::Bytes(vec![]),_=>badp.padding+=1};assert!(unpack(&badp).is_err());}
        }
        assert!(pack(&[f32::NAN],&[1,1]).is_err());assert!(pack(&[f32::INFINITY],&[1,1]).is_err());
        let weights=(0..96).map(|i|((i*7%31)as f32-15.)/8.).collect::<Vec<_>>();let p=pack(&weights,&[3,32])?;let dq=unpack(&p)?;
        let x=(0..32).map(|i|(i as f32-16.)/32.).collect::<Vec<_>>();let oracle=|w:&[f32]|w.chunks_exact(32).map(|r|r.iter().zip(&x).map(|(a,b)|a*b).sum::<f32>()).collect::<Vec<_>>();
        let actual=Tensor::from_vec(x.clone(),(1,32),&Device::Cpu)?.matmul(&Tensor::from_vec(dq.clone(),(3,32),&Device::Cpu)?.t()?)?.flatten_all()?.to_vec1::<f32>()?;
        assert!(actual.iter().zip(oracle(&dq)).all(|(a,b)|(a-b).abs()<1e-5));let max=oracle(&weights).iter().zip(actual).map(|(a,b)|(a-b).abs()).fold(0f32,f32::max);
        println!("FP4_SCALAR_MATRIX codes16 signed-zero ties saturation subnormal padding checksum malformed PASS layer_max_abs={max} optimizer0 generation0 teacher0");Ok(())
    }
}
