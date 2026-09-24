//! Explicit offline training tool; never imported by the product inference library.
#[path = "contrast.rs"]
pub mod contrast;
#[path = "quality_recovery.rs"]
pub mod recovery;
#[path = "target_loss.rs"]
pub mod target_loss;
#[path = "fresh.rs"]
pub mod fresh;
use crate::data::{self, Episode};
use candle_core::{DType, Device, Tensor, Var};
use replica_v3::{
    Error, Result,
    app::citations,
    neural::{
        self, BOS, ByteBpe, EOS, PAD,
        checkpoint::{self, TrainConfig, TrainingState},
        transformer::{Rng, Transformer, masked_loss},
    },
};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub struct Adam {
    pub moments: BTreeMap<String, Tensor>,
}

#[cfg(all(test, feature = "metal"))]
mod metal_tests {
    use super::*;
    use neural::transformer::{Config, Kernel};

    // Registered before device execution, not fitted to observed errors.
    fn close(name:&str, reference:&Tensor, actual:&Tensor, abs:f64, rel:f64)->Result<()> {
        assert_eq!(reference.dims(),actual.dims(),"{name} shape");
        assert_eq!(reference.dtype(),actual.dtype(),"{name} dtype");
        let a=reference.flatten_all()?.to_vec1::<f32>()?;
        let b=actual.flatten_all()?.to_vec1::<f32>()?;
        let mut errors:Vec<f64>=a.iter().zip(&b).map(|(&x,&y)|(x as f64-y as f64).abs()).collect();
        let bad=a.iter().zip(&b).position(|(&x,&y)|!x.is_finite()||!y.is_finite()||(x as f64-y as f64).abs()>abs+rel*(x as f64).abs());
        errors.sort_by(f64::total_cmp);
        println!("NUMERIC {name} shape={:?} max={} p99={} rms={} first_bad={bad:?}",reference.dims(),errors.last().unwrap(),errors[(errors.len()-1)*99/100],(errors.iter().map(|x|x*x).sum::<f64>()/errors.len()as f64).sqrt());
        assert!(bad.is_none(),"{name}: registered pointwise tolerance exceeded");Ok(())
    }
    fn vector(name:&str, reference:&Tensor, actual:&Tensor, limit:f64, zero_abs:f64, cosine:bool)->Result<()> {
        assert_eq!(reference.dims(),actual.dims());
        let a=reference.flatten_all()?.to_vec1::<f32>()?;
        let b=actual.flatten_all()?.to_vec1::<f32>()?;
        assert!(a.iter().chain(&b).all(|x|x.is_finite()),"{name} finite");
        let norm=a.iter().map(|&x|(x as f64).powi(2)).sum::<f64>().sqrt();
        let other=b.iter().map(|&x|(x as f64).powi(2)).sum::<f64>().sqrt();
        let floor=1e-6*(a.len() as f64).sqrt();
        let err=a.iter().zip(&b).map(|(&x,&y)|(x as f64-y as f64).powi(2)).sum::<f64>().sqrt();
        let max=a.iter().zip(&b).map(|(&x,&y)|(x as f64-y as f64).abs()).fold(0f64,f64::max);
        let mut errors:Vec<f64>=a.iter().zip(&b).map(|(&x,&y)|(x as f64-y as f64).abs()).collect();
        errors.sort_by(f64::total_cmp);
        let p99=errors[(errors.len()-1)*99/100];let rms=err/(a.len()as f64).sqrt();
        let cos=if norm>floor && other>floor {Some(a.iter().zip(&b).map(|(&x,&y)|x as f64*y as f64).sum::<f64>()/(norm*other))}else{None};
        println!("VECTOR {name} shape={:?} norm={norm} nrmse={} max={max} p99={p99} rms={rms} near_zero_floor={floor} cosine={cos:?}",reference.dims(),err/norm.max(floor));
        if norm<=floor {assert!(max<=zero_abs,"{name}: near-zero absolute error");}
        else {assert!(err/norm<=limit,"{name}: NRMSE");if cosine {assert!(cos.is_some_and(|v|v>=0.999),"{name}: cosine");}}
        Ok(())
    }
    #[test]
    #[ignore = "bounded primitive localization after the recorded Metal gradient failure"]
    fn metal_f32_backward_boundary()->Result<()> {
        let gpu=neural::Backend::Metal0.open()?;
        let mut ss=vec![];
        for row in 0..8 {ss.push(Sample{tokens:(0..12-row%3).map(|n|8+(n+row)%32).chain([EOS]).collect(),response_start:7,curriculum:false});}
        let mut outcomes=vec![];
        for device in [&Device::Cpu,&gpu] {
            let b=batch(&ss,&(0..8).collect::<Vec<_>>(),device)?;
            let values:Vec<f32>=(0..8*12*264).map(|n|((n%73)as f32-36.)/31.).collect();
            let x=Var::from_vec(values,(8,12,264),device)?;
            let (ce,answer,_,_)=response_objective(&x,&b,1.,true)?;
            let token_grad=ce.backward()?.get(&x).unwrap().detach();
            let answer_grad=answer.backward()?.get(&x).unwrap().detach();
            let w=Var::from_vec((0..264*32).map(|n|(n%29)as f32/30.).collect::<Vec<_>>(),(264,32),device)?;
            let y=w.index_select(&b.input.flatten_all()?,0)?;
            let embedding_grad=y.sqr()?.mean_all()?.backward()?.get(&w).unwrap().detach();
            device.synchronize()?;
            outcomes.push((token_grad,answer_grad,embedding_grad));
        }
        println!("PRIMITIVE backward6 optimizer0 native_forward0 generation0 teacher0");
        vector("TOKEN-loss-logit-gradient",&outcomes[0].0,&outcomes[1].0,1e-2,1e-6,true)?;
        vector("ANSWER-loss-logit-gradient",&outcomes[0].1,&outcomes[1].1,1e-2,1e-6,true)?;
        vector("repeated-index-embedding-gradient",&outcomes[0].2,&outcomes[1].2,1e-2,1e-6,true)?;
        Ok(())
    }
    #[test]
    #[ignore = "bounded attention primitive localization, no model generation/update"]
    fn metal_f32_attention_backward_boundary()->Result<()> {
        use neural::transformer::{rms_norm,rotary,repeat_kv,gqa_attention,attention_mask,linear};
        let gpu=neural::Backend::Metal0.open()?;
        for op in ["rms","rotary","repeat-kv","gqa","linear","silu"] {
            let mut outcomes=vec![];
            for device in [&Device::Cpu,&gpu] {
                let shape=(8,4,12,8);
                let x=Var::from_vec((0..8*4*12*8).map(|i|((i%67)as f32-33.)/51.).collect::<Vec<_>>(),shape,device)?;
                let y=match op {
                    "rms"=>rms_norm(&x,&Tensor::ones((4,1,8),DType::F32,device)?,1e-6)?,
                    "rotary"=>rotary(&x,0,10000.)?,
                    "repeat-kv"=>repeat_kv(&x,8)?,
                    "gqa"=>{
                        let k=Tensor::from_vec((0..8*2*12*8).map(|i|((i%31)as f32-15.)/22.).collect::<Vec<_>>(),(8,2,12,8),device)?;
                        let v=(&k*0.7)?;
                        let mask=attention_mask(0..12,0..12,Some(8),None,8,device)?;
                        gqa_attention(&x,&k,&v,&mask)?
                    },
                    "linear"=>linear(&x,&Tensor::from_vec((0..8*16).map(|i|((i%17)as f32-8.)/13.).collect::<Vec<_>>(),(16,8),device)?)?,
                    "silu"=>candle_nn::ops::silu(&x)?,
                    _=>unreachable!(),
                };
                let upstream=Tensor::from_vec((0..y.elem_count()).map(|i|((i%41)as f32-20.)/32.).collect::<Vec<_>>(),y.shape(),device)?;
                let loss=(&y*&upstream)?.sum_all()?;
                let g=loss.backward()?.get(&x).unwrap().detach();device.synchronize()?;
                outcomes.push((y,g));
            }
            println!("PRIMITIVE {op} completed backward2 optimizer0 generation0 teacher0");
            close(&format!("{op}/forward"),&outcomes[0].0,&outcomes[1].0,1e-4,1e-3)?;
            vector(&format!("{op}/gradient"),&outcomes[0].1,&outcomes[1].1,1e-2,1e-6,true)?;
        }Ok(())
    }
    #[test]
    #[ignore = "GPU materialization diagnostic before reduction repair; no model calls"]
    fn metal_f32_canonical_sum_diagnostic()->Result<()> {
        let gpu=neural::Backend::Metal0.open()?;
        let values:Vec<f32>=(0..8*4*2*12*8).map(|i|((i%41)as f32-20.)/32.).collect();
        let input=Tensor::from_vec(values.clone(),(8,4,2,12,8),&gpu)?;
        let materialized=input.permute([0,1,3,4,2])?.contiguous()?;
        assert!(materialized.device().same_device(&gpu));
        let mut copy_oracle=Vec::new();let mut sum_oracle=Vec::new();
        for b in 0..8 {for h in 0..4 {for t in 0..12 {for d in 0..8 {
            let a=values[(((b*4+h)*2)*12+t)*8+d];
            let z=values[(((b*4+h)*2+1)*12+t)*8+d];
            copy_oracle.extend([a,z]);sum_oracle.push(a+z);
        }}}}
        gpu.synchronize()?;
        assert_eq!(materialized.flatten_all()?.to_vec1::<f32>()?,copy_oracle);
        let output=materialized.sum_keepdim(4)?.reshape((8,4,1,12,8))?;
        gpu.synchronize()?;
        assert_eq!(output.flatten_all()?.to_vec1::<f32>()?,sum_oracle);
        println!("CANONICAL_SUM device={:?} exact_copy=true exact_sum=true reductions1 temporary_bytes={} host_readback_bytes={} native_calls0 backward0 optimizer0",gpu.location(),values.len()*4,(copy_oracle.len()+sum_oracle.len())*4);
        Ok(())
    }
    #[test]
    #[ignore = "scalar-oracle localization of the observed Metal non-last-axis reduction error"]
    fn metal_f32_middle_axis_sum_oracle()->Result<()> {
        let gpu=neural::Backend::Metal0.open()?;
        let values:Vec<f32>=(0..8*4*2*12*8).map(|i|((i%41)as f32-20.)/32.).collect();
        let mut oracle=Vec::new();
        for b in 0..8 {for h in 0..4 {for t in 0..12 {for d in 0..8 {
            oracle.push(values[(((b*4+h)*2)*12+t)*8+d]+values[(((b*4+h)*2+1)*12+t)*8+d]);
        }}}}
        let expected=Tensor::from_vec(oracle,(8,4,1,12,8),&Device::Cpu)?;
        for device in [&Device::Cpu,&gpu] {
            let input=Tensor::from_vec(values.clone(),(8,4,2,12,8),device)?;
            let result=input.sum_keepdim(2)?;device.synchronize()?;
            let actual=result.flatten_all()?.to_vec1::<f32>()?;
            let gold=expected.flatten_all()?.to_vec1::<f32>()?;
            let first=gold.iter().zip(&actual).position(|(x,y)|x!=y);
            println!("SUM_ORACLE device={:?} shape={:?} stride={:?} axis2 first={:?} expected_actual={:?} native_calls0 backward0 optimizer0",
                device.location(),input.dims(),input.stride(),first,first.map(|i|(gold[i],actual[i])));
            close("middle-axis-sum/scalar-oracle",&expected,&result,1e-4,1e-3)?;
            assert_eq!(gold,actual,"exact dyadic sum oracle");
        }Ok(())
    }
    #[test]
    #[ignore = "actual device/cache/optimizer admission boundary, model calls zero"]
    fn metal_f32_admission_is_fail_closed()->Result<()> {
        let gpu=neural::Backend::Metal0.open()?;
        assert!(neural::Backend::Cpu.verify(&gpu).is_err());
        assert!(neural::Backend::Metal0.verify(&Device::Cpu).is_err());
        let cpu=Transformer::init(Config::tiny(264),20260924,Device::Cpu)?;
        let tensors=cpu.vars.iter().map(|(n,v)|Ok((n.clone(),v.as_detached_tensor().to_device(&gpu)?))).collect::<Result<BTreeMap<_,_>>>()?;
        let mut metal=Transformer::from_tensors(cpu.config.clone(),tensors,gpu.clone())?;
        let ids=Tensor::new(&[[8u32]],&gpu)?;
        let mut foreign=cpu.cache("scope");
        assert!(metal.forward_cached(&ids,&mut foreign,"scope").is_err());
        metal.set_kernel(Kernel::RustDecodeGemv);
        assert!(metal.forward(&ids,None).is_err());
        assert_eq!(metal.capabilities().device,"Metal");
        assert!(!metal.capabilities().decode);
        metal.set_kernel(Kernel::Reference);
        assert!(metal.capabilities().decode);assert!(!metal.capabilities().training);
        let before=metal.weights_content_id()?;
        let mut adam=Adam::new(&metal.vars)?;
        assert!(adam.moments.values().all(|t|t.device().same_device(&gpu)));
        let failure=adam.step_constant(&metal.vars,&BTreeMap::new(),&TrainConfig::default(),1,3e-5).unwrap_err().to_string();
        assert!(failure.contains("METAL_F32_TRAINING_NOT_ACCEPTED"));
        assert_eq!(metal.weights_content_id()?,before);
        for t in adam.moments.values(){assert!(t.flatten_all()?.to_vec1::<f32>()?.iter().all(|&x|x==0.));}
        println!("METAL_ADMISSION device/cache/kernel/rejected-Adam PASS; forward0 backward0 optimizer0 generation0 teacher0");Ok(())
    }
    #[test]
    #[ignore = "preregistered bounded layout suite; scalar f64 logical-index oracle"]
    fn metal_f32_reduction_layouts()->Result<()> {
        let gpu=neural::Backend::Metal0.open()?;
        // (source shape, reduction axes after view, view). No Cartesian sweep.
        let cases:&[(&[usize],&[usize],&str)]=&[
            (&[8,4,2,12,8],&[2],"plain"),(&[8,2,4,12,48],&[2],"plain"),
            (&[1,2,1,1,8],&[2],"plain"),(&[1,2,2,31,48],&[2],"plain"),
            (&[8,2,4,32,8],&[2],"plain"),(&[1,2,4,33,48],&[2],"plain"),
            (&[3,4,5],&[0],"plain"),(&[3,4,5],&[1],"plain"),
            (&[3,4,5],&[2],"plain"),(&[3,4,5],&[0,2],"plain"),
            (&[3,4,5],&[0,1,2],"plain"),(&[3,4,5],&[1],"transpose"),
            (&[3,4,5],&[1],"narrow"),(&[2,1,3],&[1],"broadcast"),
            (&[2,1,3],&[1],"plain"),(&[2,3],&[],"plain"),
            (&[2,0,3],&[1],"plain"),(&[2,3],&[1,1],"invalid"),
            (&[2,3],&[2],"invalid"),(&[],&[],"plain"),
            (&[3,4,5],&[2,0],"transpose"),(&[3,4,5],&[],"transpose"),
        ];
        for (case,&(shape,axes,view)) in cases.iter().enumerate() {
            let mut rng=Rng::new(20260924);
            let raw:Vec<f32>=(0..shape.iter().product::<usize>()).map(|i|if case%2==0 {((i%41)as f32-20.)/32.} else {(rng.next_u64()%65521)as f32/32749.-1.}).collect();
            let mut outputs=Vec::new();
            for device in [&Device::Cpu,&gpu] {
                let created=Tensor::from_vec(raw.clone(),shape,device);
                if shape.contains(&0) && device.is_metal() {
                    // This device rejects a zero-byte buffer before reduction.
                    // Preserve that existing constructor failure, not a fake sum.
                    let error=created.unwrap_err().to_string();
                    assert!(error.contains("Failed to create metal resource: Buffer"));
                    println!("LAYOUT {case} Metal existing zero-buffer rejection; reduction_not_called");
                    continue;
                }
                let x=created?;
                let x=match view {
                    "transpose"=>x.transpose(0,2)?,
                    "narrow"=>x.narrow(0,1,2)?.narrow(2,1,3)?,
                    "broadcast"=>x.broadcast_as((2,4,3))?,
                    _=>x,
                };
                if view=="invalid" {assert!(x.sum_keepdim(axes).is_err());continue;}
                let mut out_shape=x.dims().to_vec();for &axis in axes{out_shape[axis]=1;}
                let mut oracle=vec![0f64;out_shape.iter().product()];
                // Derive each original storage index from the view stride/offset.
                // No Tensor reduction or contiguous-copy helper enters this oracle.
                for linear in 0..x.elem_count() {
                    let mut remaining=linear;let mut storage=x.layout().start_offset();
                    let mut output=0;let mut multiplier=1;
                    for axis in (0..x.rank()).rev(){
                        let coordinate=remaining%x.dims()[axis];remaining/=x.dims()[axis];
                        storage+=coordinate*x.stride()[axis];
                        if !axes.contains(&axis){output+=coordinate*multiplier;}
                        multiplier*=out_shape[axis];
                    }
                    oracle[output]+=raw[storage]as f64;
                }
                println!("LAYOUT {case} device={:?} shape={:?} stride={:?} offset={} axes={axes:?} entering_sum",device.location(),x.dims(),x.stride(),x.layout().start_offset());
                let y=x.sum_keepdim(axes)?;device.synchronize()?;
                assert_eq!(y.dims(),out_shape);
                let expected=Tensor::from_vec(oracle.iter().map(|&v|v as f32).collect::<Vec<_>>(),out_shape,&Device::Cpu)?;
                close(&format!("layout/{case}"),&expected,&y,1e-4,1e-3)?;
                outputs.push(y);
            }
            println!("LAYOUT {case} PASS view={view} reductions={}",outputs.len());
        }
        // Malformed view bounds fail before dispatch or allocation on both devices.
        for device in [&Device::Cpu,&gpu] {
            let x=Tensor::zeros((2,3),DType::F32,device)?;
            assert!(x.narrow(0,2,1).is_err());
            assert!(x.permute([0,0]).is_err());
            assert!(x.broadcast_as((3,3)).is_err());
        }
        println!("LAYOUT_SUITE cases22 native0 optimizer0");Ok(())
    }
    #[test]
    #[ignore = "analytic K/V repeat VJP and actual Q/K/V Var GQA"]
    fn metal_f32_repeat_and_qkv_vjp()->Result<()> {
        use neural::transformer::{repeat_kv,gqa_attention,attention_mask};
        let gpu=neural::Backend::Metal0.open()?;
        for group in [2,4] {for kind in ["K","V"] {
            let mut upstream:Vec<f32>=(0..8*2*group*12*8).map(|i|((i%41)as f32-20.)/32.).collect();
            if kind=="V"{upstream.reverse();}
            let mut oracle=Vec::new();
            for b in 0..8{for h in 0..2{for t in 0..12{for d in 0..8{
                oracle.push((0..group).map(|g|upstream[(((b*2+h)*group+g)*12+t)*8+d]as f64).sum::<f64>()as f32);
            }}}}
            let expected=Tensor::from_vec(oracle,(8,2,12,8),&Device::Cpu)?;
            for device in [&Device::Cpu,&gpu] {
                let x=Var::from_vec((0..8*2*12*8).map(|i|(i%31)as f32/31.).collect::<Vec<_>>(),(8,2,12,8),device)?;
                let y=repeat_kv(&x,2*group)?;
                let u=Tensor::from_vec(upstream.clone(),y.shape(),device)?;
                let grad=(&y*&u)?.sum_all()?.backward()?.get(&x).unwrap().detach();
                device.synchronize()?;
                vector(&format!("analytic-{kind}/G{group}/{:?}",device.location()),&expected,&grad,1e-2,1e-6,true)?;
                assert_eq!(expected.flatten_all()?.to_vec1::<f32>()?,grad.flatten_all()?.to_vec1::<f32>()?);
            }
        }}
        let mut outcomes=Vec::new();
        for device in [&Device::Cpu,&gpu] {
            let q=Var::from_vec((0..2*4*5*8).map(|i|((i%37)as f32-18.)/29.).collect::<Vec<_>>(),(2,4,5,8),device)?;
            let k=Var::from_vec((0..2*2*5*8).map(|i|((i%31)as f32-15.)/23.).collect::<Vec<_>>(),(2,2,5,8),device)?;
            let v=Var::from_vec((0..2*2*5*8).map(|i|((i%43)as f32-21.)/33.).collect::<Vec<_>>(),(2,2,5,8),device)?;
            let mask=attention_mask(0..5,0..5,Some(3),None,2,device)?;
            let y=gqa_attention(&q,&k,&v,&mask)?;
            let u=Tensor::from_vec((0..y.elem_count()).map(|i|((i%41)as f32-20.)/32.).collect::<Vec<_>>(),y.shape(),device)?;
            let grad=(&y*&u)?.sum_all()?.backward()?;device.synchronize()?;
            outcomes.push((y,grad.get(&q).unwrap().detach(),grad.get(&k).unwrap().detach(),grad.get(&v).unwrap().detach()));
        }
        close("GQA-forward-all-vars",&outcomes[0].0,&outcomes[1].0,1e-4,1e-3)?;
        for (name,c,m) in [("Q",&outcomes[0].1,&outcomes[1].1),("K",&outcomes[0].2,&outcomes[1].2),("V",&outcomes[0].3,&outcomes[1].3)] {
            vector(&format!("GQA-{name}"),c,m,1e-2,1e-6,true)?;
        }
        println!("VJP_SUITE backward10 native0 optimizer0");Ok(())
    }
    #[test]
    #[ignore = "explicit actual Metal F32 numerical gate; two TINY optimizer calls maximum"]
    fn metal_f32_tiny_forward_gradient_update()->Result<()> {
        let gpu=neural::Backend::Metal0.open()?;
        assert!(gpu.is_metal());
        println!("ACTUAL_DEVICE {:?} dtype=F32 fallback=0 Candle=0.11.0",gpu.location());
        let cpu=Transformer::init(Config::tiny(264),20260924,Device::Cpu)?;
        let tensors=cpu.vars.iter().map(|(n,v)|Ok((n.clone(),v.as_detached_tensor().to_device(&gpu)?))).collect::<Result<BTreeMap<_,_>>>()?;
        let mut metal=Transformer::from_tensors(cpu.config.clone(),tensors,gpu.clone())?;
        assert_eq!(cpu.weights_content_id()?,metal.weights_content_id()?);
        assert_eq!(cpu.config.semantic_id()?,metal.config.semantic_id()?);
        let mut ss=vec![];
        for row in 0..8 {ss.push(Sample{tokens:(0..12-row%3).map(|n|8+(n+row)%32).chain([EOS]).collect(),response_start:7,curriculum:false});}
        let indices:Vec<_>=(0..8).collect();
        let cb=batch(&ss,&indices,&Device::Cpu)?;let mb=batch(&ss,&indices,&gpu)?;
        assert_eq!(cb.valid,mb.valid);assert_eq!(cb.tokens,mb.tokens);
        assert_eq!(cb.input.to_vec2::<u32>()?,mb.input.to_vec2::<u32>()?);
        assert_eq!(cb.mask.to_vec2::<f32>()?,mb.mask.to_vec2::<f32>()?);
        let cl=cpu.forward(&cb.input,Some(&cb.valid))?;
        let ml=metal.forward(&mb.input,Some(&mb.valid))?;gpu.synchronize()?;
        close("padded-forward8x12x264",&cl,&ml,1e-4,1e-3)?;
        let (cce,co,ct,cn)=response_objective(&cl,&cb,1.,true)?;
        let (mce,mo,mt,mn)=response_objective(&ml,&mb,1.,true)?;
        assert_eq!((ct,cn),(mt,mn));assert_eq!(cn,8);
        close("TOKEN-CE",&cce,&mce,1e-5,1e-3)?;
        close("ANSWER-CE",&co,&mo,1e-5,1e-3)?;
        let mut full_gradients=Vec::new();
        for (label,closs,mloss) in [("TOKEN",&cce,&mce),("ANSWER",&co,&mo)] {
            println!("BACKWARD entering CPU/TINY1 and Metal/TINY1 objective={label}; optimizer0 generation0 teacher0");
            let cg=closs.backward()?;let mg=mloss.backward()?;gpu.synchronize()?;
            let mut cgmap=BTreeMap::new();let mut mgmap=BTreeMap::new();
            for(n,v)in &cpu.vars {
                let c=cg.get(v).expect("connected CPU leaf").detach();
                let m=mg.get(&metal.vars[n]).expect("connected Metal leaf").detach();
                assert!(m.device().same_device(&gpu));
                vector(&format!("gradient/{label}/{n}"),&c,&m,1e-2,1e-6,true)?;
                cgmap.insert(n.clone(),c);mgmap.insert(n.clone(),m);
            }
            full_gradients.push((cgmap,mgmap));
        }
        // Preserve each objective's actual mask denominator across padded 4+4.
        for (device_index,model) in [&cpu,&metal].into_iter().enumerate() {
            let mut accumulated:[BTreeMap<String,Tensor>;2]=Default::default();
            for start in [0,4] {
                let micro=batch(&ss,&(start..start+4).collect::<Vec<_>>(),&model.device)?;
                let logits=model.forward(&micro.input,Some(&micro.valid))?;
                let (token,answer,nt,ne)=response_objective(&logits,&micro,1.,true)?;
                for (objective,loss,weight) in [(0,&token,nt as f64/ct as f64),(1,&answer,ne as f64/cn as f64)] {
                    println!("MICROBATCH device={:?} start={start} objective={objective} target={nt} examples={ne} backward_enter",model.device.location());
                    let gradients=loss.backward()?;
                    for (name,var) in &model.vars {
                        let weighted=(gradients.get(var).unwrap()*weight)?;
                        let next=if let Some(old)=accumulated[objective].get(name){(old+weighted)?}else{weighted};
                        accumulated[objective].insert(name.clone(),next);
                    }
                }
            }
            model.device.synchronize()?;
            for objective in 0..2 {for name in model.vars.keys() {
                let full=if device_index==0 {&full_gradients[objective].0[name]} else {&full_gradients[objective].1[name]};
                vector(&format!("micro4+4/{device_index}/{objective}/{name}"),full,&accumulated[objective][name],1e-2,1e-6,true)?;
            }}
        }
        let (cgmap,mgmap)=full_gradients.pop().unwrap();
        let mut foreign=cpu.cache("gate");
        let ids=mb.input.narrow(0,0,1)?;
        assert!(metal.forward_cached(&ids,&mut foreign,"gate").is_err());
        metal.set_kernel(Kernel::RustDecodeGemv);
        assert!(metal.forward(&ids,None).is_err());metal.set_kernel(Kernel::Reference);
        let mut cache=metal.cache("gate");let full=metal.forward(&ids,None)?;
        for pos in 0..ids.dim(1)? {
            let got=metal.forward_cached(&ids.narrow(1,pos,1)?,&mut cache,"gate")?;
            close(&format!("token-cache/{pos}"),&full.narrow(1,pos,1)?,&got,1e-4,1e-3)?;
        }
        let original=cpu.vars.iter().map(|(n,v)|Ok((n.clone(),v.as_detached_tensor().copy()?))).collect::<Result<BTreeMap<_,_>>>()?;
        let cfg=TrainConfig{lr:3e-5,..Default::default()};
        let mut ca=Adam::new(&cpu.vars)?;let mut ma=Adam::new(&metal.vars)?;
        println!("UPDATE entering CPU/TINY1 and Metal/TINY1; generation0 teacher0");
        let cstats=ca.step_constant(&cpu.vars,&cgmap,&cfg,1,3e-5)?;
        // Test-only entry into the SAME Adam body after full TOKEN/ANSWER and
        // microbatch gates. No production caller or environment bypass exists.
        let mstats=ma.apply_admitted_step(&metal.vars,&mgmap,&cfg,1,3e-5,|_,_,_,_|Ok(()))?;gpu.synchronize()?;
        println!("UPDATE CPU={cstats:?} Metal={mstats:?} committed=2");
        for(n,v)in &cpu.vars {
            vector(&format!("delta/{n}"),&(v.as_tensor()-&original[n])?,&(&metal.vars[n].to_device(&Device::Cpu)?-&original[n])?,2e-2,1e-7,false)?;
        }
        for(n,v)in &ca.moments {vector(n,v,&ma.moments[n],1e-2,1e-6,false)?;assert!(ma.moments[n].device().same_device(&gpu));}
        println!("METAL_TINY_NUMERIC_PASS tiny_optimizer2 tiny_backward12 tiny_forward19 generation0 teacher0 clock1");Ok(())
    }
}
impl Adam {
    fn proposal(old:&Tensor,g:&Tensor,previous_m:&Tensor,previous_v:&Tensor,c:&TrainConfig,step:usize,lr:f64)->Result<(Tensor,Tensor,Tensor)> {
        let m=((previous_m*c.beta1)?+(g*(1.-c.beta1))?)?;
        let v=((previous_v*c.beta2)?+(g.sqr()?*(1.-c.beta2))?)?;
        let update=((&m/(1.-c.beta1.powi(step as i32)))?/((&v/(1.-c.beta2.powi(step as i32)))?.sqrt()?+c.eps)?)?;
        let next=((old*(1.-lr*c.weight_decay))?-(update*lr)?)?;
        Ok((next.detach(),m.detach(),v.detach()))
    }
    pub fn new(vars: &BTreeMap<String, Var>) -> Result<Self> {
        let mut moments = BTreeMap::new();
        for (name, var) in vars {
            moments.insert(
                format!("adam.m.{name}"),
                Tensor::zeros(var.dims(), DType::F32, var.device())?,
            );
            moments.insert(
                format!("adam.v.{name}"),
                Tensor::zeros(var.dims(), DType::F32, var.device())?,
            );
        }
        Ok(Self { moments })
    }
    pub fn step(
        &mut self,
        vars: &BTreeMap<String, Var>,
        grads: &BTreeMap<String, Tensor>,
        config: &TrainConfig,
        step: usize,
    ) -> Result<(f64, f64)> {
        self.step_observed(vars, grads, config, step, |_, _, _, _| Ok(()))
    }
    pub fn step_observed(
        &mut self,
        vars: &BTreeMap<String, Var>,
        grads: &BTreeMap<String, Tensor>,
        config: &TrainConfig,
        step: usize,
        observe: impl FnMut(&str, &Tensor, &Tensor, &Tensor) -> Result<()>,
    ) -> Result<(f64, f64)> {
        self.step_with_rate(
            vars,
            grads,
            config,
            step,
            config.learning_rate(step),
            observe,
        )
    }
    pub fn step_constant(
        &mut self,
        vars: &BTreeMap<String, Var>,
        grads: &BTreeMap<String, Tensor>,
        config: &TrainConfig,
        step: usize,
        rate: f64,
    ) -> Result<(f64, f64)> {
        self.step_with_rate(vars, grads, config, step, rate, |_, _, _, _| Ok(()))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn step_constant_observed(
        &mut self,
        vars: &BTreeMap<String, Var>,
        grads: &BTreeMap<String, Tensor>,
        config: &TrainConfig,
        step: usize,
        rate: f64,
        observe: impl FnMut(&str, &Tensor, &Tensor, &Tensor) -> Result<()>,
    ) -> Result<(f64, f64)> {
        self.step_with_rate(vars, grads, config, step, rate, observe)
    }
    // Only the rate policy differs; moments, clipping, decay and cumulative Adam clock are shared.
    #[allow(clippy::too_many_arguments)]
    fn step_with_rate(
        &mut self,
        vars: &BTreeMap<String, Var>,
        grads: &BTreeMap<String, Tensor>,
        config: &TrainConfig,
        step: usize,
        lr: f64,
        observe: impl FnMut(&str, &Tensor, &Tensor, &Tensor) -> Result<()>,
    ) -> Result<(f64, f64)> {
        if !lr.is_finite() || lr <= 0. || lr > 0.1 || step == 0 || step > i32::MAX as usize {
            return Err(Error::Invalid("optimizer rate/clock".into()));
        }
        if vars.values().any(|v|v.device().is_metal()) {
            return Err(Error::Unsupported("METAL_F32_TRAINING_NOT_ACCEPTED: verified parent-bound runtime profile required; optimizer_calls=0".into()));
        }
        self.apply_admitted_step(vars,grads,config,step,lr,observe)
    }
    // Admission stays above this private shared implementation. Numerical tests
    // may enter only after their complete gradient gate on the fixed TINY fixture.
    #[allow(clippy::too_many_arguments)]
    fn apply_admitted_step(
        &mut self,
        vars: &BTreeMap<String, Var>,
        grads: &BTreeMap<String, Tensor>,
        config: &TrainConfig,
        step: usize,
        lr: f64,
        mut observe: impl FnMut(&str, &Tensor, &Tensor, &Tensor) -> Result<()>,
    ) -> Result<(f64, f64)> {
        let mut norm2 = 0f64;
        for name in vars.keys() {
            let grad = grads
                .get(name)
                .ok_or_else(|| Error::Model(format!("disconnected trainable variable {name}")))?;
            let square = grad.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
            if !square.is_finite() {
                return Err(Error::Model("nonfinite gradient".into()));
            }
            norm2 += square;
        }
        let norm = norm2.sqrt();
        let clip = (config.clip / (norm + 1e-12)).min(1.);
        let mut updates = Vec::new();
        let mut delta2 = 0f64;
        for (name, var) in vars {
            let g = (&grads[name] * clip)?;
            let old = var.as_detached_tensor();
            let (next,m,v)=Self::proposal(&old,&g,&self.moments[&format!("adam.m.{name}")],&self.moments[&format!("adam.v.{name}")],config,step,lr)?;
            let delta = (&next - &old)?.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
            if !delta.is_finite() {
                return Err(Error::Model("nonfinite optimizer update".into()));
            }
            delta2 += delta;
            observe(name, &grads[name], &old, &next)?;
            updates.push((name.clone(), next.detach(), m.detach(), v.detach()));
        }
        // Validate/allocate the complete step before touching any master weight.
        for (name, next, m, v) in updates {
            vars[&name].set(&next)?;
            self.moments.insert(format!("adam.m.{name}"), m);
            self.moments.insert(format!("adam.v.{name}"), v);
        }
        Ok((norm, delta2.sqrt()))
    }
}
#[derive(Clone)]
pub struct Sample {
    pub tokens: Vec<u32>,
    pub response_start: usize,
    pub curriculum: bool,
}
pub fn samples(episodes: &[Episode], tok: &ByteBpe, seq_len: usize) -> Result<Vec<Sample>> {
    samples_with_framing(episodes, tok, seq_len, neural::Framing::QuestionEvidence)
}
pub fn samples_with_framing(
    episodes: &[Episode],
    tok: &ByteBpe,
    seq_len: usize,
    framing: neural::Framing,
) -> Result<Vec<Sample>> {
    let mut out = Vec::new();
    for e in episodes {
        if e.answer.is_empty() {
            let mut tokens = vec![BOS];
            tokens.extend(tok.encode(e.request.input.as_bytes())?);
            tokens.push(EOS);
            // Episode split precedes chunking. One boundary token is context only.
            for start in (0..tokens.len() - 1).step_by(seq_len) {
                let end = (start + seq_len + 1).min(tokens.len());
                out.push(Sample {
                    tokens: tokens[start..end].to_vec(),
                    response_start: 1,
                    curriculum: false,
                });
            }
        } else {
            let answer = tok.encode(e.answer.as_bytes())?;
            let mut request = e.request.clone();
            request.limits.context_tokens = seq_len as u32;
            request.limits.max_tokens = (answer.len() + 1) as u32;
            let prompt = tok.prepare_with_framing(&request, framing, seq_len as u32, "training")?;
            if citations(&e.answer)?
                .iter()
                .any(|id| !prompt.provided.contains(id))
            {
                return Err(Error::Invalid(format!(
                    "training evidence excluded for {} at length {seq_len}",
                    e.id
                )));
            }
            let response_start = prompt.token_ids.len();
            let mut tokens = prompt.token_ids;
            tokens.extend(answer);
            tokens.push(EOS);
            out.push(Sample {
                tokens,
                response_start,
                curriculum: e.family.starts_with("copy/"),
            });
        }
    }
    if out.is_empty() {
        return Err(Error::Invalid("empty training samples".into()));
    }
    Ok(out)
}
fn numeric_samples(tok: &ByteBpe) -> Result<Vec<Sample>> {
    ["가 나 다 가 나 다", "오른쪽 왼쪽 오른쪽 왼쪽"]
        .iter()
        .map(|s| {
            let mut tokens = vec![BOS];
            tokens.extend(tok.encode(s.as_bytes())?);
            tokens.push(EOS);
            Ok(Sample {
                tokens,
                response_start: 1,
                curriculum: false,
            })
        })
        .collect()
}
struct Batch {
    input: Tensor,
    target: Tensor,
    mask: Tensor,
    first_target_mask: Tensor,
    valid: Vec<bool>,
    tokens: usize,
}
fn batch(samples: &[Sample], indices: &[usize], device: &Device) -> Result<Batch> {
    let len = indices
        .iter()
        .map(|&i| samples[i].tokens.len() - 1)
        .max()
        .ok_or_else(|| Error::Invalid("empty batch".into()))?;
    let mut input = vec![PAD; indices.len() * len];
    let mut target = input.clone();
    let mut mask = vec![0f32; input.len()];
    let mut first_target_mask = mask.clone();
    let mut valid = vec![false; input.len()];
    let mut tokens = 0;
    for (row, &index) in indices.iter().enumerate() {
        let sample = &samples[index];
        if sample.tokens.len() < 2 || sample.response_start >= sample.tokens.len() {
            return Err(Error::Invalid("sample/teacher forcing boundary".into()));
        }
        for p in 0..sample.tokens.len() - 1 {
            let i = row * len + p;
            input[i] = sample.tokens[p];
            target[i] = sample.tokens[p + 1];
            valid[i] = true;
            mask[i] = f32::from((p + 1 >= sample.response_start) as u8);
            first_target_mask[i] = f32::from((p + 1 == sample.response_start.max(1)) as u8);
            tokens += 1;
        }
    }
    Ok(Batch {
        input: Tensor::from_vec(input, (indices.len(), len), device)?,
        target: Tensor::from_vec(target, (indices.len(), len), device)?,
        mask: Tensor::from_vec(mask, (indices.len(), len), device)?,
        first_target_mask: Tensor::from_vec(first_target_mask, (indices.len(), len), device)?,
        valid,
        tokens,
    })
}
// Both losses use the real number of target tokens. The optional objective adds
// (weight-1)*NLL only at each sample's first supervised position; it never weights
// padding/prompt tokens or conditions on a token value, question, seed or answer.
fn response_loss(logits: &Tensor, batch: &Batch, weight: f64) -> Result<(Tensor, Tensor, usize)> {
    if !weight.is_finite() || !(1.0..=16.0).contains(&weight) {
        return Err(Error::Invalid("first target weight 1..16".into()));
    }
    let (ce, n) = masked_loss(logits, &batch.target, &batch.mask)?;
    let objective = if weight == 1. {
        ce.clone()
    } else {
        let (first, count) = masked_loss(logits, &batch.target, &batch.first_target_mask)?;
        (&ce + (first * ((weight - 1.) * count as f64 / n as f64))?)?
    };
    Ok((ce, objective, n))
}
// Response masks, including EOS, define each example's mass. This reduction has
// no task/token/annotation-dependent weights. Keep the historical TOKEN path's
// arithmetic and its target-count accumulation unchanged.
fn answer_mean_loss(logits: &Tensor, batch: &Batch) -> Result<Tensor> {
    let examples = batch.mask.dim(0)?;
    if examples == 0 { return Err(Error::Invalid("empty answer batch".into())); }
    let mut means = Vec::with_capacity(examples);
    for row in 0..examples {
        let mask = batch.mask.narrow(0, row, 1)?;
        if mask.to_vec2::<f32>()?.iter().flatten().any(|&v| v != 0. && v != 1.) {
            return Err(Error::Invalid("answer mask must contain only zero or one".into()));
        }
        let (ce, n) = masked_loss(&logits.narrow(0,row,1)?, &batch.target.narrow(0,row,1)?, &mask)?;
        if n == 0 || !ce.to_scalar::<f32>()?.is_finite() {
            return Err(Error::Invalid("empty/nonfinite supervised answer".into()));
        }
        means.push(ce);
    }
    Ok(Tensor::stack(&means,0)?.mean_all()?)
}
fn response_objective(logits: &Tensor, batch: &Batch, weight: f64, answer: bool)
    -> Result<(Tensor, Tensor, usize, usize)> {
    let (ce, token_objective, targets) = response_loss(logits,batch,weight)?;
    if !ce.to_scalar::<f32>()?.is_finite() { return Err(Error::Invalid("nonfinite response CE".into())); }
    if batch.mask.sum(1)?.to_vec1::<f32>()?.iter().any(|n|*n<=0. || !n.is_finite()) {
        return Err(Error::Invalid("empty supervised answer".into()));
    }
    if !answer { return Ok((ce,token_objective,targets,targets)); }
    if weight != 1. { return Err(Error::Invalid("answer mean requires unweighted response CE".into())); }
    Ok((ce,answer_mean_loss(logits,batch)?,targets,batch.mask.dim(0)?))
}
// Training-only paired supervision. The two targets share their prefix up to
// the first divergence, so neither teacher prefix reveals the selected side.
// No extra forward, generated answer correction, or product inference oracle.
fn pair_contrast_loss(
    logits: &Tensor,
    samples: &[Sample],
    indices: &[usize],
    pairs: &[(usize, usize)],
    sidewise: bool,
) -> Result<Tensor> {
    let (rows, len, vocab) = logits.dims3()?;
    if rows != indices.len() || indices.iter().any(|&i| i >= samples.len()) {
        return Err(Error::Invalid("paired loss batch bounds".into()));
    }
    let zero = Tensor::zeros((), DType::F32, logits.device())?;
    let mut losses = Vec::with_capacity(pairs.len());
    let mut seen = std::collections::BTreeSet::new();
    for &(a, b) in pairs {
        if a >= rows || b >= rows || a == b || !seen.insert(a) || !seen.insert(b) {
            return Err(Error::Invalid("paired loss duplicate/self/out-of-range row".into()));
        }
        let sa = &samples[indices[a]];
        let sb = &samples[indices[b]];
        if sa.response_start == 0 || sb.response_start == 0
            || sa.response_start >= sa.tokens.len() || sb.response_start >= sb.tokens.len()
            || sa.tokens.last() != Some(&EOS) || sb.tokens.last() != Some(&EOS) {
            return Err(Error::Invalid("paired loss target boundary/EOS".into()));
        }
        let ta = &sa.tokens[sa.response_start..];
        let tb = &sb.tokens[sb.response_start..];
        let j = ta.iter().zip(tb).position(|(x,y)| x != y)
            .ok_or_else(|| Error::Invalid("paired loss requires distinct targets".into()))?;
        let (pa,pb) = (sa.response_start+j-1, sb.response_start+j-1);
        let (ya,yb) = (ta[j] as usize,tb[j] as usize);
        if pa >= len || pb >= len || ya >= vocab || yb >= vocab {
            return Err(Error::Invalid("paired loss position/token bounds".into()));
        }
        let at = |r,p,y| logits.narrow(0,r,1)?.narrow(1,p,1)?.narrow(2,y,1)?.reshape(());
        let da = (at(a,pa,ya)? - at(a,pa,yb)?)?;
        let db = (at(b,pb,yb)? - at(b,pb,ya)?)?;
        // log(exp(0)+exp(1-d)), using the backend's stable log-sum-exp.
        let penalty = |d: &Tensor| -> candle_core::Result<Tensor> {
            Tensor::stack(&[&zero,&(d.neg()? + 1.)?],0)?.log_sum_exp(0)
        };
        losses.push(if sidewise { ((penalty(&da)? + penalty(&db)?)? * 0.5)? }
            else { penalty(&(da + db)?)? });
    }
    Ok(if losses.is_empty() { zero } else { (Tensor::stack(&losses,0)?.mean_all()? * 0.1)? })
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(super) struct RecordGrounding {
    query: usize,
    records: [(usize, usize); 2],
    selected: usize,
}
impl RecordGrounding {
    pub(super) fn from_sample(sample: &Sample, selected: usize) -> Result<Self> {
        let prefix = sample.tokens.get(..sample.response_start)
            .ok_or_else(|| Error::Invalid("grounding response boundary".into()))?;
        let roles: Vec<_> = prefix.iter().copied().filter(|&t|t < neural::SPECIALS as u32).collect();
        if selected > 1 || roles != [BOS,neural::SYSTEM_ROLE,neural::END_ROLE,neural::USER_ROLE,
            neural::END_ROLE,neural::EVIDENCE_ROLE,neural::END_ROLE,neural::EVIDENCE_ROLE,
            neural::END_ROLE,neural::ASSISTANT_ROLE] || sample.tokens.last()!=Some(&EOS) {
            return Err(Error::Invalid("grounding requires exact two-record prompt framing".into()));
        }
        let spans: Vec<_> = prefix.iter().enumerate().filter(|(_,t)|**t==neural::EVIDENCE_ROLE)
            .map(|(i,_)| {
                let end=i+1+prefix[i+1..].iter().position(|&t|t==neural::END_ROLE).unwrap();
                (i+1,end)
            }).collect();
        let value=Self{query:prefix.len()-1,records:[spans[0],spans[1]],selected};
        value.validate(prefix.len())?;
        Ok(value)
    }
    fn validate(&self, len: usize) -> Result<()> {
        if self.selected>1 || self.query>=len || self.records.iter().any(|&(a,b)|a>=b || b>self.query)
            || self.records[0].1>self.records[1].0 {
            return Err(Error::Invalid("grounding annotation position/span bounds".into()));
        }
        Ok(())
    }
}
// The existing last-layer attention at the first response query supplies this
// training-only loss. Labels never change token input, masks or inference.
fn record_grounding_loss(q:&Tensor,k:&Tensor,allowed:&Tensor,labels:&[Option<RecordGrounding>]) -> Result<Tensor> {
    let (batch,heads,len,dim)=q.dims4()?;
    let (kb,kh,kl,kd)=k.dims4()?;
    if labels.len()!=batch || kb!=batch || kl!=len || kd!=dim || dim==0 || kh==0
        || heads==0 || !heads.is_multiple_of(kh) || allowed.dims()!=[batch,1,len,len] {
        return Err(Error::Invalid("grounding attention shape".into()));
    }
    let mut losses=vec![];
    for (row,label) in labels.iter().enumerate() {
        let Some(label)=label else {continue;};label.validate(len)?;
        let mask=allowed.narrow(0,row,1)?.narrow(2,label.query,1)?;
        let visible=mask.flatten_all()?.to_vec1::<f32>()?;
        if label.records.iter().any(|&(a,b)|visible[a..b].iter().any(|&v|v!=1.)) {
            return Err(Error::Invalid("grounding selected evidence masked or padded".into()));
        }
        let keys=neural::transformer::repeat_kv(&k.narrow(0,row,1)?,heads)?;
        let scores=(q.narrow(0,row,1)?.narrow(2,label.query,1)?.contiguous()?
            .matmul(&keys.transpose(2,3)?.contiguous()?)?/(dim as f64).sqrt())?
            .broadcast_add(&((&mask-1.)?*1e9)?)?;
        let attention=candle_nn::ops::softmax(&scores,candle_core::D::Minus1)?.broadcast_mul(&mask)?;
        let mass=|slot:usize| {let (a,b)=label.records[slot];attention.narrow(3,a,b-a)?.sum_all()};
        let a=mass(label.selected)?;let b=(&a+mass(1-label.selected)?)?;
        let denominator=b.to_scalar::<f32>()?;
        if !denominator.is_finite() || denominator<=0. {return Err(Error::Model("nonfinite/zero grounding record mass".into()));}
        losses.push((a/b)?.clamp(1e-8,1.)?.log()?.neg()?);
    }
    Ok(if losses.is_empty() {Tensor::zeros((),DType::F32,q.device())?}
        else {(Tensor::stack(&losses,0)?.mean_all()?*0.1)?})
}
fn validation_loss(
    model: &Transformer,
    samples: &[Sample],
    control: &mut recovery::RunControl,
) -> Result<f64> {
    let mut total = 0f64;
    let mut targets = 0;
    for index in 0..samples.len() {
        control.check("before_validation_teacher")?;
        let b = batch(samples, &[index], &model.device)?;
        control.begin_teacher()?;
        let (loss, n) = masked_loss(
            &model.forward(&b.input, Some(&b.valid))?,
            &b.target,
            &b.mask,
        )?;
        total += f64::from(loss.to_scalar::<f32>()?) * n as f64;
        targets += n;
        control.check("validation_teacher_returned")?;
    }
    let loss = total / targets as f64;
    if !loss.is_finite() {
        return Err(Error::Model("nonfinite validation loss".into()));
    }
    Ok(loss)
}
// An oracle paraphrase ablation of already explicit query targets. No expected
// answer, evidence value or citation ID is accepted by this diagnostic helper.
// It is private to the training executable, never part of a product request.
fn mentions_target(question: &str, target: &str) -> bool {
    !target.is_empty()
        && question.match_indices(target).any(|(start, matched)| {
            (!target.as_bytes()[0].is_ascii_digit()
                || start == 0
                || !question.as_bytes()[start - 1].is_ascii_digit())
                && question[start + matched.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_ascii_digit())
        })
}
fn known_question_form(
    category: usize,
    original: &str,
    entity: &str,
    context: &str,
) -> Result<String> {
    if !mentions_target(original, entity) || !mentions_target(original, context) {
        return Err(Error::Invalid(
            "ablation target must be explicit in the original question".into(),
        ));
    }
    match category {
        0 => Ok(format!("{entity} {context} 현재 이동 지시는 무엇인가?")),
        2 => Ok(format!("{entity}는 {context}에서 어느 방향으로 가야 하나?")),
        _ => Err(Error::Invalid(
            "question-form ablation is limited to QA0/QA2".into(),
        )),
    }
}
// The field is an explicit oracle task label for this ablation. Gold answers and
// record contents are not arguments; binding identifiers must occur in the question.
fn known_field_question_form(
    original: &str,
    field: &str,
    entity: &str,
    context: &str,
) -> Result<String> {
    if field == "entity-cue" {
        return Ok("제공된 원문에서 숫자 앞에 적힌 대상의 분류명만 그대로 복사해줘.".into());
    }
    let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
    let target = if field == "value" { number } else { entity };
    if !mentions_target(original, target) || !mentions_target(original, context) {
        return Err(Error::Invalid(
            "field ablation target must be explicit in the original question".into(),
        ));
    }
    match field {
        "number" => Ok(format!(
            "{context} 자료의 대상 {entity}에서 분류명 뒤 숫자만 원문대로 적어줘."
        )),
        "context" => Ok(format!(
            "제공 기록에서 {entity}의 {context} 위치 이름 전체만 복사하라."
        )),
        "value" => Ok(format!(
            "식별 숫자가 {number}인 대상의 {context} current 원문에서 '이동 지시는' 다음 방향 단어만 복사해줘."
        )),
        _ => Err(Error::Invalid("unsupported oracle field task".into())),
    }
}
pub enum FieldAblation {
    None,
    Question,
    Record,
    QuestionAndRecord,
    QaRecord,
}
#[derive(Clone, Copy)]
enum RecordTargetMode {
    ExactEntity,
    ExplicitNumericField,
}
// Oracle evidence-selection diagnostic, never retrieval or product inference.
// Selection reads only the explicit identifiers and current status, not gold values.
fn isolate_current_record(
    request: &mut replica_v3::model::ModelRequest,
    entity: &str,
    context: &str,
    mode: RecordTargetMode,
) -> Result<()> {
    let number = entity.trim_start_matches(|c: char| !c.is_ascii_digit());
    let target = match mode {
        RecordTargetMode::ExactEntity => entity,
        RecordTargetMode::ExplicitNumericField => number,
    };
    let exact_mention = |atom: &str| {
        mentions_target(&request.input, atom)
            && request.input.match_indices(atom).any(|(start, _)| {
                request.input[..start]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !c.is_alphanumeric())
                    && request.input[start + atom.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| !c.is_ascii_digit())
            })
    };
    if entity.is_empty()
        || context.is_empty()
        || number.is_empty()
        || !number.bytes().all(|b| b.is_ascii_digit())
        || !mentions_target(&request.input, target)
        || !exact_mention(context)
        || (matches!(mode, RecordTargetMode::ExactEntity) && !exact_mention(entity))
    {
        return Err(Error::Invalid(
            "INVALID_DIAGNOSTIC_INPUT: record target must be explicit in the question".into(),
        ));
    }
    let matches: Vec<_> = request
        .evidence
        .items
        .iter()
        .filter(|e| {
            e.version_status == "current"
                && e.original_excerpt
                    .split_once("의 ")
                    .is_some_and(|(name, text)| {
                        name.trim_start_matches(|c: char| !c.is_ascii_digit()) == number
                            && (matches!(mode, RecordTargetMode::ExplicitNumericField)
                                || name == entity)
                            && text.starts_with(&format!("{context} 이동 지시는 "))
                    })
                && !e.excerpt_truncated
        })
        .cloned()
        .collect();
    if matches.len() != 1 {
        return Err(Error::Invalid(
            "INVALID_DIAGNOSTIC_INPUT: record ablation requires exactly one current match".into(),
        ));
    }
    request.evidence.items = matches;
    Ok(())
}
/// Diagnostic on a predeclared corpus split, not the independent final test.
pub fn evaluate_corpus(
    checkpoint: &Path,
    corpus: &Path,
    output: &Path,
    limit: usize,
    split: &str,
    rephrase: bool,
    field_ablation: FieldAblation,
) -> Result<()> {
    use std::io::Write;
    let mut control = recovery::RunControl::command(false)?;
    control.check("evaluate_corpus_started")?;
    if !rephrase && matches!(field_ablation, FieldAblation::None) {
        return recovery::evaluate_native_source(
            checkpoint,
            corpus,
            output,
            limit,
            split,
            &mut control,
        );
    }
    let rephrase_field = matches!(
        field_ablation,
        FieldAblation::Question | FieldAblation::QuestionAndRecord
    );
    let qa_record = matches!(field_ablation, FieldAblation::QaRecord);
    let single_record = matches!(
        field_ablation,
        FieldAblation::Record | FieldAblation::QuestionAndRecord | FieldAblation::QaRecord
    );
    let (manifest, train, validation) = data::load(corpus)?;
    control.check("evaluate_corpus_loaded")?;
    let (episodes, split_hash) = match split {
        "train" => (train, &manifest.train.sha256),
        "validation" => (validation, &manifest.validation.sha256),
        _ => return Err(Error::Invalid("diagnostic split".into())),
    };
    if ((rephrase || rephrase_field || single_record) && split != "validation")
        || (rephrase && (rephrase_field || (single_record && !qa_record)))
    {
        return Err(Error::Invalid(
            "question-form ablation requires validation".into(),
        ));
    }
    let episodes: Vec<_> = episodes
        .iter()
        .filter(|episode| {
            if qa_record {
                matches!(episode.category, 0 | 2) && !episode.family.starts_with("copy/")
            } else if single_record {
                episode.family.starts_with("copy/") && episode.family.ends_with("/value")
            } else if rephrase_field {
                episode.family.starts_with("copy/")
            } else {
                !rephrase
                    || (matches!(episode.category, 0 | 2) && !episode.family.starts_with("copy/"))
            }
        })
        .collect();
    if limit == 0 || limit > episodes.len() {
        return Err(Error::Invalid("diagnostic generation count".into()));
    }
    // Validate every requested diagnostic transformation before loading a model or publishing rows.
    let mut requests = Vec::with_capacity(limit);
    for episode in episodes.iter().take(limit) {
        let mut request = episode.request.clone();
        let mut fields = episode.binding.split('/');
        let entity = fields.next().unwrap_or_default();
        let context = fields.next().unwrap_or_default();
        if single_record {
            isolate_current_record(
                &mut request,
                entity,
                context,
                if qa_record {
                    RecordTargetMode::ExactEntity
                } else {
                    RecordTargetMode::ExplicitNumericField
                },
            )?;
        }
        if rephrase {
            request.input = known_question_form(episode.category, &request.input, entity, context)?;
        }
        if rephrase_field {
            request.input = known_field_question_form(
                &request.input,
                episode.family.rsplit('/').next().unwrap_or_default(),
                entity,
                context,
            )?;
        }
        requests.push(request);
    }
    let loaded = checkpoint::load(checkpoint, Device::Cpu, false)?;
    control.check("evaluate_model_loaded")?;
    if let Some(state) = &loaded.manifest.training
        && (state.corpus_hash != manifest.train.sha256
            || state.validation_hash != manifest.validation.sha256)
    {
        return Err(Error::Invalid("diagnostic corpus lineage mismatch".into()));
    }
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    replica_v3::binary::write_value_record(&mut log, &replica_v3::binary::record!({"header":true,"split":split,"checkpoint_sha256":loaded.manifest.weights_sha256,"split_sha256":split_hash,"limit":limit,"final_heldout":false,"oracle_question_ablation":rephrase || rephrase_field,"oracle_field_task_label":rephrase_field || (single_record && !qa_record),"oracle_record_selection":single_record,"eligible_episodes":episodes.len()}))?;
    let mut exact = 0;
    let mut failed = 0;
    let mut groups: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let mut evaluated = Vec::new();
    for (episode, request) in episodes.iter().take(limit).zip(&requests) {
        if control.check("evaluate_next_case").is_err() {
            break;
        }
        let row = recovery::evaluate_one(&loaded, episode, request, &mut control);
        let matched = row["exact_match"] == true;
        failed += usize::from(!row["error"].is_null());
        exact += usize::from(matched);
        let group = if episode.family.starts_with("copy/") {
            "copy".to_string()
        } else {
            format!("qa-{}", episode.category)
        };
        let count = groups.entry(group).or_default();
        count[0] += usize::from(matched);
        count[1] += 1;
        replica_v3::binary::write_value_record(&mut log, &row)?;
        log.flush()?;
        println!(
            "{split} id={} exact={matched} error={}",
            episode.id, row["error"]
        );
        evaluated.push(row);
        if control.check("evaluate_case_recorded").is_err() {
            break;
        }
    }
    let _ = control.check("evaluate_summary");
    let mut summary = replica_v3::binary::record!({"summary":true,"split":split,"exact_matches":exact,"denominator":evaluated.len(),"generation_failures":failed,"groups_correct_total":groups,"final_heldout":false,"oracle_question_ablation":rephrase || rephrase_field,"oracle_field_task_label":rephrase_field || (single_record && !qa_record),"oracle_record_selection":single_record});
    summary["diagnostic_score"] = recovery::summarize(&evaluated)?;
    recovery::add_partial_counts(&mut summary, &evaluated, limit, &control);
    replica_v3::binary::write_value_record(&mut log, &summary)?;
    log.sync_all()?;
    let _ = control.seal_terminal();
    let mut terminal = replica_v3::binary::record!({"terminal":true});
    recovery::add_partial_counts(&mut terminal, &evaluated, limit, &control);
    replica_v3::binary::write_value_record(&mut log, &terminal)?;
    log.sync_all()?;
    println!("{summary}");
    control.stop_result()
}
fn decode_generated(
    tok: &ByteBpe,
    result: Result<neural::transformer::Generated>,
) -> (
    Option<String>,
    Option<neural::transformer::Generated>,
    Option<String>,
) {
    match result {
        Ok(g) => match tok.decode(&g.tokens) {
            Ok(text) => (Some(text), Some(g), None),
            Err(error) => (None, Some(g), Some(error.to_string())),
        },
        Err(error) => (None, None, Some(error.to_string())),
    }
}
// Group size 1 preserves the original sampler exactly. Larger groups draw complete
// consecutive blocks from the eligible pool; they do not mix samples inside attention.
fn draw_indices(pool: &[usize], config: &TrainConfig, rng: &mut Rng) -> Result<Vec<usize>> {
    let group = config.sample_group_size;
    if group == 0
        || config.microbatch == 0
        || !config.microbatch.is_multiple_of(group)
        || pool.is_empty()
        || !pool.len().is_multiple_of(group)
    {
        return Err(Error::Invalid(
            "sample pool/microbatch must contain complete groups".into(),
        ));
    }
    let mut selected = Vec::with_capacity(config.microbatch);
    for _ in 0..config.microbatch / group {
        let start = (rng.next_u64() % (pool.len() / group) as u64) as usize * group;
        selected.extend_from_slice(&pool[start..start + group]);
    }
    Ok(selected)
}
pub fn sampling_exposure(start: &Path, end: &Path, corpus: &Path, limit: usize) -> Result<()> {
    let state = |path| -> Result<TrainingState> {
        checkpoint::load(path, Device::Cpu, false)?
            .manifest
            .training
            .ok_or_else(|| Error::Invalid("sampling audit requires training state".into()))
    };
    let start = state(start)?;
    let end = state(end)?;
    let (manifest, episodes, _) = data::load(corpus)?;
    if start.config != end.config
        || start.corpus_hash != end.corpus_hash
        || end.corpus_hash != manifest.train.sha256
        || start.validation_hash != end.validation_hash
        || end.validation_hash != manifest.validation.sha256
        || start.step > end.step
        || limit == 0
        || limit > episodes.len()
        || episodes.iter().any(|e| e.answer.is_empty())
    {
        return Err(Error::Invalid(
            "sampling audit requires one unchanged QA run and corpus".into(),
        ));
    }
    let copy: Vec<_> = episodes
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.family.starts_with("copy/").then_some(i))
        .collect();
    let mut counts = vec![0usize; episodes.len()];
    let all: Vec<_> = (0..episodes.len()).collect();
    let mut rng = Rng::new(start.sampler_state);
    for step in start.step..end.step {
        let pool = if step < end.config.curriculum_steps {
            &copy
        } else {
            &all
        };
        for _ in 0..end.config.accumulation {
            for index in draw_indices(pool, &end.config, &mut rng)? {
                counts[index] += 1;
            }
        }
    }
    if rng.state != end.sampler_state {
        return Err(Error::Corrupt(
            "replayed sampler does not match saved state".into(),
        ));
    }
    let histogram = |values: &[usize]| {
        let mut result = BTreeMap::new();
        for &n in values {
            *result.entry(n).or_insert(0usize) += 1;
        }
        result
    };
    replica_v3::binary::print_record(&replica_v3::binary::record!({
            "method":"DETERMINISTIC_SAMPLER_REPLAY_NOT_PER_DRAW_LOG", "sampler_state_matches":true,
            "start_step":start.step,"end_step":end.step,"corpus_sha256":manifest.train.sha256,
            "episodes":episodes.len(),"draws":counts.iter().sum::<usize>(),
            "all_exposure_histogram":histogram(&counts),"prefix_exposure_histogram":histogram(&counts[..limit]),
            "prefix":episodes.iter().zip(&counts).take(limit).map(|(e,n)|replica_v3::binary::record!({"id":e.id,"draws":n})).collect::<Vec<_>>()
        }))?;
    Ok(())
}
fn rss_kib() -> Result<u64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()?;
    if !out.status.success() {
        return Err(Error::Invalid("RSS observation failed".into()));
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .map_err(|_| Error::Invalid("RSS parse".into()))
}
pub struct Run<'a> {
    pub checkpoint: &'a Path,
    pub corpus: Option<&'a Path>,
    pub output: &'a Path,
    pub resume: bool,
    pub numeric_probe: bool,
    pub config: TrainConfig,
    pub stop_after: Option<usize>,
    pub measure_rss: bool,
    pub extend_steps: Option<usize>,
    pub extend_microbatch: Option<usize>,
    pub extend_sample_group_size: Option<usize>,
    pub extend_curriculum_steps: Option<usize>,
    pub extend_first_target_weight: Option<f64>,
    pub extend_lr: Option<f64>,
    pub extend_warmup: Option<usize>,
    pub source_id: Option<String>,
    pub replace_corpus: bool,
}
pub fn train(run: Run<'_>, cancel: std::sync::Arc<AtomicBool>) -> Result<()> {
    let mut control = recovery::RunControl::new(
        cancel,
        std::time::Duration::from_secs(900),
        16 * 1024 * 1024,
    )?;
    train_controlled(run, &mut control)
}
fn train_controlled(run: Run<'_>, control: &mut recovery::RunControl) -> Result<()> {
    train_with_policy(run, control, None)
}
fn train_with_policy(run: Run<'_>, control: &mut recovery::RunControl, fresh: Option<(&fresh::Plan, &Path)>) -> Result<()> {
    control.measure_rss = run.measure_rss;
    control.check("training_started")?;
    if run.resume && fresh.is_none() {
        recovery::reject_unbound_objective_resume(run.checkpoint)?;
    }
    if run.extend_steps.is_none()
        && (run.extend_microbatch.is_some()
            || run.extend_sample_group_size.is_some()
            || run.extend_curriculum_steps.is_some()
            || run.extend_first_target_weight.is_some()
            || run.extend_lr.is_some()
            || run.extend_warmup.is_some())
    {
        return Err(Error::Invalid(
            "batch/curriculum/objective/LR changes require explicit extension".into(),
        ));
    }
    if run.replace_corpus && (!run.resume || run.extend_steps.is_none() || run.numeric_probe) {
        return Err(Error::Invalid(
            "corpus change requires an explicit extension and real corpus".into(),
        ));
    }
    let started = Instant::now();
    let mut loaded = checkpoint::load(run.checkpoint, Device::Cpu, run.resume)?;
    if loaded.model.adapter().is_some() && fresh.is_none() {
        return Err(Error::Invalid("adapter requires its explicitly bound execution profile".into()));
    }
    let framing = fresh.map_or(neural::Framing::QuestionEvidence, |(p, _)| p.framing());
    if loaded.manifest.training.is_some() && loaded.manifest.framing()? != framing {
        return Err(Error::Invalid(
            "ACTUAL_FRAMING_BINDING_MISMATCH; optimizer_calls=0".into(),
        ));
    }
    let fresh_parent = match fresh {
        Some((p, _)) => p.parent_entry(run.checkpoint, &loaded)?,
        None => false,
    };
    control.check("training_loaded")?;
    let mut config = if run.resume {
        loaded
            .manifest
            .training
            .as_ref()
            .ok_or_else(|| Error::Invalid("resume requires optimizer state".into()))?
            .config
            .clone()
    } else {
        run.config
    };
    if fresh_parent {let p=fresh.unwrap().0;config=p.config.clone();loaded.manifest.source_id=p.source_identity().into();}
    if let Some(additional) = run.extend_steps {
        let previous = loaded
            .manifest
            .training
            .as_ref()
            .filter(|_| run.resume)
            .ok_or_else(|| Error::Invalid("extension requires a resume checkpoint".into()))?;
        let source_id = run
            .source_id
            .as_ref()
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| {
                Error::Invalid("extension requires current source manifest SHA-256".into())
            })?;
        if additional == 0 || additional > 5000 {
            return Err(Error::Invalid(
                "extension budget is 1..5000 additional steps".into(),
            ));
        }
        config.budget_start_step = previous.step;
        config.budget_start_tokens = previous.consumed_tokens;
        config.max_steps = previous
            .step
            .checked_add(additional)
            .ok_or_else(|| Error::Invalid("step budget overflow".into()))?;
        if let Some(microbatch) = run.extend_microbatch {
            config.microbatch = microbatch;
        }
        if let Some(group) = run.extend_sample_group_size {
            config.sample_group_size = group;
        }
        if let Some(weight) = run.extend_first_target_weight {
            config.first_target_weight = weight;
        }
        if let Some(lr) = run.extend_lr {
            config.lr = lr;
        }
        if let Some(warmup) = run.extend_warmup {
            config.warmup = warmup;
        }
        if let Some(steps) = run.extend_curriculum_steps {
            if steps > additional {
                return Err(Error::Invalid(
                    "copy curriculum exceeds extension budget".into(),
                ));
            }
            config.curriculum_steps = previous.step + steps;
        }
        config.max_tokens = previous
            .consumed_tokens
            .checked_add(20_000_000)
            .ok_or_else(|| Error::Invalid("token budget overflow".into()))?;
        loaded.manifest.source_id = source_id.clone();
        println!(
            "explicit_extension start_step={} start_tokens={} additional_steps={additional} additional_token_cap=20000000 microbatch={} sample_group_size={} copy_until_step={} first_target_weight={} lr={} warmup={} schedule_restart=true optimizer_reset=false source_id={source_id}",
            previous.step,
            previous.consumed_tokens,
            config.microbatch,
            config.sample_group_size,
            config.curriculum_steps,
            config.first_target_weight,
            config.lr,
            config.warmup
        );
    }
    config.validate(loaded.model.config.context)?;
    let estimated_bytes = config.microbatch
        * loaded.model.config.layers
        * loaded.model.config.heads
        * config.seq_len
        * config.seq_len
        * 4
        * 12
        + loaded.model.config.parameters() * 4 * 16;
    if estimated_bytes > 16usize * 1024 * 1024 * 1024 {
        return Err(Error::Invalid(
            "training tensor planning guard exceeds 16 GiB".into(),
        ));
    }
    println!("tensor_planning_guard_bytes={estimated_bytes} planning_guard_is_measurement=false");
    let (mut train, validation, corpus_hash, validation_hash, corpus_manifest) = if run.numeric_probe {
        let data = numeric_samples(&loaded.tokenizer)?;
        (
            data.clone(),
            data,
            loaded.tokenizer.train_hash.clone(),
            neural::hash(b"NUMERIC_OVERFIT_NOT_HELDOUT"),
            None,
        )
    } else {
        let corpus=run.corpus.ok_or_else(|| Error::Invalid("explicit --corpus required".into()))?;
        let (manifest, train, validation) = match fresh {
            Some((p,_))=>p.training_corpus(corpus)?,None=>data::load(corpus)?,
        };
        let resumed_corpus = run.resume
            && loaded
                .manifest
                .training
                .as_ref()
                .is_some_and(|s| s.corpus_hash == manifest.train.sha256);
        if manifest.train.sha256 != loaded.tokenizer.train_hash
            && !resumed_corpus
            && !run.replace_corpus
            && !fresh.is_some_and(|(p,_)|p.reuses_tokenizer_mapping(&loaded.tokenizer))
        {
            return Err(Error::Corrupt("tokenizer/corpus mismatch".into()));
        }
        (
            samples_with_framing(&train, &loaded.tokenizer, config.seq_len, framing)?,
            samples_with_framing(
                &validation,
                &loaded.tokenizer,
                loaded.model.config.context,
                framing,
            )?,
            manifest.train.sha256.clone(),
            manifest.validation.sha256.clone(),
            Some(manifest),
        )
    };
    if let Some((p, root)) = fresh {
        train.extend(p.additional_samples(root, &loaded.tokenizer)?);
        p.apply_training_values(root, &loaded.tokenizer, &mut train)?;
    }
    let grounding = match fresh {
        Some((p,root))=>p.grounding_labels(root,&loaded.tokenizer,&train)?,
        None=>None,
    };
    if train.iter().any(|s| s.tokens.len() > config.seq_len + 1) {
        return Err(Error::Invalid("training sample context".into()));
    }
    let copy_indices: Vec<_> = train
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.curriculum.then_some(i))
        .collect();
    let all_indices: Vec<_> = (0..train.len()).collect();
    let starting_step = if run.resume {
        loaded
            .manifest
            .training
            .as_ref()
            .expect("checked resume")
            .step
    } else {
        0
    };
    if starting_step < config.curriculum_steps && copy_indices.is_empty() {
        return Err(Error::Invalid(
            "curriculum requires explicit copy training episodes".into(),
        ));
    }
    if !train.len().is_multiple_of(config.sample_group_size)
        || (starting_step < config.curriculum_steps
            && !copy_indices.len().is_multiple_of(config.sample_group_size))
    {
        return Err(Error::Invalid(
            "training sample pool has an incomplete group".into(),
        ));
    }
    let mut state = if run.resume {
        let state = loaded.manifest.training.clone().expect("checked");
        if state.contrast16 {
            return Err(Error::Invalid(
                "contrast16 requires its fixed full-pass diagnostic trainer".into(),
            ));
        }
        state
    } else {
        TrainingState {
            resume_binding: None,
            contrast16: false,
            parent_checkpoint_hash: Some(loaded.manifest.weights_sha256.clone()),
            config: config.clone(),
            step: 0,
            consumed_tokens: 0,
            target_tokens: 0,
            sampler_state: config.seed,
            corpus_hash: corpus_hash.clone(),
            validation_hash: validation_hash.clone(),
            previous_corpora: fresh.map_or_else(Vec::new, |(p, _)| {
                p.new_state_tokenizer_provenance(&corpus_hash, &loaded.tokenizer)
            }),
            initial_weight_hash: loaded.manifest.initial_weight_hash.clone(),
            train_loss: None,
            validation_loss: None,
        }
    };
    state.config = config.clone();
    if run.replace_corpus || (fresh_parent && fresh.is_some_and(|(p,_)|fresh::is_binding_expansion(p))) {
        if state.corpus_hash != corpus_hash && !state.previous_corpora.contains(&state.corpus_hash)
        {
            state.previous_corpora.push(state.corpus_hash.clone());
        }
        if state.previous_corpora.len() > 32 {
            return Err(Error::Invalid("training corpus lineage limit".into()));
        }
        println!(
            "explicit_corpus_change previous={} current={} validation={} frozen_tokenizer_training_hash={}",
            state.corpus_hash, corpus_hash, validation_hash, loaded.tokenizer.train_hash
        );
        state.corpus_hash = corpus_hash.clone();
        state.validation_hash = validation_hash.clone();
        state.validation_loss = None;
        state.train_loss = None;
    }
    if state.corpus_hash != corpus_hash || state.validation_hash != validation_hash {
        return Err(Error::Corrupt("resume corpus/split mismatch".into()));
    }
    // New runs and explicit extensions bind the exact executed generic schedule/loss.
    // Ordinary resume was checked before any tensor work and cannot change this policy.
    let binding = match fresh {
        Some((plan, _)) => {
            if config != plan.config { return Err(Error::Invalid("fresh config mismatch".into())); }
            let binding = plan.binding(&state, &loaded.tokenizer)?;
            if run.resume && !fresh_parent && (state.resume_binding.as_ref() != Some(&binding) || state.sampler_state != state.step as u64) {
                return Err(Error::Invalid("fresh sampler/objective/policy mismatch; optimizer_calls=0".into()));
            }
            if !run.resume { state.sampler_state = 0; }
            binding
        }
        None => checkpoint::ResumeBinding::default_for(&state, &loaded.tokenizer),
    };
    state.resume_binding = Some(binding);
    println!(
        "available_framed_train_tokens={} available_framed_validation_tokens={} frozen_tokenizer={} previous_corpora={:?}",
        train.iter().map(|s| s.tokens.len()).sum::<usize>(),
        validation.iter().map(|s| s.tokens.len()).sum::<usize>(),
        loaded.tokenizer.id(),
        state.previous_corpora
    );
    if loaded.manifest.optimizer_protocol.is_some() {
        return Err(Error::Invalid("bound research optimizer requires its exact native execution policy".into()));
    }
    let mut adam = if run.resume {
        Adam {
            moments: loaded.optimizer,
        }
    } else {
        Adam::new(&loaded.model.vars)?
    };
    if !run.resume && loaded.manifest.training.is_some() {
        return Err(Error::Invalid(
            "trained checkpoint requires --resume".into(),
        ));
    }
    control.check("training_prepared")?;
    std::fs::create_dir(run.output)?;
    if let Some(mut manifest) = corpus_manifest {
        manifest.train.tokens = Some(train.iter().map(|s| s.tokens.len()).sum());
        manifest.validation.tokens = Some(validation.iter().map(|s| s.tokens.len()).sum());
        println!(
            "CORPUS_REPORT {}",
            replica_v3::binary::record!({
                "corpus":manifest,"source_directory":run.corpus,"tokenizer_sha256":loaded.tokenizer.id(),
                "tokenizer_training_hash":loaded.tokenizer.train_hash,"previous_corpora":state.previous_corpora,
                "tokenizer_mapping_only_provenance":fresh.is_some_and(|(p,_)|p.reuses_tokenizer_mapping(&loaded.tokenizer)),
                "token_count_kind":"available framed samples before sampling/repetition; LM overlap context included"
            })
        );
    }
    let mut peak = control.last_rss_kib;
    let mut rng = Rng::new(state.sampler_state);
    let mut reason = "BUDGET_REACHED";
    let mut last_validated = None;
    let mut fresh_evaluated = None;
    let mut executed_input_tokens = 0u64;
    let mut executed_optimizer_calls = 0u64;
    let mut executed_target_tokens=0u64;
    let mut executed_padding_tokens=0u64;
    let target_allowance=match fresh {Some((p,root))=>p.remaining_targets(root)?,None=>None};
    let input_allowance=match fresh {Some((p,_))=>p.remaining_input()?,None=>None};
    let mut token_budget_reached = false;
    let mut fresh_stop = None;
    let mut signal_observation: Option<fresh::SignalObservation> = None;
    let mut diagnostic_counts = (0usize,0usize);
    let mut fresh_trace = if fresh.is_some() {
        Some(std::fs::OpenOptions::new().write(true).create_new(true).open(run.output.join("updates.r3rows"))?)
    } else {None};
    let mut outcome = (|| -> Result<()> {
        let initial_loss = if fresh.is_some() { state.validation_loss.unwrap_or(0.) } else { validation_loss(&loaded.model, &validation, control)? };
        last_validated = if fresh.is_some() {None} else {Some(state.step)};
        println!(
            "backend={} dtype=F32 profile={} parameters={} train_samples={} validation_samples={} initial_validation_loss={} numeric_overfit_only={} loaded_weight_hash={} random_initial_weight_hash={} peak_sampled_rss_KiB={peak:?}",
            neural::cpu_backend(),
            loaded.model.config.profile,
            loaded.model.config.parameters(),
            train.len(),
            validation.len(),
            if fresh.is_some() {"NOT_RUN_FIXED_PROBE_FOLLOWS".into()} else {format!("{initial_loss:.8}")},
            run.numeric_probe,
            loaded.model.weight_hash()?,
            loaded.manifest.initial_weight_hash
        );
        let mut best = state.validation_loss.unwrap_or(initial_loss);
        if fresh.is_none() {state.validation_loss = Some(initial_loss);}
        let mut initial = loaded.manifest.clone();
        initial.training = Some(state.clone());
        initial.status = "TRAINING".into();
        control.check("before_training_start_checkpoint")?;
        checkpoint::save(
            &run.output.join("start"),
            &loaded.model,
            &loaded.tokenizer,
            initial,
            &adam.moments,
        )?;
        control.check("training_start_checkpoint_saved")?;
        if let Some((plan, root)) = fresh {
            fresh_stop = fresh::evaluate_boundary(plan, root, &run.output.join("start"), state.step, control)?;
            if plan.evaluation_due(state.step) {
                fresh_evaluated = Some(state.step);
            }
        }
        while state.step < config.max_steps {
            if fresh_stop.is_some() { reason = "TRAINING"; break; }
            control.check("training_step")?;
            if run.stop_after.is_some_and(|n| state.step >= n) {
                reason = "TRAINING";
                break;
            }
            if let Some((p,root))=fresh && fresh::SignalObservation::due(p,state.step+1) {
                    let path=run.output.join(format!("signal-before-{:04}",state.step+1));
                    loaded.model.refresh_identity()?;
                    let mut manifest=loaded.manifest.clone();manifest.training=Some(state.clone());
                    checkpoint::save(&path,&loaded.model,&loaded.tokenizer,manifest,&adam.moments)?;
                    signal_observation=fresh::SignalObservation::start(p,root,state.step+1,&loaded.tokenizer,&path)?;
                    if let Some(obs)=&mut signal_observation {obs.before(&loaded.model,control)?;}
            }
            let sampler_before = rng.state;
            let mut gradients: BTreeMap<String, Tensor> = BTreeMap::new();
            let mut loss_sum = 0f64;
            let mut objective_sum = 0f64;
            let mut objective_denominator = 0usize;
            let mut examples = 0usize;
            let mut answer_ce_sum = 0f64;
            let mut targets = 0usize;
            let mut step_tokens = 0;
            let mut aborted = false;
            let mut task_stats = Vec::new();
            let balanced = fresh.map(|(plan, _)| plan.training_draw(state.step));
            for micro in 0..config.accumulation {
                control.check("before_training_microbatch")?;
                let pool = if state.step < config.curriculum_steps {
                    &copy_indices
                } else {
                    &all_indices
                };
                let indices = match &balanced {
                    Some(indices) => indices[micro*config.microbatch..(micro+1)*config.microbatch].to_vec(),
                    None => draw_indices(pool, &config, &mut rng)?,
                };
                let b = batch(&train, &indices, &loaded.model.device)?;
                let batch_targets=indices.iter().map(|&i|(train[i].tokens.len()-train[i].response_start) as u64).sum::<u64>();
                if state
                    .consumed_tokens
                    .checked_add(b.tokens as u64)
                    .is_none_or(|n| n > config.max_tokens)
                    || target_allowance.is_some_and(|n|executed_target_tokens+batch_targets>n)
                    || input_allowance.is_some_and(|n|executed_input_tokens+b.tokens as u64>n)
                {
                    aborted = true;
                    break;
                }
                let labels=grounding.as_ref().map(|all|indices.iter().map(|&i|all[i].clone()).collect::<Vec<_>>());
                let mut ground_loss=None;
                let logits = if let Some(labels)=labels.as_ref().filter(|ls|ls.iter().any(Option::is_some)) {
                    let last=loaded.model.config.layers-1;
                    loaded.model.forward_observed(&b.input,Some(&b.valid),&mut |layer,q,k,mask| {
                        control.check("grounding_layer")?;
                        if layer==last {ground_loss=Some(record_grounding_loss(q,k,mask,labels)?);}
                        Ok(())
                    })?
                } else { loaded.model.forward(&b.input, Some(&b.valid))? };
                let answer = state.resume_binding.as_ref().is_some_and(|b| b.family == checkpoint::ANSWER_MEAN_FAMILY);
                let (loss, mut objective, n, denominator) = response_objective(
                    &logits,
                    &b,
                    config.first_target_weight,
                    answer,
                )?;
                if fresh.is_some_and(|(p,_)|p.is_answer_mean_study()) {
                    answer_ce_sum += f64::from(answer_mean_loss(&logits,&b)?.to_scalar::<f32>()?) * indices.len() as f64;
                }
                examples += indices.len();
                if let Some((plan,_)) = fresh
                    && let Some((sidewise,pairs)) = plan.contrast_pairs(&indices)? {
                    objective = (objective + pair_contrast_loss(&logits,&train,&indices,&pairs,sidewise)?)?;
                }
                if labels.as_ref().is_some_and(|ls|ls.iter().any(Option::is_some)) {
                    objective=(objective+ground_loss.ok_or_else(||Error::Model("missing last-layer grounding loss".into()))?)?;
                }
                if fresh.is_some() {
                    let predictions=logits.argmax(2)?.to_vec2::<u32>()?;
                    let labels=b.target.to_vec2::<u32>()?;let masks=b.mask.to_vec2::<f32>()?;
                    for (row,&index) in indices.iter().enumerate() {
                        let (ce,count)=masked_loss(&logits.narrow(0,row,1)?,&b.target.narrow(0,row,1)?,&b.mask.narrow(0,row,1)?)?;
                        let correct=predictions[row].iter().zip(&labels[row]).zip(&masks[row]).filter(|((a,b),m)|a==b&&**m>0.).count();
                        let bucket=fresh.map_or(micro*config.microbatch+row,|(plan,_)|plan.sample_bucket(index));
                        let token_observation=if fresh.is_some_and(|(p,_)|p.observes_target_tokens(state.step)) {
                            let lp=candle_nn::ops::log_softmax(&logits.narrow(0,row,1)?.squeeze(0)?,1)?.to_vec2::<f32>()?;
                            Some(masks[row].iter().enumerate().filter(|(_,m)|**m>0.).map(|(i,_)|replica_v3::binary::record!({"target":labels[row][i],"nll":-lp[i][labels[row][i] as usize],"argmax":predictions[row][i]})).collect::<Vec<_>>())
                        } else {None};
                        task_stats.push(replica_v3::binary::record!({"bucket":bucket,"index":index,"input":train[index].tokens.len()-1,"target":count,"ce":ce.to_scalar::<f32>()?,"correct_tokens":correct,"padding":b.input.dim(1)?-(train[index].tokens.len()-1),"target_token_observation":token_observation}));
                    }
                }
                let value = f64::from(loss.to_scalar::<f32>()?);
                let objective_value = f64::from(objective.to_scalar::<f32>()?);
                if !value.is_finite() || !objective_value.is_finite() {
                    return Err(Error::Model(
                        "nonfinite training loss; previous checkpoint retained".into(),
                    ));
                }
                let grads = objective.backward()?;
                // Work already executed counts against the budget even if this
                // accumulation is cancelled before the atomic optimizer update.
                state.consumed_tokens += b.tokens as u64;
                executed_input_tokens += b.tokens as u64;
                executed_target_tokens += n as u64;
                executed_padding_tokens += (b.input.elem_count()-b.tokens) as u64;
                step_tokens += b.tokens;
                loss_sum += value * n as f64;
                objective_sum += objective_value * denominator as f64;
                objective_denominator += denominator;
                targets += n;
                for (name, var) in &loaded.model.vars {
                    let g = grads
                        .get(var)
                        .ok_or_else(|| Error::Model(format!("missing gradient {name}")))?;
                    let weighted = (g * denominator as f64)?.detach();
                    let accumulated = match gradients.remove(name) {
                        Some(old) => (old + weighted)?,
                        None => weighted,
                    };
                    gradients.insert(name.clone(), accumulated.detach());
                }
                control.check("training_microbatch_returned")?;
            }
            if aborted {
                rng.state = sampler_before;
                token_budget_reached = true;
                break;
            }
            for gradient in gradients.values_mut() {
                *gradient = (&*gradient / objective_denominator as f64)?;
            }
            control.check("before_training_optimizer")?;
            let actual_lr=fresh.map_or_else(||config.learning_rate(state.step+1),|(p,_)|p.learning_rate(state.step+1));
            executed_optimizer_calls += 1;
            let adam_step = replica_v3::neural::artifact::adapter_base(&loaded.model)
                .map_or(Ok(state.step+1),|base|state.step.checked_sub(base.step).map(|n|n+1)
                    .ok_or_else(||Error::Invalid("adapter optimizer clock before base".into())))?;
            let mut adapter_norms=[[0f64;4];2];
            let (grad_norm, delta) = if let Some(obs)=&mut signal_observation {
                adam.step_constant_observed(&loaded.model.vars,&gradients,&config,adam_step,actual_lr,
                    |name,_,old,next|obs.delta(name,old,next))?
            } else if loaded.model.adapter().is_some() {
                adam.step_constant_observed(&loaded.model.vars,&gradients,&config,adam_step,actual_lr,|name,g,old,next|{
                    let group=if name.ends_with(".a"){0}else{1};
                    for (i,t) in [g.clone(),old.clone(),next.clone(),(next-old)?].iter().enumerate(){
                        let norm=t.sqr()?.sum_all()?.to_scalar::<f32>()?as f64;
                        if !norm.is_finite(){return Err(Error::Model("nonfinite adapter observation".into()));}adapter_norms[group][i]+=norm;
                    }Ok(())
                })?
            } else {adam.step_constant(&loaded.model.vars, &gradients, &config, adam_step,actual_lr)?};
            state.step += 1;
            state.target_tokens += targets as u64;
            state.sampler_state = if fresh.is_some() {state.step as u64} else {rng.state};
            state.train_loss = Some(loss_sum / targets as f64);
            state.validation_loss = None;
            if let Some(trace)=&mut fresh_trace {
                use std::io::Write;
                let mut row=replica_v3::binary::record!({"step":state.step,"sampler":state.sampler_state,"epoch":fresh.map_or(state.step/1024,|(p,_)|p.epoch(state.step)),"paired_cursor":fresh.and_then(|(p,_)|p.pair_position(state.step-1)),"draw":fresh.map(|(p,_)|p.draw(state.step-1)),"sample_indices":balanced,"tasks":task_stats,"input":step_tokens,"target":targets,"ce":state.train_loss,"objective":objective_sum/objective_denominator as f64,"lr":actual_lr,"lr_bits":actual_lr.to_bits(),"grad_norm":grad_norm,"clip":(config.clip/(grad_norm+1e-12)).min(1.),"delta_norm":delta});
                if let Some(base)=replica_v3::neural::artifact::adapter_base(&loaded.model) {
                    row["adapter"]=replica_v3::binary::record!({"base_step":base.step,"base_content":base.content,"base_physical":base.physical,
                        "updates":adam_step,"parameters":loaded.model.vars.values().map(|v|v.elem_count()).sum::<usize>(),
                        "adam_bytes":adam.moments.values().map(|v|v.elem_count()*4).sum::<usize>(),
                        "A_B_gradient_before_after_delta_l2":adapter_norms.map(|g|g.map(f64::sqrt)),"finite":true});
                }
                if fresh.is_some_and(|(p,_)|p.is_answer_mean_study()) {
                    row["token_ce"]=replica_v3::binary::record!(loss_sum/targets as f64);
                    row["answer_mean_ce"]=replica_v3::binary::record!(answer_ce_sum/examples as f64);
                    row["effective_training_objective"]=replica_v3::binary::record!(objective_sum/objective_denominator as f64);
                    row["objective_denominator"]=replica_v3::binary::record!(objective_denominator);
                    row["target_tokens"]=replica_v3::binary::record!(targets);
                    row["examples"]=replica_v3::binary::record!(examples);
                }
                replica_v3::binary::write_value_record(trace,&row)?;
                trace.flush()?;
                if state.step==config.budget_start_step+1||state.step.is_multiple_of(32){trace.sync_all()?;}
            }
            if let Some(obs)=&mut signal_observation {
                control.check("signal_optimizer_committed")?;
                obs.after(&loaded.model,control)?;
                if obs.finish(None,state.step,None)?!="COMPLETED" {return Err(Error::Invalid("incomplete signal observation".into()));}
                let (f,b)=obs.counts();diagnostic_counts.0+=f;diagnostic_counts.1+=b;
                signal_observation=None;
            }
            #[cfg(feature = "test-support")]
            if fresh.is_some_and(|(p, _)| p.is_tiny())
                && (state.step == config.max_steps
                    || (fresh.is_some_and(|(p, _)| p.framing() == neural::Framing::EvidenceQuestion)
                        && run.stop_after == Some(state.step)))
            {
                control.fixture_boundary = std::env::var("R3_FRESH_TRAIN_STOP").ok();
            }
            control.check("training_optimizer_returned")?;
            let rss = control.last_rss_kib;
            peak = peak.max(rss);
            println!(
                "step={} loss={:.8} objective={:.8} first_target_weight={} grad_norm={grad_norm:.8} weight_delta_l2={delta:.8} lr={:.8} consumed_tokens={} target_tokens={} step_tokens={step_tokens} elapsed_s={:.3} rss_KiB={rss:?} peak_sampled_rss_KiB={peak:?}",
                state.step,
                state.train_loss.expect("observed"),
                objective_sum / objective_denominator as f64,
                config.first_target_weight,
                actual_lr,
                state.consumed_tokens,
                state.target_tokens,
                started.elapsed().as_secs_f64()
            );
            if state.step == 1 {
                println!(
                    "after_first_update_weight_hash={}",
                    loaded.model.weight_hash()?
                );
            }
            if state.step.is_multiple_of(config.validate_every) || state.step == config.max_steps || fresh.is_some_and(|(p,_)|p.evaluation_due(state.step)) {
                let value = if fresh.is_some() { state.train_loss.unwrap() } else { validation_loss(&loaded.model, &validation, control)? };
                if fresh.is_none() {
                    last_validated = Some(state.step);
                    state.validation_loss = Some(value);
                    println!(
                    "validation step={} loss={value:.8} samples={} selection=VALIDATION_ONLY",
                    state.step,
                    validation.len()
                    );
                }
                let improved = fresh.is_none() && value < best;
                best = best.min(value);
                loaded.model.refresh_identity()?;
                let mut manifest = loaded.manifest.clone();
                manifest.training = Some(state.clone());
                manifest.status = "TRAINING".into();
                let path = run.output.join(format!("step-{:06}", state.step));
                control.check("before_training_checkpoint")?;
                let m = checkpoint::save(
                    &path,
                    &loaded.model,
                    &loaded.tokenizer,
                    manifest,
                    &adam.moments,
                )?;
                control.check("training_checkpoint_saved")?;
                if let Some((plan, root)) = fresh {
                    fresh_stop = fresh::evaluate_boundary(plan, root, &path, state.step, control)?;
                    if plan.evaluation_due(state.step) {
                        fresh_evaluated = Some(state.step);
                    }
                }
                println!(
                    "checkpoint={} sha256={} best_validation={improved}",
                    path.display(),
                    m.weights_sha256
                );
                if improved {
                    let record = replica_v3::binary::record!({"checkpoint":path.file_name(),"validation_loss":value,"step":state.step});
                    neural::write_new(
                        &run.output.join(format!("best-{:06}.r3b", state.step)),
                        &replica_v3::binary::to_vec(&record)?,
                    )?;
                }
            }
        }
        if fresh.is_none() && !token_budget_reached && last_validated != Some(state.step) {
            state.validation_loss = Some(validation_loss(&loaded.model, &validation, control)?);
            last_validated = Some(state.step);
        }
        Ok(())
    })();
    if let Some(obs)=&signal_observation {
        let (f,b)=obs.counts();diagnostic_counts.0+=f;diagnostic_counts.1+=b;
    }
    if let Err(error) = &outcome {
        control.classify_error(error);
    }
    let trace_saved=fresh_trace.as_ref().map_or(Ok(()),|f|f.sync_all());
    if let Err(error)=&trace_saved{control.classify_error(&Error::Io(std::io::Error::new(error.kind(),error.to_string())));}
    let _ = control.check("before_training_preservation");
    let work_elapsed = started.elapsed().as_secs_f64();
    let cleanup = Instant::now();
    let saved_reason = control.reason().unwrap_or(reason);
    let mut manifest = loaded.manifest;
    manifest.training = Some(state.clone());
    manifest.status = match saved_reason {
        "CANCELLED" | "RESOURCE_LIMIT" => saved_reason,
        "TIME_BUDGET" => "BUDGET_EXHAUSTED",
        "INTEGRITY_FAIL" | "RESOURCE_OBSERVATION_FAILED" => "TRAINING",
        _ => reason,
    }
    .into();
    let path = run.output.join("final");
    let saved = loaded.model.refresh_identity().and_then(|_| {
        checkpoint::save(
            &path,
            &loaded.model,
            &loaded.tokenizer,
            manifest,
            &adam.moments,
        )
    });
    if let Err(error) = &saved {
        control.classify_error(error);
    }
    let _ = control.check("training_checkpoint_preserved");
    let _ = control.seal_terminal();
    reason = control.reason().unwrap_or(reason);
    let mut receipt = control.receipt();
    receipt["reason"] = replica_v3::binary::record!(reason);
    receipt["checkpoint_saved"] = replica_v3::binary::record!(saved.is_ok());
    receipt["checkpoint_save_status_reason"] = replica_v3::binary::record!(saved_reason);
    receipt["save_error"] = replica_v3::binary::record!(saved.as_ref().err().map(ToString::to_string));
    receipt["work_error"] = replica_v3::binary::record!(outcome.as_ref().err().map(ToString::to_string));
    receipt["trace_error"]=replica_v3::binary::record!(trace_saved.as_ref().err().map(ToString::to_string));
    receipt["work_elapsed_seconds"] = replica_v3::binary::record!(work_elapsed);
    receipt["cleanup_elapsed_seconds"] = replica_v3::binary::record!(cleanup.elapsed().as_secs_f64());
    receipt["final_evaluation_complete"] = replica_v3::binary::record!(if fresh.is_some() {
        fresh_evaluated == Some(state.step)
    } else {
        last_validated == Some(state.step)
    });
    receipt["executed_input_tokens_including_uncommitted"] =
        replica_v3::binary::record!(executed_input_tokens);
    receipt["optimizer_calls"] = replica_v3::binary::record!(executed_optimizer_calls);
    receipt["diagnostic_microbatch_forwards"]=replica_v3::binary::record!(diagnostic_counts.0);
    receipt["diagnostic_sample_forwards"]=replica_v3::binary::record!(diagnostic_counts.0*8);
    receipt["diagnostic_backwards"]=replica_v3::binary::record!(diagnostic_counts.1);
    receipt["executed_target_tokens_including_uncommitted"]=replica_v3::binary::record!(executed_target_tokens);
    receipt["executed_padding_tokens"]=replica_v3::binary::record!(executed_padding_tokens);
    receipt["token_budget_reached"]=replica_v3::binary::record!(token_budget_reached);
    receipt["candidate_eligible"] = replica_v3::binary::record!(false);
    if fresh.is_some() {
        receipt["fresh_stop"] = replica_v3::binary::record!(fresh_stop);
        receipt["validation_loss_kind"] = replica_v3::binary::record!("see fixed-probe evaluation; checkpoint loss is training CE");
    }
    if let Some(obs)=signal_observation.take() {
        let pure_time=matches!(&outcome,Err(Error::Model(message)) if message=="TIME_BUDGET");
        let error=outcome.as_ref().err().map(ToString::to_string).or_else(||Some("incomplete observation".into()));
        match obs.finish(error,state.step,Some((&path,&receipt,pure_time))) {
            Ok("NOT_INVOKED_TIME_PAUSE")=>receipt["signal_observation_state"]=replica_v3::binary::record!("NOT_INVOKED_TIME_PAUSE"),
            Ok(state)=>{receipt["signal_observation_state"]=replica_v3::binary::record!(state);outcome=Err(Error::Invalid(state.into()));},
            Err(e)=>{receipt["signal_observation_state"]=replica_v3::binary::record!("BLOCKED_OBSERVATION_PUBLICATION");outcome=Err(e);},
        }
    }
    neural::write_new(&run.output.join("train-control.r3b"), &replica_v3::binary::to_vec(&receipt)?)?;
    println!("TRAIN_CONTROL {receipt}");
    println!(
        "TRAIN_END reason={reason} steps={} consumed_tokens={} target_tokens={} train_loss={:?} validation_loss={:?} elapsed_s={:.3} peak_sampled_rss_KiB={peak:?} checkpoint={} sha256={} exact_resume=optimizer_boundary task_quality=NOT_EVALUATED",
        state.step,
        state.consumed_tokens,
        state.target_tokens,
        state.train_loss,
        state.validation_loss,
        started.elapsed().as_secs_f64(),
        path.display(),
        saved
            .as_ref()
            .map_or("NOT_SAVED", |m| m.weights_sha256.as_str())
    );
    control.stop_result().and(outcome).and(saved.map(|_| ()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn answer_mean_scalar_mask_gradient_and_accumulation() -> Result<()> {
        let device=Device::Cpu;
        let make=|lengths:&[usize]| lengths.iter().enumerate().map(|(i,&n)| {
            let mut tokens=vec![BOS,8+i as u32,20];
            tokens.extend((0..n-1).map(|j|8+(j%12) as u32)); tokens.push(EOS);
            Sample{tokens,response_start:3,curriculum:false}
        }).collect::<Vec<_>>();
        let mut fd=0;
        for lengths in [vec![2,2,2,2,17,17,17,17],vec![1,3,5,7,2,4,6,8],vec![3;8]] {
            let ss=make(&lengths);let b=batch(&ss,&(0..8).collect::<Vec<_>>(),&device)?;
            let t=b.input.dim(1)?;let v=32;
            let values=(0..8*t*v).map(|i|(i%29) as f32*0.07-1.).collect::<Vec<_>>();
            let masks=b.mask.flatten_all()?.to_vec1::<f32>()?;
            let targets=b.target.flatten_all()?.to_vec1::<u32>()?;
            for answer in [false,true] {
                let logits=Var::from_vec(values.clone(),(8,t,v),&device)?;
                let (_,loss,n,den)=response_objective(&logits,&b,1.,answer)?;
                assert_eq!(n,lengths.iter().sum::<usize>());assert_eq!(den,if answer {8}else{n});
                let reference=|values:&[f32]| {
                    let mut total=0.;
                    for row in 0..8*t {
                        let z=&values[row*v..(row+1)*v];let max=z.iter().copied().fold(f32::NEG_INFINITY,f32::max) as f64;
                        let sum=z.iter().map(|x|(f64::from(*x)-max).exp()).sum::<f64>();
                        let coefficient=f64::from(masks[row])/if answer {8.*lengths[row/t] as f64}else{n as f64};
                        total+=(max+sum.ln()-f64::from(z[targets[row] as usize]))*coefficient;
                    } total
                };
                assert!((f64::from(loss.to_scalar::<f32>()?)-reference(&values)).abs()<2e-6);
                let grads=loss.backward()?;let actual=grads.get(&logits).unwrap().flatten_all()?.to_vec1::<f32>()?;
                for row in 0..8*t {
                    let z=&values[row*v..(row+1)*v];let max=z.iter().copied().fold(f32::NEG_INFINITY,f32::max) as f64;
                    let sum=z.iter().map(|x|(f64::from(*x)-max).exp()).sum::<f64>();
                    let coefficient=f64::from(masks[row])/if answer {8.*lengths[row/t] as f64}else{n as f64};
                    for k in 0..v {
                        let expected=(((f64::from(z[k])-max).exp()/sum)-f64::from(k==targets[row] as usize))*coefficient;
                        assert!((f64::from(actual[row*v+k])-expected).abs()<2e-6);
                        if masks[row]==0. {assert_eq!(actual[row*v+k],0.);}
                    }
                }
                for row in 0..8 {let eos=row*t+ss[row].tokens.len()-2;assert_eq!(targets[eos],EOS);assert_eq!(masks[eos],1.);assert!(actual[eos*v+EOS as usize]<0.);}
                // Eight central differences; the analytic gradient covers every layout/coordinate.
                for i in [0,2*v+8,(t+2)*v+9,(8*t-1)*v+EOS as usize].into_iter().filter(|_|lengths[0]==2 && lengths[7]==17) {
                    let mut lo=values.clone();let mut hi=values.clone();lo[i]-=0.001;hi[i]+=0.001;
                    let expected=(reference(&hi)-reference(&lo))/f64::from(hi[i]-lo[i]);fd+=1;
                    assert!((f64::from(actual[i])-expected).abs()<2e-6);
                }
                for sizes in [vec![8],vec![4,4],vec![3,5],vec![2,6]] {
                    let mut start=0;let mut total=0.;let mut norm=0;
                    for size in sizes {
                        let ids=(start..start+size).collect::<Vec<_>>();let mb=batch(&ss,&ids,&device)?;
                        let mt=mb.input.dim(1)?;
                        let slice=logits.narrow(0,start,size)?.narrow(1,0,mt)?;
                        let (_,l,_,d)=response_objective(&slice,&mb,1.,answer)?;
                        total+=f64::from(l.to_scalar::<f32>()?)*d as f64;norm+=d;
                        let mg=l.backward()?;let g=mg.get(&logits).unwrap().flatten_all()?.to_vec1::<f32>()?;
                        for r in start..start+size {for k in 0..t*v {
                            assert!((g[r*t*v+k]*d as f32/den as f32-actual[r*t*v+k]).abs()<2e-6);
                        }} start+=size;
                    }
                    assert_eq!(norm,den);assert!((total/norm as f64-reference(&values)).abs()<2e-6);
                }
                let reversed=(0..8).rev().collect::<Vec<_>>();let rb=batch(&ss,&reversed,&device)?;
                let rv=reversed.iter().flat_map(|&i|values[i*t*v..(i+1)*t*v].iter().copied()).collect::<Vec<_>>();
                let (_,rl,_,_)=response_objective(&Tensor::from_vec(rv,(8,t,v),&device)?,&rb,1.,answer)?;
                assert!((rl.to_scalar::<f32>()?-loss.to_scalar::<f32>()?).abs()<2e-6);
                let mut invalid=batch(&ss,&[0],&device)?;
                invalid.mask=Tensor::zeros(invalid.mask.shape(),DType::F32,&device)?;
                assert!(response_objective(&logits.narrow(0,0,1)?.narrow(1,0,invalid.input.dim(1)?)?,&invalid,1.,answer).is_err());
                let mut mixed=batch(&ss,&[0,1],&device)?;
                let mut mm=mixed.mask.to_vec2::<f32>()?;mm[0].fill(0.);
                mixed.mask=Tensor::from_vec(mm.concat(),mixed.mask.shape(),&device)?;
                assert!(response_objective(&logits.narrow(0,0,2)?.narrow(1,0,mixed.input.dim(1)?)?,&mixed,1.,answer).is_err());
                let nan=Tensor::full(f32::NAN,(8,t,v),&device)?;
                assert!(response_objective(&nan,&b,1.,answer).is_err());
            }
            if lengths.iter().all(|n|*n==3) {
                let logits=Tensor::from_vec(values,(8,t,v),&device)?;
                let (ce,l,_,_)=response_objective(&logits,&b,1.,true)?;
                assert!((ce.to_scalar::<f32>()?-l.to_scalar::<f32>()?).abs()<2e-6);
            }
        }
        println!("ANSWER_SCALAR finite_difference_coordinates={fd} optimizer=0 generation=0 teacher=0 native_forward=0 tolerance=2e-6");
        Ok(())
    }
    #[test]
    fn answer_mean_random_native_gradient_microbatch() -> Result<()> {
        let ss=(0..8).map(|i| {
            let mut tokens=vec![BOS,8+i as u32,20];let n=if i<4 {2}else{17};
            tokens.extend((0..n-1).map(|j|8+j as u32));tokens.push(EOS);
            Sample{tokens,response_start:3,curriculum:false}
        }).collect::<Vec<_>>();
        let cfg=TrainConfig{warmup:0,clip:1.,..Default::default()};
        let mut reference:Option<(BTreeMap<String,Vec<f32>>,BTreeMap<String,Vec<f32>>)>=None;
        let mut forwards=0;
        for sizes in [vec![8],vec![4,4],vec![3,5],vec![2,6]] {
            let model=Transformer::init(neural::transformer::Config::tiny(264),93,Device::Cpu)?;
            let mut gradients=BTreeMap::new();let mut count=0;
            for size in sizes {
                let ids=(count..count+size).collect::<Vec<_>>();let b=batch(&ss,&ids,&Device::Cpu)?;
                let logits=model.forward(&b.input,Some(&b.valid))?;forwards+=1;
                let (_,loss,_,den)=response_objective(&logits,&b,1.,true)?;
                let gs=loss.backward()?;
                for (name,var) in &model.vars {
                    let g=(gs.get(var).unwrap()*den as f64)?.detach();
                    let total=match gradients.remove(name) {Some(old)=>(old+g)?,None=>g};
                    gradients.insert(name.clone(),total);
                }count+=size;
            }
            for g in gradients.values_mut(){*g=(&*g/count as f64)?;}
            let flat=gradients.iter().map(|(n,t)|Ok((n.clone(),t.flatten_all()?.to_vec1::<f32>()?))).collect::<Result<BTreeMap<_,_>>>()?;
            let mut adam=Adam::new(&model.vars)?;
            adam.step_constant(&model.vars,&gradients,&cfg,1,3e-4)?;
            let weights=model.vars.iter().map(|(n,t)|Ok((n.clone(),t.flatten_all()?.to_vec1::<f32>()?))).collect::<Result<BTreeMap<_,_>>>()?;
            if let Some((rg,rw))=&reference {
                for (name,g) in &flat {for (x,y) in g.iter().zip(&rg[name]){assert!((x-y).abs()<2e-5,"gradient {name}");}}
                for (name,w) in &weights {for (x,y) in w.iter().zip(&rw[name]){assert!((x-y).abs()<2e-5,"Adam {name}");}}
            } else {reference=Some((flat,weights));}
        }
        println!("ANSWER_NATIVE random_TINY optimizer=4 native_forward={forwards} backward={forwards} generation=0 teacher=0 tolerance=2e-5");
        Ok(())
    }
    #[test]
    fn record_grounding_matches_scalar_gradient_and_mask_contract() -> Result<()> {
        let device=Device::Cpu;
        let qv:Vec<f32>=(0..48).map(|i|(i%13) as f32*0.17-0.9).collect();
        let kv:Vec<f32>=(0..24).map(|i|(i%11) as f32*0.13-0.5).collect();
        let q=Var::from_vec(qv.clone(),(2,2,6,2),&device)?;
        let k=Var::from_vec(kv.clone(),(2,1,6,2),&device)?;
        let mask:Vec<f32>=(0..72).map(|i|f32::from(i%6<=i/6%6)).collect();
        let allowed=Tensor::from_vec(mask.clone(),(2,1,6,6),&device)?;
        let label=RecordGrounding{query:4,records:[(1,2),(2,4)],selected:0};
        let scalar=|q:&[f32],k:&[f32],selected:usize| {
            let mut mass=[0f64;2];
            for head in 0..2 {
                let scores:Vec<_>=(0..5).map(|p|(0..2).map(|d|f64::from(q[(head*6+4)*2+d])*f64::from(k[p*2+d])).sum::<f64>()/2f64.sqrt()).collect();
                let norm=scores.iter().map(|s|s.exp()).sum::<f64>();
                mass[0]+=scores[1].exp()/norm;
                mass[1]+=(scores[2].exp()+scores[3].exp())/norm;
            }
            -0.1*(mass[selected]/(mass[0]+mass[1])).max(1e-8).ln()
        };
        for selected in 0..2 {
            let mut l=label.clone();l.selected=selected;
            let loss=record_grounding_loss(&q,&k,&allowed,&[Some(l),None])?;
            assert!((f64::from(loss.to_scalar::<f32>()?)-scalar(&qv,&kv,selected)).abs()<1e-7);
            let grads=loss.backward()?;
            for (var,values,is_q) in [(&q,&qv,true),(&k,&kv,false)] {
                let got=grads.get(var).unwrap().flatten_all()?.to_vec1::<f32>()?;
                assert!(got.iter().any(|g|g.abs()>1e-5));
                for (i,g) in got.iter().enumerate() {
                    let mut lo=values.clone();let mut hi=values.clone();lo[i]-=0.001;hi[i]+=0.001;
                    let eval=|v:&[f32]|if is_q {scalar(v,&kv,selected)}else{scalar(&qv,v,selected)};
                    let expected=(eval(&hi)-eval(&lo))/f64::from(hi[i]-lo[i]);
                    assert!((f64::from(*g)-expected).abs()<2e-6,"q={is_q} index={i}: {g} != {expected}");
                    if i>=values.len()/2 || (!is_q && i/2==5) || (is_q && i/2%6!=4) {assert_eq!(*g,0.);}
                }
            }
        }
        assert_eq!(record_grounding_loss(&q,&k,&allowed,&[None,None])?.to_scalar::<f32>()?,0.);
        for l in [RecordGrounding{selected:2,..label.clone()},RecordGrounding{query:6,..label.clone()},
            RecordGrounding{records:[(1,1),(2,4)],..label.clone()},RecordGrounding{records:[(1,3),(2,4)],..label.clone()},
            RecordGrounding{records:[(1,2),(2,5)],..label.clone()}] {
            assert!(record_grounding_loss(&q,&k,&allowed,&[Some(l),None]).is_err());
        }
        let mut hidden=mask;hidden[4*6+1]=0.;
        assert!(record_grounding_loss(&q,&k,&Tensor::from_vec(hidden,(2,1,6,6),&device)?,&[Some(label.clone()),None]).is_err());
        assert!(record_grounding_loss(&q,&k,&allowed,&[Some(label)]).is_err());
        assert!(record_grounding_loss(&q,&k.narrow(3,0,1)?,&allowed,&[None,None]).is_err());
        println!("GROUNDING_SCALAR backward=2 optimizer=0 model_forward=0 generation=0 teacher=0");
        Ok(())
    }
    #[test]
    fn record_grounding_reaches_random_native_parameters_without_changing_logits() -> Result<()> {
        let model=Transformer::init(neural::transformer::Config::tiny(264),93,Device::Cpu)?;
        let ids=Tensor::new(&[[8u32,9,10,11,12,13]],&Device::Cpu)?;
        let label=RecordGrounding{query:4,records:[(1,2),(2,4)],selected:0};
        let before=model.weight_hash()?;
        let ordinary=model.forward(&ids,None)?;
        let mut auxiliary=None;
        let observed=model.forward_observed(&ids,None,&mut |layer,q,k,allowed| {
            if layer+1==model.config.layers {auxiliary=Some(record_grounding_loss(q,k,allowed,&[Some(label.clone())])?);}
            Ok(())
        })?;
        assert_eq!(ordinary.flatten_all()?.to_vec1::<f32>()?,observed.flatten_all()?.to_vec1::<f32>()?);
        let auxiliary=auxiliary.unwrap();assert!(auxiliary.to_scalar::<f32>()?>0.);
        let gradients=auxiliary.backward()?;
        for name in ["embedding","layer.1.q","layer.1.k","layer.1.q_norm","layer.1.k_norm"] {
            let g=gradients.get(&model.vars[name]).unwrap().flatten_all()?.to_vec1::<f32>()?;
            assert!(g.iter().all(|v|v.is_finite()));assert!(g.iter().any(|v|v.abs()>1e-8),"{name}");
        }
        assert_eq!(before,model.weight_hash()?);
        println!("GROUNDING_NATIVE model_forward=2 backward=1 optimizer=0 generation=0 teacher=0 logits=EXACT gradients=NONZERO");
        Ok(())
    }
    #[test]
    fn decode_failure_preserves_generated_tokens_and_eos_receipt() {
        let tok =
            ByteBpe::train(&["가".as_bytes().to_vec()], &neural::hash(b"fixture"), 264).unwrap();
        let ids = tok.encode(&[0xea, 0xb0]).unwrap();
        let generated = neural::transformer::Generated {
            synchronized_phases_ms: None,
            tokens: ids.clone(),
            generated: ids.len() + 1,
            finish: "stop".into(),
            first_token_ms: 0,
            generation_ms: 0,
            cache_bytes: 0,
            retained: vec![],
            attention_workspace_bytes: 0,
        };
        let (text, raw, error) = decode_generated(&tok, Ok(generated));
        assert!(text.is_none() && error.is_some());
        let raw = raw.expect("strict decode failure must not erase actual generation");
        assert_eq!(raw.tokens, ids);
        assert_eq!(raw.finish, "stop");
        assert_eq!(raw.generated, raw.tokens.len() + 1);
    }
    #[test]
    fn masked_first_response_learns_context_and_generates_without_future_labels() {
        let device = Device::Cpu;
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 137, device.clone()).unwrap();
        // Every prompt ends identically. Its only varying token precedes the local
        // window; the first response must therefore learn to use earlier context.
        let samples: Vec<_> = (8..12)
            .map(|cue| {
                let mut tokens = vec![BOS, cue, 31, 32, 33, 34, 35, 36, 37, neural::ASSISTANT_ROLE];
                let response_start = tokens.len();
                tokens.push(cue);
                tokens.extend(40..51);
                tokens.push(EOS);
                Sample {
                    tokens,
                    response_start,
                    curriculum: false,
                }
            })
            .collect();
        let b = batch(&samples, &[0, 1, 2, 3], &device).unwrap();
        let first_loss = || {
            masked_loss(
                &model.forward(&b.input, Some(&b.valid)).unwrap(),
                &b.target,
                &b.first_target_mask,
            )
            .unwrap()
            .0
            .to_scalar::<f32>()
            .unwrap()
        };
        let initial = first_loss();
        let config = TrainConfig {
            lr: 0.005,
            warmup: 10,
            max_steps: 200,
            seq_len: 64,
            microbatch: 4,
            accumulation: 1,
            weight_decay: 0.,
            ..Default::default()
        };
        config.validate(64).unwrap();
        let mut adam = Adam::new(&model.vars).unwrap();
        for step in 1..=config.max_steps {
            let (_, objective, _) =
                response_loss(&model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 1.).unwrap();
            let gradients = objective.backward().unwrap();
            let gradients = model
                .vars
                .iter()
                .map(|(name, var)| (name.clone(), gradients.get(var).unwrap().detach()))
                .collect();
            let (norm, delta) = adam.step(&model.vars, &gradients, &config, step).unwrap();
            assert!(norm.is_finite() && delta.is_finite());
            if step == 1 {
                assert!(norm > 0. && delta > 0.);
            }
        }
        let learned = first_loss();
        println!("TINY_NUMERIC_TEST_ONLY first_target_nll {initial}->{learned}; 200 real updates");
        assert!(learned < initial);
        for sample in &samples {
            // Autoregressive generation gets only the prompt, never its labels.
            let generated = model
                .generate(
                    &sample.tokens[..sample.response_start],
                    24,
                    30000,
                    &AtomicBool::new(false),
                    "first-response-regression",
                )
                .unwrap();
            assert_eq!(generated.tokens.first(), Some(&sample.tokens[1]));
        }
    }
    #[test]
    fn record_ablation_selects_explicit_current_target_without_gold() {
        use replica_v3::{
            model::ModelRequest,
            retrieval::{Evidence, EvidenceBundle},
        };
        let record = |id, name: &str, context: &str, value: &str, status: &str| Evidence {
            event_id: id,
            original_excerpt: format!("{name}의 {context} 이동 지시는 {value}이다."),
            excerpt_truncated: false,
            source: "fixture".into(),
            recorded_at: 100 - id,
            observed_at: None,
            version_status: status.into(),
            retrieval_reason: "lexical".into(),
            relation_path: vec![],
        };
        let request = ModelRequest {
            request_id: "record-ablation".into(),
            system: String::new(),
            input: "번호 531904인 대상의 구역8에 현재 적용되는 방향 값 한 단어만 답하라.".into(),
            limits: Default::default(),
            evidence: EvidenceBundle {
                items: vec![
                    record(71, "센서531904", "구역80", "왼쪽", "current"),
                    record(5, "센서531904", "구역8", "오른쪽", "superseded"),
                    record(99, "센서1531904", "구역8", "직진", "current"),
                    record(14, "센서531904", "구역8", "서쪽", "current"),
                ],
                ..Default::default()
            },
        };
        let mut isolated = request.clone();
        isolate_current_record(
            &mut isolated,
            "센서531904",
            "구역8",
            RecordTargetMode::ExplicitNumericField,
        )
        .unwrap();
        assert_eq!(isolated.evidence.items.len(), 1);
        assert_eq!(
            replica_v3::binary::to_value(&isolated.evidence.items[0]).unwrap(),
            replica_v3::binary::to_value(&request.evidence.items[3]).unwrap()
        );
        assert_eq!(isolated.input, request.input);
        let mut changed = request.clone();
        changed.evidence.items[3].original_excerpt =
            "센서531904의 구역8 이동 지시는 대기이다.".into();
        isolate_current_record(
            &mut changed,
            "센서531904",
            "구역8",
            RecordTargetMode::ExplicitNumericField,
        )
        .unwrap();
        assert_eq!(changed.evidence.items[0].event_id, 14);
        assert!(
            changed.evidence.items[0]
                .original_excerpt
                .ends_with("대기이다.")
        );
        for (entity, context) in [("센서53190", "구역8"), ("센서531904", "구역80")] {
            let mut invalid = request.clone();
            assert!(
                isolate_current_record(
                    &mut invalid,
                    entity,
                    context,
                    RecordTargetMode::ExplicitNumericField
                )
                .is_err()
            );
            assert_eq!(invalid.evidence.items.len(), 4);
        }
        for duplicate in [false, true] {
            let mut invalid = request.clone();
            if duplicate {
                invalid
                    .evidence
                    .items
                    .push(invalid.evidence.items[3].clone());
            } else {
                invalid.evidence.items.pop();
            }
            let before = replica_v3::binary::to_value(&invalid).unwrap();
            assert!(
                isolate_current_record(
                    &mut invalid,
                    "센서531904",
                    "구역8",
                    RecordTargetMode::ExplicitNumericField
                )
                .is_err()
            );
            assert_eq!(replica_v3::binary::to_value(&invalid).unwrap(), before);
        }
        let mut qa = request.clone();
        qa.input = "센서531904의 구역8 이동 지시와 근거는?".into();
        qa.evidence
            .items
            .push(record(77, "장비531904", "구역8", "대기", "current"));
        isolate_current_record(
            &mut qa,
            "센서531904",
            "구역8",
            RecordTargetMode::ExactEntity,
        )
        .unwrap();
        assert_eq!(qa.evidence.items[0].event_id, 14);
        qa.evidence.items[0].original_excerpt = "장비531904의 구역8 이동 지시는 서쪽이다.".into();
        let before = replica_v3::binary::to_value(&qa).unwrap();
        assert!(
            isolate_current_record(
                &mut qa,
                "센서531904",
                "구역8",
                RecordTargetMode::ExactEntity
            )
            .is_err()
        );
        assert_eq!(replica_v3::binary::to_value(&qa).unwrap(), before);
    }
    #[test]
    fn harness_m02_qa_wrong_full_name_rejected_before_mutation() {
        let mut request = replica_v3::model::ModelRequest {
            request_id: "independent-target-mismatch".into(),
            system: String::new(),
            input: "장비31의 구역1 이동 지시와 근거는?".into(),
            limits: Default::default(),
            evidence: replica_v3::retrieval::EvidenceBundle {
                items: vec![replica_v3::retrieval::Evidence {
                    event_id: 23,
                    original_excerpt: "센서31의 구역1 이동 지시는 동쪽이다.".into(),
                    excerpt_truncated: false,
                    source: "fixture".into(),
                    recorded_at: 1,
                    observed_at: None,
                    version_status: "current".into(),
                    retrieval_reason: "fixture".into(),
                    relation_path: vec![],
                }],
                ..Default::default()
            },
        };
        let before = replica_v3::binary::to_value(&request).unwrap();
        assert!(
            isolate_current_record(
                &mut request,
                "센서31",
                "구역1",
                RecordTargetMode::ExactEntity
            )
            .is_err()
        );
        assert_eq!(replica_v3::binary::to_value(&request).unwrap(), before);
        for input in [
            "센서310의 구역1 지시는?",
            "센서31의 구역10 지시는?",
            "구역1 지시는?",
            "다른센서31의 구역1 지시는?",
        ] {
            request.input = input.into();
            let before = replica_v3::binary::to_value(&request).unwrap();
            assert!(
                isolate_current_record(
                    &mut request,
                    "센서31",
                    "구역1",
                    RecordTargetMode::ExactEntity
                )
                .is_err()
            );
            assert_eq!(replica_v3::binary::to_value(&request).unwrap(), before);
        }
        request.input = "센서31의 구역1 지시는?".into();
        isolate_current_record(
            &mut request,
            "센서31",
            "구역1",
            RecordTargetMode::ExactEntity,
        )
        .unwrap();
        request.input = "번호 31 대상의 구역1 방향만 복사해줘.".into();
        assert!(
            isolate_current_record(
                &mut request,
                "센서31",
                "구역1",
                RecordTargetMode::ExactEntity
            )
            .is_err()
        );
        isolate_current_record(
            &mut request,
            "센서31",
            "구역1",
            RecordTargetMode::ExplicitNumericField,
        )
        .unwrap();
        let mut other = request.evidence.items[0].clone();
        other.event_id = 51;
        other.original_excerpt = "장비31의 구역1 이동 지시는 서쪽이다.".into();
        request.evidence.items.push(other);
        for _ in 0..2 {
            let before = replica_v3::binary::to_value(&request).unwrap();
            assert!(
                isolate_current_record(
                    &mut request,
                    "센서31",
                    "구역1",
                    RecordTargetMode::ExplicitNumericField
                )
                .is_err()
            );
            assert_eq!(replica_v3::binary::to_value(&request).unwrap(), before);
            let mut qa = request.clone();
            qa.input = "센서31의 구역1 지시는?".into();
            isolate_current_record(&mut qa, "센서31", "구역1", RecordTargetMode::ExactEntity)
                .unwrap();
            assert_eq!(qa.evidence.items[0].event_id, 23);
            request.evidence.items.reverse();
        }
    }
    #[test]
    fn grouped_sampling_preserves_default_draws_and_complete_blocks() {
        let pool = [2, 5, 9, 12, 17, 21, 24, 28];
        let mut config = TrainConfig {
            microbatch: 8,
            ..Default::default()
        };
        let mut actual = Rng::new(97);
        let mut reference = Rng::new(97);
        for _ in 0..5 {
            let expected: Vec<_> = (0..8)
                .map(|_| pool[(reference.next_u64() % 8) as usize])
                .collect();
            assert_eq!(draw_indices(&pool, &config, &mut actual).unwrap(), expected);
            assert_eq!(actual.state, reference.state);
        }
        config.sample_group_size = 4;
        for _ in 0..5 {
            let selected = draw_indices(&pool, &config, &mut actual).unwrap();
            for block in selected.as_chunks::<4>().0 {
                let offset = (reference.next_u64() % 2) as usize * 4;
                assert_eq!(block, &pool[offset..offset + 4]);
            }
            assert_eq!(actual.state, reference.state);
        }
        let before = actual.state;
        for invalid_pool in [&pool[..7], &pool[..0]] {
            assert!(draw_indices(invalid_pool, &config, &mut actual).is_err());
            assert_eq!(actual.state, before);
        }
        for invalid_group in [0, 3, 16] {
            config.sample_group_size = invalid_group;
            assert!(config.validate(2048).is_err());
            assert!(draw_indices(&pool, &config, &mut actual).is_err());
            assert_eq!(actual.state, before);
        }
        let mut legacy = replica_v3::binary::to_value(TrainConfig::default()).unwrap();
        legacy.as_object_mut().unwrap().remove("sample_group_size");
        let decoded: TrainConfig = replica_v3::binary::from_value(legacy).unwrap();
        assert_eq!(decoded.sample_group_size, 1);
    }
    #[test]
    fn field_question_ablation_rejects_unmentioned_targets_and_never_reads_gold() {
        let q = "장비531904(구역8)의 식별 숫자 부분만 보여줘.";
        let actual = known_field_question_form(q, "number", "장비531904", "구역8").unwrap();
        assert!(actual.contains("장비531904") && actual.contains("구역8"));
        assert!(!actual.contains("event:") && !actual.contains("왼쪽"));
        assert!(known_field_question_form(q, "number", "장비53190", "구역8").is_err());
        assert!(known_field_question_form(q, "context", "장비531904", "구역80").is_err());
        let q = "번호 531904인 대상의 구역8에 현재 적용되는 방향 값 한 단어만 답하라.";
        assert!(known_field_question_form(q, "value", "장비531904", "구역8").is_ok());
        assert!(known_field_question_form(q, "value", "장비53190", "구역8").is_err());
        assert!(
            known_field_question_form(
                "번호 1531904인 대상의 구역8",
                "value",
                "장비531904",
                "구역8"
            )
            .is_err()
        );
        assert!(known_field_question_form(q, "value", "장비", "구역8").is_err());
        assert!(known_field_question_form(q, "unsupported", "531904", "구역8").is_err());
        assert!(
            known_field_question_form("대상의 종류 이름만 적어줘.", "entity-cue", "", "").is_ok()
        );
    }
    #[test]
    fn question_ablation_preserves_explicit_targets_without_reading_answer_fields() {
        let original = "자료에 현장9도 나오지만 구역8에 해당하는 장비531904의 방향을 읽어줘.";
        let a = known_question_form(2, original, "장비531904", "구역8").unwrap();
        assert!(a.contains("장비531904") && a.contains("구역8"));
        assert!(!a.contains("event:") && !a.contains("오른쪽"));
        assert!(known_question_form(2, original, "장비123", "구역8").is_err());
        assert!(known_question_form(2, original, "장비531904", "통로7").is_err());
        assert!(known_question_form(2, original, "장비53190", "구역8").is_err());
        assert!(known_question_form(2, original, "", "구역8").is_err());
        assert!(known_question_form(1, original, "장비531904", "구역8").is_err());
        let current = known_question_form(0, original, "장비531904", "구역8").unwrap();
        assert!(current.contains("장비531904") && current.contains("구역8"));
    }
    #[test]
    fn first_target_objective_matches_scalar_ce_and_gradients_without_mask_leakage() {
        let device = Device::Cpu;
        let samples = [
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, EOS],
                response_start: 1,
                curriculum: false,
            },
        ];
        let b = batch(&samples, &[0, 1], &device).unwrap();
        assert_eq!(
            b.first_target_mask.to_vec2::<f32>().unwrap(),
            [vec![0., 0., 1., 0.], vec![1., 0., 0., 0.]]
        );
        let values: Vec<f32> = (0..96).map(|i| (i % 19) as f32 * 0.1 - 0.9).collect();
        let logits = Var::from_vec(values.clone(), (2, 4, 12), &device).unwrap();
        let labels = b.target.flatten_all().unwrap().to_vec1::<u32>().unwrap();
        let masks = b.mask.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for weight in [1., 4., 8.] {
            let (ce, objective, count) = response_loss(&logits, &b, weight).unwrap();
            assert_eq!(count, 4); // Denominator is actual supervised tokens, not weighted count or PAD.
            let gradients = objective.backward().unwrap();
            let actual = gradients
                .get(&logits)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            let mut raw = 0.;
            let mut weighted = 0.;
            for (row, tokens) in values.as_chunks::<12>().0.iter().enumerate() {
                let sum: f64 = tokens.iter().map(|v| f64::from(*v).exp()).sum();
                let label = labels[row] as usize;
                let nll = sum.ln() - f64::from(tokens[label]);
                let w = if row == 2 || row == 4 { weight } else { 1. };
                raw += nll * f64::from(masks[row]) / 4.;
                weighted += nll * f64::from(masks[row]) * w / 4.;
                for (column, token) in tokens.iter().enumerate() {
                    let p = f64::from(*token).exp() / sum;
                    let expected =
                        (p - f64::from(column == label)) * f64::from(masks[row]) * w / 4.;
                    assert!((f64::from(actual[row * 12 + column]) - expected).abs() < 1e-6);
                }
            }
            assert!((f64::from(ce.to_scalar::<f32>().unwrap()) - raw).abs() < 1e-6);
            assert!((f64::from(objective.to_scalar::<f32>().unwrap()) - weighted).abs() < 2e-6);
        }
        for invalid in [0., 17., f64::NAN, f64::INFINITY] {
            assert!(response_loss(&logits, &b, invalid).is_err());
        }
        let mut legacy = replica_v3::binary::to_value(TrainConfig::default()).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("first_target_weight");
        let decoded: TrainConfig = replica_v3::binary::from_value(legacy).unwrap();
        assert_eq!(decoded.first_target_weight, 1.);
    }
    #[test]
    fn paired_contrast_matches_scalar_gradients_shared_prefix_and_boundaries() {
        let device=Device::Cpu;
        let samples=[
            Sample{tokens:vec![BOS,8,9,10,11,EOS],response_start:3,curriculum:false},
            Sample{tokens:vec![BOS,12,13,8,10,12,EOS],response_start:4,curriculum:false},
        ];
        let values:Vec<f32>=(0..192).map(|i|(i%23) as f32*0.07-0.8).collect();
        let ix=|r:usize,p:usize,t:usize| (r*6+p)*16+t;
        let (aa,ab,ba,bb)=(ix(0,3,11),ix(0,3,12),ix(1,4,11),ix(1,4,12));
        for sidewise in [false,true] { for shift in [0.,7.] {
            let mut v=values.clone();v[aa]+=shift;v[ba]+=shift;
            let logits=Var::from_vec(v.clone(),(2,6,16),&device).unwrap();
            let loss=pair_contrast_loss(&logits,&samples,&[0,1],&[(0,1)],sidewise).unwrap();
            let x=1.-f64::from(v[aa]-v[ab]+v[bb]-v[ba]);
            let xa=1.-f64::from(v[aa]-v[ab]);let xb=1.-f64::from(v[bb]-v[ba]);
            let expected=if sidewise {0.05*(xa.exp().ln_1p()+xb.exp().ln_1p())}else{0.1*x.exp().ln_1p()};
            assert!((f64::from(loss.to_scalar::<f32>().unwrap())-expected).abs()<1e-6);
            let grad=loss.backward().unwrap().get(&logits).unwrap().flatten_all().unwrap().to_vec1::<f32>().unwrap();
            let ga=if sidewise {0.05/(1.+(-xa).exp())}else{0.1/(1.+(-x).exp())};
            let gb=if sidewise {0.05/(1.+(-xb).exp())}else{ga};
            for (i,actual) in grad.iter().enumerate() {
                let expected=if i==aa {-ga}else if i==ab {ga}else if i==bb {-gb}else if i==ba {gb}else{0.};
                assert!((f64::from(*actual)-expected).abs()<1e-6,"gradient at {i}");
            }
            assert_eq!(loss.to_scalar::<f32>().unwrap(),pair_contrast_loss(&logits,&samples,&[0,1],&[(1,0)],sidewise).unwrap().to_scalar::<f32>().unwrap());
            assert_eq!(pair_contrast_loss(&logits,&samples,&[0,1],&[],sidewise).unwrap().to_scalar::<f32>().unwrap(),0.);
            for pairs in [vec![(0,0)],vec![(0,2)],vec![(0,1),(1,0)]] {
                assert!(pair_contrast_loss(&logits,&samples,&[0,1],&pairs,sidewise).is_err());
            }
            assert!(pair_contrast_loss(&logits,&samples,&[0,0],&[(0,1)],sidewise).is_err());
            assert!(pair_contrast_loss(&logits,&samples,&[0,2],&[(0,1)],sidewise).is_err());
        }}
        // Same sum2: separate penalties must distinguish balanced1/1 from3/-1.
        let mut aggregate=vec![];let mut separate=vec![];
        for (da,db) in [(1.,1.),(3.,-1.)] {
            let mut v=vec![0f32;192];v[aa]=da;v[bb]=db;
            let logits=Tensor::from_vec(v,(2,6,16),&device).unwrap();
            aggregate.push(pair_contrast_loss(&logits,&samples,&[0,1],&[(0,1)],false).unwrap().to_scalar::<f32>().unwrap());
            separate.push(pair_contrast_loss(&logits,&samples,&[0,1],&[(0,1)],true).unwrap().to_scalar::<f32>().unwrap());
        }
        assert_eq!(aggregate[0],aggregate[1]);assert!(separate[1]>separate[0]);
        for sidewise in [false,true] { for sign in [-1.,1.] {
            let mut v=values.clone();for i in [aa,bb] {v[i]=sign*1000.;}for i in [ab,ba] {v[i]=-sign*1000.;}
            let logits=Var::from_vec(v,(2,6,16),&device).unwrap();
            let loss=pair_contrast_loss(&logits,&samples,&[0,1],&[(0,1)],sidewise).unwrap();
            assert!(loss.to_scalar::<f32>().unwrap().is_finite());
            assert!(loss.backward().unwrap().get(&logits).unwrap().flatten_all().unwrap().to_vec1::<f32>().unwrap().iter().all(|v|v.is_finite()));
        }}
        let b=batch(&samples,&[0,1],&device).unwrap();
        let logits=Tensor::from_vec(values,(2,6,16),&device).unwrap();
        let (ce,objective,count)=response_loss(&logits,&b,1.).unwrap();
        assert_eq!(count,6);assert_eq!(ce.to_scalar::<f32>().unwrap(),objective.to_scalar::<f32>().unwrap());
        println!("PAIR_CONTRAST_SCALAR optimizer=0 generation=0 teacher=0");
    }
    #[test]
    fn citation_precision_adam_lr_f64_reference() -> Result<()> {
        let device=Device::Cpu;
        let model=Transformer::init(neural::transformer::Config::tiny(264),93,device.clone())?;
        let ss=vec![Sample{tokens:vec![BOS,8,9,10,EOS],response_start:3,curriculum:false}];
        let b=batch(&ss,&[0],&device)?;
        let logits=model.forward(&b.input,Some(&b.valid))?;
        let (_,loss,_,_)=response_objective(&logits,&b,1.,true)?;
        let backward=loss.backward()?;
        let grads=model.vars.iter().map(|(n,v)|Ok((n.clone(),backward.get(v).ok_or_else(||Error::Model("precision disconnected gradient".into()))?.detach()))).collect::<Result<BTreeMap<_,_>>>()?;
        let config=TrainConfig{lr:3e-4,warmup:0,clip:1.,weight_decay:0.01,..Default::default()};
        let mut inherited=Adam::new(&model.vars)?;
        inherited.step_constant(&model.vars,&grads,&config,10496,3e-4)?;
        let mut vars=Vec::new();let mut adams=Vec::new();
        for _ in 0..2 {
            vars.push(model.vars.iter().map(|(n,v)|Ok((n.clone(),Var::from_tensor(&v.as_detached_tensor())?))).collect::<Result<BTreeMap<_,_>>>()?);
            adams.push(Adam{moments:inherited.moments.iter().map(|(n,v)|(n.clone(),v.clone())).collect()});
        }
        let norm=grads.values().map(|g|g.flatten_all()?.to_vec1::<f32>().map(|v|v.into_iter().map(|x|f64::from(x).powi(2)).sum::<f64>())).collect::<candle_core::Result<Vec<_>>>()?.iter().sum::<f64>().sqrt();
        let clip=(config.clip/(norm+1e-12)).min(1.);
        let rates=[3e-4,3e-5];
        for i in 0..2 {adams[i].step_constant(&vars[i],&grads,&config,10497,rates[i])?;}
        let mut nonzero=0;let mut maximum_error=0f64;let mut coordinates=0;
        for (name,var) in &model.vars {
            let old=var.flatten_all()?.to_vec1::<f32>()?;let g=grads[name].flatten_all()?.to_vec1::<f32>()?;
            let m=inherited.moments[&format!("adam.m.{name}")].flatten_all()?.to_vec1::<f32>()?;
            let v=inherited.moments[&format!("adam.v.{name}")].flatten_all()?.to_vec1::<f32>()?;
            let mut next=vec![];
            for branch in &vars {next.push(branch[name].flatten_all()?.to_vec1::<f32>()?);}
            for prefix in ["adam.m.","adam.v."] {
                let key=format!("{prefix}{name}");
                assert_eq!(adams[0].moments[&key].flatten_all()?.to_vec1::<f32>()?,adams[1].moments[&key].flatten_all()?.to_vec1::<f32>()?);
            }
            let actual_m=adams[0].moments[&format!("adam.m.{name}")].flatten_all()?.to_vec1::<f32>()?;
            let actual_v=adams[0].moments[&format!("adam.v.{name}")].flatten_all()?.to_vec1::<f32>()?;
            for j in 0..old.len() {
                coordinates+=1;nonzero+=usize::from(m[j]!=0. && v[j]!=0.);
                let theta=f64::from(old[j]);let grad=f64::from(g[j])*clip;
                let mr=config.beta1*f64::from(m[j])+(1.-config.beta1)*grad;
                let vr=config.beta2*f64::from(v[j])+(1.-config.beta2)*grad*grad;
                for (a,r) in [(f64::from(actual_m[j]),mr),(f64::from(actual_v[j]),vr)] {assert!((a-r).abs()<=1e-10+2e-6*r.abs());}
                let u=(mr/(1.-config.beta1.powi(10497)))/((vr/(1.-config.beta2.powi(10497))).sqrt()+config.eps);
                let mut tolerance=[0.;2];let mut delta=[0.;2];let mut reference=[0.;2];
                for i in 0..2 {
                    reference[i]=-rates[i]*(config.weight_decay*theta+u);delta[i]=f64::from(next[i][j])-theta;
                    tolerance[i]=2.*f64::from(f32::EPSILON)*theta.abs()+2e-5*reference[i].abs()+1e-10;
                    let error=(delta[i]-reference[i]).abs();maximum_error=maximum_error.max(error);
                    assert!(error<=tolerance[i],"{name}/{j} LR{} error{error} bound{}",rates[i],tolerance[i]);
                }
                assert!((reference[1]-0.1*reference[0]).abs()<=1e-12*reference[0].abs().max(1e-12));
                assert!((delta[1]-0.1*delta[0]).abs()<=tolerance[1]+0.1*tolerance[0]);
            }
        }
        assert!(nonzero>0);
        println!("CITATION_PRECISION_ADAM coordinates={coordinates} inherited_nonzero={nonzero} max_absolute_delta_error={maximum_error} optimizer3 forward1 backward1 generation0 teacher0 FD0");Ok(())
    }
    #[test]
    fn adam_matches_independent_reference_and_teacher_forcing_masks() {
        let device = Device::Cpu;
        let mut vars = BTreeMap::new();
        vars.insert("x".into(), Var::new(&[1f32, -2.], &device).unwrap());
        let mut adam = Adam::new(&vars).unwrap();
        let config = TrainConfig {
            warmup: 0,
            max_steps: 10,
            clip: 100.,
            ..Default::default()
        };
        let mut grads = BTreeMap::new();
        grads.insert("x".into(), Tensor::new(&[0.5f32, -0.25], &device).unwrap());
        let lr = config.learning_rate(1);
        adam.step(&vars, &grads, &config, 1).unwrap();
        let actual = vars["x"].to_vec1::<f32>().unwrap();
        for (index, (w, g)) in [(1f64, 0.5f64), (-2., -0.25)].into_iter().enumerate() {
            let expected = w * (1. - lr * config.weight_decay) - lr * g / (g.abs() + config.eps);
            assert!((actual[index] as f64 - expected).abs() < 1e-6);
        }
        let samples = vec![
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, EOS],
                response_start: 1,
                curriculum: false,
            },
        ];
        let b = batch(&samples, &[0, 1], &device).unwrap();
        assert_eq!(
            b.input.to_vec2::<u32>().unwrap(),
            vec![vec![BOS, 8, 9, 10], vec![BOS, 11, PAD, PAD]]
        );
        assert_eq!(
            b.target.to_vec2::<u32>().unwrap(),
            vec![vec![8, 9, 10, EOS], vec![11, EOS, PAD, PAD]]
        );
        assert_eq!(
            b.mask.to_vec2::<f32>().unwrap(),
            vec![vec![0., 0., 1., 1.], vec![1., 1., 0., 0.]]
        );
        let logits = Tensor::zeros((2, 4, 264), DType::F32, &device).unwrap();
        let (loss, n) = masked_loss(&logits, &b.target, &b.mask).unwrap();
        assert_eq!(n, 4);
        assert!((loss.to_scalar::<f32>().unwrap() as f64 - 264f64.ln()).abs() < 1e-6);
        let mut clipped = Adam::new(&vars).unwrap();
        let config = TrainConfig { clip: 1., ..config };
        grads.insert("x".into(), Tensor::new(&[3f32, 4.], &device).unwrap());
        let (norm, _) = clipped.step(&vars, &grads, &config, 1).unwrap();
        assert!((norm - 5.).abs() < 1e-6);
        let moment = clipped.moments["adam.m.x"].to_vec1::<f32>().unwrap();
        assert!((moment[0] - 0.06).abs() < 1e-6 && (moment[1] - 0.08).abs() < 1e-6);
        let before = vars["x"].to_vec1::<f32>().unwrap();
        grads.insert("x".into(), Tensor::new(&[f32::NAN, 1.], &device).unwrap());
        assert!(clipped.step(&vars, &grads, &config, 2).is_err());
        assert_eq!(vars["x"].to_vec1::<f32>().unwrap(), before);
    }
    #[test]
    fn harness_h3_constant_rate_independent_scalar_reference() {
        let vars = BTreeMap::from([("x".into(), Var::new(&[1f32], &Device::Cpu).unwrap())]);
        let mut adam = Adam::new(&vars).unwrap();
        let c = TrainConfig {
            lr: 0.001,
            warmup: 0,
            max_steps: 100,
            clip: 100.,
            ..Default::default()
        };
        let (mut weight, mut m, mut v) = (1f64, 0f64, 0f64);
        let rate = 3e-5;
        for (clock, g) in [(51usize, 0.5f64), (52, -0.25)] {
            m = c.beta1 * m + (1. - c.beta1) * g;
            v = c.beta2 * v + (1. - c.beta2) * g * g;
            weight = weight * (1. - rate * c.weight_decay)
                - rate * (m / (1. - c.beta1.powi(clock as i32)))
                    / ((v / (1. - c.beta2.powi(clock as i32))).sqrt() + c.eps);
            let grads =
                BTreeMap::from([("x".into(), Tensor::new(&[g as f32], &Device::Cpu).unwrap())]);
            adam.step_constant(&vars, &grads, &c, clock, rate).unwrap();
            assert!((f64::from(vars["x"].to_vec1::<f32>().unwrap()[0]) - weight).abs() < 1e-6);
        }
        let before = vars["x"].to_vec1::<f32>().unwrap();
        let grads = BTreeMap::from([("x".into(), Tensor::new(&[1f32], &Device::Cpu).unwrap())]);
        for bad in [f64::NAN, 0., -1.] {
            assert!(adam.step_constant(&vars, &grads, &c, 53, bad).is_err());
        }
        assert_eq!(vars["x"].to_vec1::<f32>().unwrap(), before);
        println!("H3_SCALAR_REFERENCE actual_scalar_updates=2 SMALL/TINY_updates=0");
    }
}
