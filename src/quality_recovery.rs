//! Bounded training-only recovery diagnostics; never imported by the product library.
use super::*;
use clap::Subcommand;
use replica_v3::{model::ModelRequest, neural::checkpoint::Loaded};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, io::Write, path::PathBuf};

#[derive(Subcommand)]
pub enum Command {
    /// Verify the actual one-factor traces, fresh-process artifacts and product worker.
    Close {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        control: PathBuf,
        #[arg(long)]
        treatment: PathBuf,
        #[arg(long)]
        legacy_tokenizer: PathBuf,
        #[arg(long)]
        worker_binary: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// One-factor C/W screening only, fixed tape and at most 50 real updates.
    Arm {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long, value_parser=["C","W"])]
        arm: String,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        output: PathBuf,
    },
    Freeze {
        #[arg(long)]
        parent: PathBuf,
        #[arg(long)]
        start: PathBuf,
        #[arg(long)]
        failed: PathBuf,
        #[arg(long)]
        probe: PathBuf,
        #[arg(long)]
        corpus: PathBuf,
        #[arg(long)]
        parent_corpus: PathBuf,
        #[arg(long)]
        parent_log: PathBuf,
        #[arg(long)]
        failed_log: PathBuf,
        #[arg(long)]
        source_id: String,
        #[arg(long)]
        output: PathBuf,
    },
    Replay {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        source_id: Option<String>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value="watch", value_parser=["watch", "failures", "all"])]
        panel: String,
    },
    Audit {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Numeric {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}
#[derive(Serialize, Deserialize)]
struct Frozen {
    version: u32,
    registry: Value,
    corpus: PathBuf,
    parent_corpus: PathBuf,
    start: PathBuf,
    train_hash: String,
    validation_hash: String,
    parent_train_hash: String,
    tokenizer: String,
    watch: Vec<Episode>,
    failures: Vec<Episode>,
    previous_parent: Vec<Value>,
    previous_failed: Vec<Value>,
}
fn digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    Ok(neural::hash(&serde_json::to_vec(value)?))
}
fn file_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn source_commit() -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !out.status.success() {
        return Err(Error::Invalid(
            "diagnostic source commit unavailable".into(),
        ));
    }
    Ok(String::from_utf8(out.stdout)
        .map_err(|_| Error::Invalid("source commit encoding".into()))?
        .trim()
        .to_string())
}
fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    neural::write_new(path, &serde_json::to_vec_pretty(value)?)
}
fn load(path: &Path) -> Result<Frozen> {
    let f: Frozen = serde_json::from_slice(&neural::read_bounded(path, 16 * 1024 * 1024)?)?;
    if f.version != 1 || f.watch.len() != 32 || f.failures.len() > 16 {
        return Err(Error::Corrupt("recovery frozen panel".into()));
    }
    Ok(f)
}
fn scene(e: &Episode) -> &str {
    let id =
        e.id.strip_prefix("qa-pairs/")
            .and_then(|s| s.splitn(3, '/').nth(2))
            .unwrap_or(&e.id);
    id.rsplit_once('/').map_or(id, |(a, _)| a)
}
fn rows(path: &Path) -> Result<(Value, Vec<Value>)> {
    let bytes = neural::read_bounded(path, 16 * 1024 * 1024)?;
    let values: Vec<Value> = bytes
        .split(|&b| b == b'\n')
        .filter(|b| !b.is_empty())
        .map(serde_json::from_slice)
        .collect::<std::result::Result<_, _>>()?;
    let header = values
        .first()
        .filter(|v| v["header"] == true)
        .ok_or_else(|| Error::Corrupt("evaluation header".into()))?
        .clone();
    let cases: Vec<_> = values
        .iter()
        .filter(|v| v.get("exact_match").is_some())
        .cloned()
        .collect();
    let summary = summarize(&cases)?;
    let old = values
        .last()
        .ok_or_else(|| Error::Corrupt("evaluation summary".into()))?;
    if old["summary"] != true
        || old["denominator"] != summary["denominator"]
        || old["exact_matches"] != summary["exact_matches"]
        || old["generation_failures"] != summary["generation_failures"]
    {
        return Err(Error::Corrupt("evaluation ledger count mismatch".into()));
    }
    Ok((header, cases))
}
fn summarize(rows: &[Value]) -> Result<Value> {
    let mut ids = BTreeSet::new();
    let mut groups: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let (mut errors, mut empty, mut utf8, mut first_eos, mut control, mut timeout) =
        (0, 0, 0, 0, 0, 0);
    let (mut nll, mut tokens, mut correct, mut macro_ce) = (0., 0u64, 0u64, 0.);
    let mut scored = 0;
    for row in rows {
        if !ids.insert(
            row["id"]
                .as_str()
                .ok_or_else(|| Error::Corrupt("missing case ID".into()))?,
        ) {
            return Err(Error::Corrupt(
                "duplicate case ID in score denominator".into(),
            ));
        }
        let error = row["error"].as_str().unwrap_or("");
        errors += usize::from(!error.is_empty());
        utf8 += usize::from(error.contains("UTF-8"));
        control += usize::from(error.contains("control token"));
        timeout += usize::from(error.contains("timeout"));
        empty += usize::from(row["actual"] == "" && error.is_empty());
        first_eos += usize::from(
            row["generation"]["finish"] == "stop" && row["generation"]["generated"] == 1,
        );
        let matched = row["actual"].is_string()
            && row["actual"] == row["expected"]
            && row["generation"]["finish"] == "stop"
            && error.is_empty();
        if row["exact_match"] != matched {
            return Err(Error::Corrupt("strict EM ledger mismatch".into()));
        }
        let g = if row["family"].as_str().unwrap_or("").starts_with("copy/") {
            "copy".into()
        } else {
            format!("qa-{}", row["category"])
        };
        let g = groups.entry(g).or_default();
        g[0] += usize::from(matched);
        g[1] += 1;
        let t = &row["teacher_forced_diagnostic_after_generation"];
        if let (Some(mean), Some(n), Some(c)) = (
            t["mean_nll"].as_f64(),
            t["target_tokens_including_eos"].as_u64(),
            t["teacher_forced_correct_tokens"].as_u64(),
        ) {
            nll += mean * n as f64;
            tokens += n;
            correct += c;
            macro_ce += mean;
            scored += 1;
        }
    }
    let aux = groups.get("copy").copied().unwrap_or_default();
    let qa: [usize; 2] = groups
        .iter()
        .filter(|(k, _)| *k != "copy")
        .fold([0, 0], |a, (_, v)| [a[0] + v[0], a[1] + v[1]]);
    Ok(
        json!({"summary":true,"denominator":rows.len(),"exact_matches":qa[0]+aux[0],"qa":qa,"auxiliary":aux,
        "groups_correct_total":groups,"generation_failures":errors,"invalid_utf8":utf8,"empty":empty,"first_eos":first_eos,"control":control,"timeout":timeout,
        "teacher_forced_micro_ce":(tokens>0).then(||nll/tokens as f64),"teacher_forced_macro_ce":(scored>0).then(||macro_ce/scored as f64),
        "teacher_forced_token_accuracy":(tokens>0).then(||correct as f64/tokens as f64),"teacher_forced_cases":scored,"target_tokens_including_eos":tokens,
        "final_heldout":false,"metric":"strict-full-answer-eos-v1; errors included"}),
    )
}
fn registry(path: &Path) -> Result<Value> {
    let l = checkpoint::load(path, Device::Cpu, true)?;
    let s = l
        .manifest
        .training
        .as_ref()
        .ok_or_else(|| Error::Invalid("resume state required".into()))?;
    let schedule: Vec<_> = [0,1,5,20,50,100].iter().map(|&n| json!({"segment_step":n,"model_step":s.config.budget_start_step+n,"lr":s.config.learning_rate(s.config.budget_start_step+n)})).collect();
    Ok(
        json!({"path":path,"physical_hash":file_hash(path)?,"model_content_hash":l.model.weight_hash()?,"manifest":l.manifest,
        "tokenizer_wire":l.tokenizer.id(),"tokenizer_semantic":l.tokenizer.semantic_id(),"architecture_id":l.model.config.id()?,
        "dtype":"F32","backend":neural::cpu_backend(),"adam_shapes":l.optimizer.iter().map(|(k,v)|(k,v.dims())).collect::<BTreeMap<_,_>>(),
        "cumulative_model_step":s.step,"optimizer_step":s.step,"schedule_step":s.step-s.config.budget_start_step,
        "segment_update_count":s.step-s.config.budget_start_step,"current_lr_derived_config":s.config.learning_rate(s.step),"schedule_derived_config":schedule}),
    )
}
pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Close {
            fixture,
            control,
            treatment,
            legacy_tokenizer,
            worker_binary,
            source_id,
            output,
        } => close(
            &fixture,
            &control,
            &treatment,
            &legacy_tokenizer,
            &worker_binary,
            &source_id,
            &output,
        ),
        Command::Arm {
            fixture,
            arm,
            source_id,
            output,
        } => arm_run(&fixture, &arm, &source_id, &output),
        Command::Freeze {
            parent,
            start,
            failed,
            probe,
            corpus,
            parent_corpus,
            parent_log,
            failed_log,
            source_id,
            output,
        } => {
            let (m, _, v) = data::load(&corpus)?;
            let (p, _, _) = data::load(&parent_corpus)?;
            let mut watch = Vec::new();
            let mut scenes = BTreeSet::new();
            for (category, count) in [7, 7, 6, 6, 6].into_iter().enumerate() {
                let selected: Vec<_> = v
                    .iter()
                    .filter(|e| {
                        e.category == category
                            && !e.family.starts_with("copy/")
                            && scenes.insert(scene(e).to_string())
                    })
                    .take(count)
                    .cloned()
                    .collect();
                if selected.len() != count {
                    return Err(Error::Invalid("watch metadata coverage".into()));
                }
                watch.extend(selected);
            }
            let (ph, pr) = rows(&parent_log)?;
            let (fh, fr) = rows(&failed_log)?;
            let registry = json!({"GENERAL_QA_PARENT":registry(&parent)?,"U2_POLICY_START":registry(&start)?,"U2_AFTER_20":registry(&probe)?,"U2_AFTER_250":registry(&failed)?,
                "source_id":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?,"old_parent_ledger":ph,"old_failed_ledger":fh,"parent_raw_hash":file_hash(&parent_log)?,"failed_raw_hash":file_hash(&failed_log)?,
                "parent_observed_log_summary":summarize(&pr)?,"failed_observed_log_summary":summarize(&fr)?});
            if ph["split_sha256"] != m.validation.sha256
                || fh["split_sha256"] != m.validation.sha256
                || ph["checkpoint_sha256"]
                    != registry["U2_POLICY_START"]["manifest"]["weights_sha256"]
                || fh["checkpoint_sha256"] != registry["U2_AFTER_250"]["manifest"]["weights_sha256"]
                || registry["GENERAL_QA_PARENT"]["model_content_hash"]
                    != registry["U2_POLICY_START"]["model_content_hash"]
                || pr.len() != v.len()
                || fr.len() != v.len()
            {
                return Err(Error::Corrupt("artifact/log comparison identity".into()));
            }
            for row in pr.iter().chain(&fr) {
                let e = v
                    .iter()
                    .find(|e| row["id"] == e.id)
                    .ok_or_else(|| Error::Corrupt("log case absent".into()))?;
                if row["question"] != e.request.input
                    || row["expected"] != e.answer
                    || row["evidence"] != serde_json::to_value(&e.request.evidence)?
                {
                    return Err(Error::Corrupt("old log request/target mismatch".into()));
                }
            }
            let mut failures = Vec::new();
            let mut ids = BTreeSet::new();
            for kind in 0..4 {
                for row in fr
                    .iter()
                    .filter(|r| match kind {
                        0 => r["error"].as_str().is_some_and(|e| e.contains("UTF-8")),
                        1 => r["actual"] == "",
                        2 => {
                            r["actual"].is_string()
                                && r["exact_match"] == false
                                && r["category"] != 4
                        }
                        _ => r["exact_match"] == false,
                    })
                    .filter(|r| ids.insert(r["id"].as_str().unwrap().to_string()))
                    .take(4)
                {
                    failures.push(v.iter().find(|e| row["id"] == e.id).unwrap().clone());
                }
            }
            let f = Frozen {
                version: 1,
                tokenizer: registry["U2_POLICY_START"]["tokenizer_semantic"]
                    .as_str()
                    .unwrap()
                    .into(),
                registry,
                corpus,
                parent_corpus,
                start,
                train_hash: m.train.sha256,
                validation_hash: m.validation.sha256,
                parent_train_hash: p.train.sha256,
                watch,
                failures,
                previous_parent: pr,
                previous_failed: fr,
            };
            save(&output, &f)?;
            println!(
                "{}",
                json!({"frozen":output,"hash":file_hash(&output)?,"watch":f.watch.len(),"failures":f.failures.len(),"parent":f.registry["parent_observed_log_summary"],"failed":f.registry["failed_observed_log_summary"],"optimizer_updates":0})
            );
            Ok(())
        }
        Command::Replay {
            fixture,
            checkpoint,
            source_id,
            output,
            panel,
        } => replay(&fixture, &checkpoint, &output, &panel, source_id.as_deref()),
        Command::Audit { fixture, output } => audit(&fixture, &output),
        Command::Numeric {
            fixture,
            checkpoint,
            output,
        } => numeric(&fixture, &checkpoint, &output),
    }
}

fn bytes_receipt(tok: &ByteBpe, ids: &[u32]) -> Value {
    match tok.decode_bytes(ids) {
        Err(e) => json!({"byte_mapping_error":e.to_string()}),
        Ok(bytes) => {
            let utf8 = std::str::from_utf8(&bytes);
            json!({"length":bytes.len(),"sha256":neural::hash(&bytes),"hex":bytes.iter().take(2048).map(|b|format!("{b:02x}")).collect::<String>(),"hex_truncated":bytes.len()>2048,
                "utf8_valid":utf8.is_ok(),"valid_up_to":utf8.as_ref().err().map(|e|e.valid_up_to()),"error_len":utf8.as_ref().err().and_then(|e|e.error_len()),
                "utf8_error_class":utf8.err().map(|e|if e.error_len().is_none(){"incomplete_tail"}else{"invalid_sequence"})})
        }
    }
}
fn fields(text: &str) -> Option<(&str, &str, &str)> {
    let (entity, rest) = text.split_once("의 ")?;
    let (context, value) = rest.split_once(" 이동 지시는 ")?;
    let value = value.split_once("이다.")?.0;
    Some((entity, context, value))
}
fn components(actual: Option<&str>, expected: &str, provided: &[i64]) -> Value {
    let a = actual.and_then(fields);
    let e = fields(expected);
    let ids = actual.and_then(|a| citations(a).ok());
    let expected_ids = citations(expected).ok();
    json!({"entity":e.map(|e|a.is_some_and(|a|a.0==e.0)),"context":e.map(|e|a.is_some_and(|a|a.1==e.1)),"value":e.map(|e|a.is_some_and(|a|a.2==e.2)),
        "citation_exact":ids.as_ref().is_some_and(|a|Some(a)==expected_ids.as_ref()),"citation_in_provided":ids.as_ref().map(|a|a.iter().all(|id|provided.contains(id))),"citation_nonempty":ids.as_ref().is_some_and(|a|!a.is_empty())})
}
pub(super) fn evaluate_one(loaded: &Loaded, e: &Episode, request: &ModelRequest) -> Value {
    let mut row = json!({"id":e.id,"scene":scene(e),"category":e.category,"family":e.family,"question":e.request.input,"generated_question":request.input,
        "evidence":e.request.evidence,"generated_evidence":request.evidence,"expected":e.answer,"exact_match":false,"actual":null,"error":null});
    let result = (|| -> Result<()> {
        let prompt = loaded.tokenizer.prepare(
            request,
            loaded.model.config.context as u32,
            &loaded.model.config.id()?,
        )?;
        let mut raw = Vec::new();
        let result = loaded.model.generate_observed(
            &prompt.token_ids,
            request.limits.max_tokens as usize,
            request.limits.timeout_ms,
            &AtomicBool::new(false),
            &e.id,
            |id| raw.push(id),
        );
        let (text, generated, error) = decode_generated(&loaded.tokenizer, result);
        let bytes_ids: Vec<_> = raw
            .iter()
            .copied()
            .take_while(|&id| id >= neural::SPECIALS as u32)
            .collect();
        row["raw_tokens"] = json!(raw);
        row["raw_bytes"] = bytes_receipt(&loaded.tokenizer, &bytes_ids);
        row["eos_index"] = json!(raw.iter().position(|&id| id == EOS));
        row["provided"] = json!(prompt.provided);
        row["excluded"] = json!(prompt.excluded);
        row["request_digest"] = json!(digest(request)?);
        row["prompt_digest"] = json!(digest(&prompt.token_ids)?);
        row["native_prompt_digest"] = json!(prompt.token_digest);
        row["prompt_length"] = json!(prompt.token_ids.len());
        row["exact_match"] = json!(
            text.as_deref() == Some(&e.answer)
                && generated.as_ref().is_some_and(|g| g.finish == "stop")
                && error.is_none()
        );
        row["components"] = components(text.as_deref(), &e.answer, &prompt.provided);
        row["actual"] = json!(text);
        row["generation"] = json!(generated);
        row["error"] = json!(error);
        row["error_class"] = json!(error.as_ref().map(|e| if e.contains("UTF-8") {
            "strict_utf8"
        } else if e.contains("control token") {
            "control_token"
        } else if e.contains("timeout") {
            "timeout"
        } else if e.contains("cancel") {
            "cancelled"
        } else {
            "generation_or_mapping"
        }));
        row["finish_reason"] = generated
            .as_ref()
            .map_or_else(|| row["error_class"].clone(), |g| json!(g.finish));
        row["raw_generated_count"] = json!(raw.len());
        row["whitespace_only"] = json!(
            text.as_ref()
                .is_some_and(|s| !s.is_empty() && s.trim().is_empty())
        );
        // Gold enters only after free generation has completed, including failures.
        row["teacher_forced_diagnostic_after_generation"] =
            match teacher(loaded, e, &prompt.token_ids, &raw) {
                Ok(t) => t,
                Err(e) => json!({"error":e.to_string()}),
            };
        Ok(())
    })();
    if let Err(error) = result {
        row["error"] = json!(error.to_string());
        row["error_class"] = json!("preparation_or_receipt");
    }
    row
}
fn teacher(l: &Loaded, e: &Episode, prompt: &[u32], raw: &[u32]) -> Result<Value> {
    let mut gold = l.tokenizer.encode(e.answer.as_bytes())?;
    gold.push(EOS);
    let mut sequence = prompt.to_vec();
    sequence.extend(&gold);
    if sequence.len() > l.model.config.context {
        return Err(Error::ContextTooSmall);
    }
    let logits = l
        .model
        .forward(
            &Tensor::new(&sequence[..sequence.len() - 1], &Device::Cpu)?.unsqueeze(0)?,
            None,
        )?
        .narrow(1, prompt.len() - 1, gold.len())?
        .squeeze(0)?;
    let lp = candle_nn::ops::log_softmax(&logits, 1)?.to_vec2::<f32>()?;
    if lp.iter().flatten().any(|x| !x.is_finite()) {
        return Err(Error::Model("nonfinite diagnostic logits".into()));
    }
    let predicted = logits.argmax(1)?.to_vec1::<u32>()?;
    let nll: Vec<_> = gold
        .iter()
        .enumerate()
        .map(|(i, &t)| -f64::from(lp[i][t as usize]))
        .collect();
    let mismatch = gold
        .iter()
        .zip(raw)
        .position(|(a, b)| a != b)
        .or_else(|| (gold.len() != raw.len()).then_some(gold.len().min(raw.len())));
    let bytes = l.tokenizer.decode_bytes(
        &raw.iter()
            .copied()
            .take_while(|&x| x >= neural::SPECIALS as u32)
            .collect::<Vec<_>>(),
    )?;
    let byte_difference = e
        .answer
        .as_bytes()
        .iter()
        .zip(&bytes)
        .position(|(a, b)| a != b)
        .or_else(|| (e.answer.len() != bytes.len()).then_some(e.answer.len().min(bytes.len())));
    let framed = samples(
        std::slice::from_ref(e),
        &l.tokenizer,
        l.manifest
            .training
            .as_ref()
            .map_or(512, |s| s.config.seq_len),
    )?;
    let w = l
        .manifest
        .training
        .as_ref()
        .map_or(1., |s| s.config.first_target_weight);
    let mut field_accuracy: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let mut byte = 0;
    for (i, &id) in gold.iter().enumerate() {
        let name = field_at(&e.answer, byte);
        let counts = field_accuracy.entry(name.into()).or_default();
        counts[0] += usize::from(id == predicted[i]);
        counts[1] += 1;
        if id != EOS {
            byte += l.tokenizer.decode_bytes(&[id])?.len();
        }
    }
    let difference=mismatch.filter(|&i|i<gold.len()).map(|i|{
        let rival=raw.get(i).copied().unwrap_or(predicted[i]);
        json!({"index":i,"gold_id":gold[i],"actual_id":raw.get(i),"teacher_argmax":predicted[i],"gold_log_probability":lp[i][gold[i] as usize],"gold_minus_rival_logit":lp[i][gold[i] as usize]-lp[i][rival as usize],"prefix":"gold; at first divergence identical to generation prefix"})
    });
    Ok(
        json!({"target_tokens_including_eos":gold.len(),"mean_nll":nll.iter().sum::<f64>()/gold.len() as f64,"first_target_nll":nll[0],
        "remaining_mean_nll":nll.iter().skip(1).sum::<f64>()/(gold.len()-1).max(1) as f64,"objective":(nll.iter().sum::<f64>()+(w-1.)*nll[0])/gold.len() as f64,"first_target_weight":w,
        "teacher_forced_correct_tokens":gold.iter().zip(&predicted).filter(|(a,b)|a==b).count(),"first_target_correct":gold[0]==predicted[0],"last_content_correct":gold.len()>1 && gold[gold.len()-2]==predicted[gold.len()-2],"eos_correct":predicted.last()==Some(&EOS),
        "first_argmax":predicted[0],"first_eos_probability":lp[0][EOS as usize].exp(),"first_gold_probability":lp[0][gold[0] as usize].exp(),"first_argmax_probability":lp[0][predicted[0] as usize].exp(),"first_gold_id":gold[0],
        "first_difference":difference,"first_byte_difference":byte_difference,"first_difference_field":byte_difference.map(|p|field_at(&e.answer,p)),"field_token_accuracy":field_accuracy,"training_prompt_matches_generation":framed[0].tokens[..framed[0].response_start]==*prompt,
        "answer_tokenizer_roundtrip":l.tokenizer.decode(&gold[..gold.len()-1])?==e.answer}),
    )
}
fn field_at(answer: &str, byte: usize) -> &'static str {
    if byte >= answer.len() {
        return "eos";
    }
    if answer.find("[event:").is_some_and(|at| byte >= at) {
        return "citation";
    }
    if let Some((entity, context, _)) = fields(answer) {
        if byte < entity.len() {
            return "entity";
        }
        let context_start = entity.len() + "의 ".len();
        if (context_start..context_start + context.len()).contains(&byte) {
            return "context";
        }
        if byte >= context_start + context.len() + " 이동 지시는 ".len() {
            return "value";
        }
        return "format";
    }
    "text"
}
fn replay(
    fixture: &Path,
    path: &Path,
    output: &Path,
    panel: &str,
    source_id: Option<&str>,
) -> Result<()> {
    let f = load(fixture)?;
    let binary_hash = file_hash(&std::env::current_exe()?)?;
    let source_id = source_id
        .or_else(|| {
            (f.registry["binary_hash"] == binary_hash)
                .then(|| f.registry["source_id"].as_str())
                .flatten()
        })
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| {
            Error::Invalid("changed evaluation binary requires explicit --source-id".into())
        })?;
    let l = checkpoint::load(path, Device::Cpu, false)?;
    if l.tokenizer.semantic_id() != f.tokenizer {
        return Err(Error::Corrupt("replay tokenizer".into()));
    }
    let (_, _, validation) = data::load(&f.corpus)?;
    let cases = match panel {
        "watch" => &f.watch,
        "failures" => &f.failures,
        _ => &validation,
    };
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    let ledger = json!({"header":true,"fixture_hash":file_hash(fixture)?,"binary_hash":file_hash(&std::env::current_exe()?)?,"checkpoint_physical_hash":file_hash(path)?,"model_content_hash":l.model.weight_hash()?,"tokenizer_semantic_hash":l.tokenizer.semantic_id(),"prompt_format":neural::PROMPT_FORMAT,"split_hash":f.validation_hash,"ordered_ids_hash":digest(&cases.iter().map(|e|&e.id).collect::<Vec<_>>())?,"decoding":cases.iter().map(|e|&e.request.limits).collect::<Vec<_>>(),"metric":"strict-full-answer-eos-v1","panel":panel,"final_heldout":false});
    let mut ledger = ledger;
    ledger["source_commit"] = json!(source_commit()?);
    ledger["working_source_manifest_hash"] = json!(source_id);
    ledger["model_tensor_content_digest"] = json!(l.model.weights_content_id()?);
    writeln!(out, "{ledger}")?;
    let started = Instant::now();
    let mut rows = Vec::new();
    for e in cases {
        if started.elapsed().as_secs() > 900 {
            return Err(Error::Model(
                "replay command timeout; partial receipts preserved".into(),
            ));
        }
        let row = evaluate_one(&l, e, &e.request);
        writeln!(out, "{row}")?;
        out.flush()?;
        rows.push(row);
    }
    // A->B->A: each call uses a fresh actual model cache; gold never affects request IDs.
    if cases.len() > 1 {
        let a = evaluate_one(&l, &cases[0], &cases[0].request);
        if a["raw_tokens"] != rows[0]["raw_tokens"]
            || a["error"] != rows[0]["error"]
            || a["prompt_digest"] != rows[0]["prompt_digest"]
        {
            return Err(Error::Model("cache/request order regression".into()));
        }
    }
    let old = if l.model.weight_hash()? == f.registry["U2_POLICY_START"]["model_content_hash"] {
        Some(&f.previous_parent)
    } else if l.model.weight_hash()? == f.registry["U2_AFTER_250"]["model_content_hash"] {
        Some(&f.previous_failed)
    } else {
        None
    };
    let mut differences = 0;
    if let Some(old) = old {
        for row in &rows {
            let before = old
                .iter()
                .find(|r| r["id"] == row["id"])
                .ok_or_else(|| Error::Corrupt("missing old case".into()))?;
            differences += usize::from(
                before["actual"] != row["actual"]
                    || before["error"] != row["error"]
                    || before["exact_match"] != row["exact_match"],
            );
        }
    }
    let mut summary = summarize(&rows)?;
    summary["old_output_differences"] = json!(old.map(|_| differences));
    summary["aba_equal"] = json!(true);
    summary["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    writeln!(out, "{summary}")?;
    out.sync_all()?;
    println!("{summary}");
    if differences > 0 {
        return Err(Error::Model(
            "replay differs; no training authorized by this gate".into(),
        ));
    }
    Ok(())
}

// Independent semantic check: only serialized question, original records and status/time.
// It deliberately does not accept Episode labels, answers, categories or binding metadata.
fn support(request: &ModelRequest) -> Result<Vec<i64>> {
    let q = &request.input;
    let mut focus = q.as_str();
    for marker in ["말고 ", "아닌 ", "나오지만 ", "대신 "] {
        if let Some((_, right)) = focus.rsplit_once(marker) {
            focus = right;
        }
    }
    if q.contains("원인") || q.contains("인과관계") {
        let accident: Vec<_> = request
            .evidence
            .items
            .iter()
            .filter(|r| {
                r.original_excerpt.contains("사고가 기록")
                    && r.original_excerpt.contains("원인은 확인되지 않았다")
                    && r.original_excerpt
                        .strip_prefix("이후 ")
                        .and_then(|s| s.split_once("의 사고"))
                        .is_some_and(|(entity, _)| mentions_target(q, entity))
            })
            .collect();
        if accident.len() > 1 {
            return Err(Error::Invalid(
                "DATA_AMBIGUITY: multiple accident records".into(),
            ));
        }
        if q.contains("함께") || q.contains("불확실성을 설명") {
            return chronology_citations(request)
                .ok_or_else(|| Error::Invalid("DATA_AMBIGUITY: unsupported chronology".into()));
        }
        return Ok(accident.iter().map(|r| r.event_id).collect());
    }
    let earliest = focus.contains("최초") || focus.contains("처음");
    let past = earliest
        || ["과거", "이전", "정정 전", "예전에"]
            .iter()
            .any(|s| focus.contains(s));
    let focus_has_context = request
        .evidence
        .items
        .iter()
        .filter_map(|r| fields(&r.original_excerpt))
        .any(|(_, c, _)| mentions_target(focus, c));
    let mut selected = Vec::new();
    for r in &request.evidence.items {
        if let Some((entity, context, _)) = fields(&r.original_excerpt) {
            let context_mentioned =
                mentions_target(if focus_has_context { focus } else { q }, context);
            if mentions_target(q, entity)
                && context_mentioned
                && (earliest || r.version_status == if past { "superseded" } else { "current" })
                && !r.excerpt_truncated
            {
                selected.push(r);
            }
        }
    }
    if earliest && !selected.is_empty() {
        selected.sort_by_key(|r| r.recorded_at);
        if selected.len() > 1 && selected[0].recorded_at == selected[1].recorded_at {
            return Err(Error::Invalid(
                "DATA_AMBIGUITY: tied first record timestamp".into(),
            ));
        }
        selected.truncate(1);
    }
    if selected.len() != 1 {
        return Err(Error::Invalid(format!(
            "DATA_AMBIGUITY: independent question support count={}",
            selected.len()
        )));
    }
    Ok(vec![selected[0].event_id])
}
fn scan(episodes: &[Episode], l: &Loaded, limit: usize) -> Result<Value> {
    let mut prompts: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut collisions = Vec::new();
    let mut invalid = Vec::new();
    let mut prefix_mismatch = Vec::new();
    let mut categories: BTreeMap<usize, usize> = BTreeMap::new();
    let mut starts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut positions: BTreeMap<usize, usize> = BTreeMap::new();
    let mut lengths = BTreeMap::new();
    let mut digits = BTreeMap::new();
    let mut scenes = BTreeSet::new();
    let mut questions = BTreeSet::new();
    let mut question_forms = BTreeSet::new();
    let mut values = BTreeSet::new();
    let mut value_combinations = BTreeSet::new();
    let mut entity_digit_lengths = BTreeMap::new();
    let mut orders = BTreeSet::new();
    let (
        mut checked,
        mut auxiliary,
        mut no_evidence,
        mut max_prompt,
        mut over_window,
        mut target_count,
    ) = (0, 0, 0, 0, 0, 0);
    for e in episodes.iter().take(limit) {
        *categories.entry(e.category).or_default() += 1;
        let s = samples(std::slice::from_ref(e), &l.tokenizer, 512)?.remove(0);
        let p = l.tokenizer.prepare(
            &e.request,
            l.model.config.context as u32,
            &l.model.config.id()?,
        )?;
        max_prompt = max_prompt.max(p.token_ids.len());
        over_window += usize::from(p.token_ids.len() > 256);
        if s.tokens[..s.response_start] != p.token_ids {
            prefix_mismatch.push(e.id.clone());
        }
        let key = digest(&p.token_ids)?;
        if let Some((old_answer, old_id)) = prompts.get(&key) {
            if old_answer != &e.answer {
                collisions.push(json!({"a":old_id,"b":e.id,"a_answer":old_answer,"b_answer":e.answer,"prompt":key}));
            }
        } else {
            prompts.insert(key, (e.answer.clone(), e.id.clone()));
        }
        let target = &s.tokens[s.response_start..];
        target_count += target.len();
        *starts.entry(target[0]).or_default() += 1;
        *lengths.entry(target.len()).or_insert(0usize) += 1;
        scenes.insert(scene(e).to_string());
        questions.insert(e.request.input.clone());
        // Count surface forms separately from exact questions: each ASCII digit run
        // is replaced by one '#'. This is descriptive only, never model input.
        let mut form = String::new();
        let mut in_digits = false;
        for c in e.request.input.chars() {
            if !c.is_ascii_digit() {
                form.push(c);
            } else if !in_digits {
                form.push('#');
            }
            in_digits = c.is_ascii_digit();
        }
        question_forms.insert(form);
        orders.insert(digest(
            &e.request
                .evidence
                .items
                .iter()
                .map(|r| r.event_id)
                .collect::<Vec<_>>(),
        )?);
        let mut combination = Vec::new();
        for r in &e.request.evidence.items {
            if let Some((entity, _, v)) = fields(&r.original_excerpt) {
                values.insert(v.to_string());
                combination.push(v.to_string());
                *entity_digit_lengths
                    .entry(entity.chars().filter(char::is_ascii_digit).count())
                    .or_insert(0usize) += 1;
            }
            *digits.entry(r.event_id.to_string().len()).or_insert(0usize) += 1;
        }
        combination.sort();
        value_combinations.insert(combination);
        if e.family.starts_with("copy/") {
            auxiliary += 1;
            continue;
        }
        checked += 1;
        let support = match support(&e.request) {
            Ok(ids) => ids,
            Err(error) => {
                invalid.push(json!({"id":e.id,"error":error.to_string()}));
                continue;
            }
        };
        if support.is_empty() {
            no_evidence += 1;
        }
        let mut reversed = e.request.clone();
        reversed.evidence.items.reverse();
        if self::support(&reversed)? != support {
            return Err(Error::Corrupt(
                "support changes with evidence permutation".into(),
            ));
        }
        let mut supplied = citations(&e.answer)?;
        supplied.sort_unstable();
        let mut expected = support.clone();
        expected.sort_unstable();
        let wrong_value = support.len() == 1
            && e.request
                .evidence
                .items
                .iter()
                .find(|r| r.event_id == support[0])
                .and_then(|r| fields(&r.original_excerpt))
                .is_some_and(|(_, _, v)| {
                    !e.answer.contains(&format!("{v}이다."))
                        && !e.answer.starts_with(&format!("{v}입니다."))
                });
        // A chronology plus an uncertainty statement and the accident-only uncertainty
        // answer can both be supported. Do not mistake a richer answer for a data contradiction.
        let chronology_option = chronology_citations(&e.request);
        let compatible_chronology = chronology_option.as_ref().is_some_and(|ids| {
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            sorted == supplied
        });
        let wrong_fields = support.len() == 1
            && fields(&e.answer).is_some_and(|a| {
                e.request
                    .evidence
                    .items
                    .iter()
                    .find(|r| r.event_id == support[0])
                    .and_then(|r| fields(&r.original_excerpt))
                    .is_none_or(|r| r != a)
            });
        if (expected != supplied && !compatible_chronology) || wrong_value || wrong_fields {
            invalid.push(json!({"id":e.id,"error":"DATA_AMBIGUITY: independent support/target mismatch","expected_ids":support,"actual_ids":supplied}));
        }
        if let Some(id) = support.first()
            && let Some(position) = e
                .request
                .evidence
                .items
                .iter()
                .position(|r| r.event_id == *id)
        {
            *positions.entry(position).or_default() += 1;
        }
    }
    let mut report = json!({"scanned":episodes.len().min(limit),"total":episodes.len(),"independent_full_qa_checked":checked,"auxiliary_semantics_not_checked":auxiliary,"prompt_target_contradictions":collisions,"data_ambiguities":invalid,"train_generation_prefix_mismatches":prefix_mismatch,"category_counts":categories,"first_target_counts":starts,"target_length_histogram":lengths,"supervised_tokens_including_eos":target_count,"eos_targets":episodes.len().min(limit),"record_position":positions,"citation_digit_lengths":digits,"no_evidence":no_evidence,"unique_base_ids":scenes.len(),"unique_questions":questions.len(),"unique_values":values.len(),"unique_evidence_id_orders":orders.len(),"unique_token_prompts":prompts.len(),"max_prompt_length":max_prompt,"prompts_over_window256":over_window});
    report["unique_question_forms_ascii_digit_runs_collapsed"] = json!(question_forms.len());
    report["unique_evidence_value_multisets_including_empty"] = json!(value_combinations.len());
    report["entity_digit_lengths_per_structured_evidence"] = json!(entity_digit_lengths);
    Ok(report)
}
fn chronology_citations(request: &ModelRequest) -> Option<Vec<i64>> {
    if !request.input.contains("원인") || request.evidence.items.len() != 3 {
        return None;
    }
    let mut records: Vec<_> = request.evidence.items.iter().collect();
    records.sort_by_key(|r| r.recorded_at);
    let (entity, _, _) = fields(&records[0].original_excerpt)?;
    if !mentions_target(&request.input, entity)
        || !records
            .iter()
            .all(|r| mentions_target(&r.original_excerpt, entity))
    {
        return None;
    }
    if fields(&records[0].original_excerpt).is_none()
        || !records[1].original_excerpt.contains("지시를 실행했다")
        || !records[2]
            .original_excerpt
            .contains("원인은 확인되지 않았다")
        || records
            .windows(2)
            .any(|pair| pair[0].recorded_at >= pair[1].recorded_at)
    {
        return None;
    }
    Some(records.iter().map(|r| r.event_id).collect())
}
fn audit(fixture: &Path, output: &Path) -> Result<()> {
    let f = load(fixture)?;
    let l = checkpoint::load(&f.start, Device::Cpu, false)?;
    let (m, train, validation) = data::load(&f.corpus)?;
    let (pm, parent, pv) = data::load(&f.parent_corpus)?;
    if m.train.sha256 != f.train_hash
        || m.validation.sha256 != f.validation_hash
        || pm.train.sha256 != f.parent_train_hash
    {
        return Err(Error::Corrupt("audit corpus changed".into()));
    }
    let u2 = scan(&train, &l, train.len())?;
    let original = scan(&parent, &l, 2048)?;
    let entities = |cases: &[Episode]| -> BTreeSet<String> {
        cases
            .iter()
            .flat_map(|e| e.request.evidence.items.iter())
            .filter_map(|r| fields(&r.original_excerpt).map(|f| f.0.to_string()))
            .collect()
    };
    let mut overlap = entities(&train)
        .intersection(&entities(&validation))
        .cloned()
        .collect::<Vec<_>>();
    overlap.extend(entities(&parent).intersection(&entities(&pv)).cloned());
    let fail = !overlap.is_empty()
        || [&u2, &original].iter().any(|v| {
            [
                "prompt_target_contradictions",
                "data_ambiguities",
                "train_generation_prefix_mismatches",
            ]
            .iter()
            .any(|k| !v[*k].as_array().unwrap().is_empty())
        });
    let result = json!({"u2_all":u2,"parent_bounded_first2048":original,"cross_split_entity_overlap":overlap,"optimizer_updates":0,"status":if fail{"INTEGRITY_FAIL"}else{"CHECKED_BOUNDARIES_PASS"},"fixture_hash":file_hash(fixture)?});
    save(output, &result)?;
    println!(
        "audit status={} output={}",
        result["status"],
        output.display()
    );
    if fail {
        Err(Error::Invalid(
            "data/prefix integrity gate; inspect preserved audit".into(),
        ))
    } else {
        Ok(())
    }
}
fn compare(a: &Tensor, b: &Tensor) -> Result<Value> {
    if a.dims() != b.dims() {
        return Err(Error::Model("numeric shape mismatch".into()));
    }
    let av = a.flatten_all()?.to_vec1::<f32>()?;
    let bv = b.flatten_all()?.to_vec1::<f32>()?;
    let mut max_abs = 0f64;
    let mut violations = 0;
    for (&a, &b) in av.iter().zip(&bv) {
        let error = f64::from((a - b).abs());
        max_abs = max_abs.max(error);
        violations += usize::from(
            !a.is_finite()
                || !b.is_finite()
                || error > 1e-4 + 1e-3 * f64::from(a.abs().max(b.abs())),
        );
    }
    if violations > 0 {
        return Err(Error::Model(format!(
            "numeric parity violations={violations} max_abs={max_abs}"
        )));
    }
    Ok(json!({"max_abs":max_abs,"abs_tolerance":1e-4,"rel_tolerance":1e-3,"violations":0}))
}
fn numeric(fixture: &Path, path: &Path, output: &Path) -> Result<()> {
    let started = Instant::now();
    let f = load(fixture)?;
    let l = checkpoint::load(path, Device::Cpu, false)?;
    let before = l.model.weight_hash()?;
    let cases = [f.watch[0].clone(), f.watch[31].clone()];
    let framed = samples(&cases, &l.tokenizer, 512)?;
    let b = batch(&framed, &[0, 1], &Device::Cpu)?;
    let batched = l.model.forward(&b.input, Some(&b.valid))?;
    let mut checks = Vec::new();
    for (row, s) in framed.iter().enumerate() {
        let single = batch(&framed, &[row], &Device::Cpu)?;
        checks.push(json!({"case":cases[row].id,"alone_batch":compare(&l.model.forward(&single.input,Some(&single.valid))?,&batched.narrow(0,row,1)?.narrow(1,0,s.tokens.len()-1)?)?}));
        let prompt = &s.tokens[..s.response_start];
        for len in [prompt.len(), 255, 256, 257] {
            let ids: Vec<_> = prompt.iter().copied().cycle().take(len).collect();
            let direct = l.model.forward(
                &Tensor::new(ids.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                None,
            )?;
            for chunk in [127, 128, 129] {
                let mut cache = l.model.cache("recovery-parity");
                let mut parts = Vec::new();
                for part in ids.chunks(chunk) {
                    parts.push(l.model.forward_cached(
                        &Tensor::new(part, &Device::Cpu)?.unsqueeze(0)?,
                        &mut cache,
                        "recovery-parity",
                    )?);
                }
                let parity = compare(&direct, &Tensor::cat(&parts, 1)?)?;
                let mut extended = ids.clone();
                let next = s.tokens[s.response_start];
                extended.push(next);
                let full = l
                    .model
                    .forward(
                        &Tensor::new(extended.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                        None,
                    )?
                    .narrow(1, len, 1)?;
                let cached = l.model.forward_cached(
                    &Tensor::new(&[next], &Device::Cpu)?.unsqueeze(0)?,
                    &mut cache,
                    "recovery-parity",
                )?;
                checks.push(json!({"case":row,"length":len,"chunk":chunk,"prefill":parity,"same_prefix_next":compare(&full,&cached)?,"next_argmax_equal":full.argmax(2)?.to_vec2::<u32>()?==cached.argmax(2)?.to_vec2::<u32>()?}));
            }
            let mut changed = ids.clone();
            let at = len / 2;
            for id in &mut changed[at..] {
                *id = 8 + (*id + 1) % ((l.model.config.vocab - 8) as u32);
            }
            let modified = l.model.forward(
                &Tensor::new(changed.as_slice(), &Device::Cpu)?.unsqueeze(0)?,
                None,
            )?;
            checks.push(json!({"case":row,"length":len,"causal_future":compare(&direct.narrow(1,0,at)?,&modified.narrow(1,0,at)?)?}));
        }
    }
    // One real batch: labels/masks/EOS are independently derived from literal sample IDs.
    let (_, train, _) = data::load(&f.corpus)?;
    let s = samples(&train[..8], &l.tokenizer, 512)?;
    let b = batch(&s, &(0..8).collect::<Vec<_>>(), &Device::Cpu)?;
    let ids = b.input.to_vec2::<u32>()?;
    let target = b.target.to_vec2::<u32>()?;
    let mask = b.mask.to_vec2::<f32>()?;
    let mut count = 0;
    for (r, s) in s.iter().enumerate() {
        for p in 0..ids[r].len() {
            let valid = p + 1 < s.tokens.len();
            let m = valid && p + 1 >= s.response_start;
            if mask[r][p] != f32::from(m)
                || (valid && (ids[r][p] != s.tokens[p] || target[r][p] != s.tokens[p + 1]))
            {
                return Err(Error::Corrupt("actual shift/mask boundary".into()));
            }
            count += usize::from(m);
        }
        if s.tokens.last() != Some(&EOS) {
            return Err(Error::Corrupt("missing EOS supervision".into()));
        }
    }
    let logits = l.model.forward(&b.input, Some(&b.valid))?;
    let (ce, obj, n) = response_loss(&logits, &b, 8.)?;
    if n != count {
        return Err(Error::Corrupt("actual target denominator".into()));
    }
    let grad = obj.backward()?;
    let mut norms = BTreeMap::new();
    let mut finite_difference = Vec::new();
    for (name, var) in &l.model.vars {
        let g = grad
            .get(var)
            .ok_or_else(|| Error::Model(format!("missing gradient {name}")))?;
        let norm = g.sqr()?.sum_all()?.to_scalar::<f32>()?;
        if !norm.is_finite() {
            return Err(Error::Model("nonfinite actual gradient".into()));
        }
        norms.insert(name.clone(), norm.sqrt());
    }
    // Two selected coordinates, central difference on the actual objective; no optimizer update.
    for name in ["final_norm", "embedding"] {
        if let Some(var) = l.model.vars.get(name) {
            let g = grad.get(var).unwrap().flatten_all()?.to_vec1::<f32>()?;
            let at = g
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .unwrap()
                .0;
            let original = var.flatten_all()?.to_vec1::<f32>()?;
            let h = 0.002f32;
            let mut observed = Vec::new();
            for direction in [-1., 1.] {
                let mut values = original.clone();
                values[at] += h * direction;
                var.set(&Tensor::from_vec(values, var.dims(), &Device::Cpu)?)?;
                let result = response_loss(&l.model.forward(&b.input, Some(&b.valid))?, &b, 8.)?
                    .1
                    .to_scalar::<f32>();
                var.set(&Tensor::from_vec(
                    original.clone(),
                    var.dims(),
                    &Device::Cpu,
                )?)?;
                observed.push(result? as f64);
            }
            let reference = (observed[1] - observed[0]) / (2. * h as f64);
            let error = (reference - g[at] as f64).abs();
            finite_difference.push(json!({"parameter":name,"coordinate":at,"autograd":g[at],"central_difference":reference,"step":h,"absolute_error":error,"tolerance":"0.002 + 0.05 * max(abs(reference),abs(autograd))"}));
            if error > 0.002 + 0.05 * reference.abs().max(g[at].abs() as f64) {
                return Err(Error::Model("actual objective finite difference".into()));
            }
        }
    }
    if before != l.model.weight_hash()? {
        return Err(Error::Corrupt("numeric diagnosis mutated model".into()));
    }
    let result = json!({"model_content_hash":before,"checks":checks,"actual_batch_targets":n,"plain_ce":ce.to_scalar::<f32>()?,"weighted_objective":obj.to_scalar::<f32>()?,"gradient_norms":norms,"finite_difference":finite_difference,"optimizer_updates":0,"elapsed_seconds":started.elapsed().as_secs_f64(),"status":"CHECKED_BOUNDARIES_PASS"});
    save(output, &result)?;
    println!("numeric PASS output={}", output.display());
    Ok(())
}

fn arm_config(original: &TrainConfig, arm: &str) -> Result<TrainConfig> {
    let mut c = original.clone();
    if c.microbatch != 8
        || c.accumulation != 1
        || c.sample_group_size != 8
        || c.first_target_weight != 8.
    {
        return Err(Error::Invalid(
            "C/W requires the frozen U2 policy, no implicit policy changes".into(),
        ));
    }
    match arm {
        "C" => {}
        "W" => c.first_target_weight = 1.,
        _ => return Err(Error::Invalid("screening arm".into())),
    }
    Ok(c)
}
fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&neural::read_bounded(
        path,
        16 * 1024 * 1024,
    )?)?)
}
fn trace(path: &Path) -> Result<Vec<Value>> {
    neural::read_bounded(path, 16 * 1024 * 1024)?
        .split(|&b| b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| Ok(serde_json::from_slice(b)?))
        .collect()
}
fn worker_receipt(
    binary: &Path,
    checkpoint: &Path,
    request: &ModelRequest,
) -> Result<(bool, Value, String)> {
    use std::{
        io::Read,
        process::{Command, Stdio},
        time::Duration,
    };
    let mut child = Command::new(binary)
        .args(["__model-worker", "--checkpoint"])
        .arg(checkpoint)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((replica_v3::model::MAX_RESPONSE * 2) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .take(replica_v3::model::MAX_STDERR as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let write = replica_v3::model::write_frame(
        &mut child.stdin.take().unwrap(),
        request,
        replica_v3::model::MAX_REQUEST,
    );
    if write.is_err() {
        child.kill()?;
        let _ = child.wait();
        return Err(Error::Model("worker input write failed".into()));
    }
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed()
            > replica_v3::model::LOAD_TIMEOUT
                + Duration::from_millis(request.limits.timeout_ms + 1000)
        {
            child.kill()?;
            let _ = child.wait();
            return Err(Error::Model("worker diagnostic deadline".into()));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let bytes = out
        .join()
        .map_err(|_| Error::Model("worker reader panic".into()))??;
    let error = String::from_utf8(
        err.join()
            .map_err(|_| Error::Model("worker stderr reader panic".into()))??,
    )
    .map_err(|_| Error::Model("worker stderr encoding".into()))?;
    let mut cursor = std::io::Cursor::new(bytes);
    let ready: Value = replica_v3::model::read_frame(&mut cursor, replica_v3::model::MAX_RESPONSE)?;
    if ready["ready"] != true {
        return Err(Error::Model("native worker not ready".into()));
    }
    let response = if status.success() {
        replica_v3::model::read_frame(&mut cursor, replica_v3::model::MAX_RESPONSE)?
    } else {
        Value::Null
    };
    Ok((status.success(), response, error))
}
fn close(
    fixture: &Path,
    control: &Path,
    treatment: &Path,
    legacy: &Path,
    worker: &Path,
    source_id: &str,
    output: &Path,
) -> Result<()> {
    let f = load(fixture)?;
    let cp = read_json(&control.join("policy.json"))?;
    let wp = read_json(&treatment.join("policy.json"))?;
    let c = trace(&control.join("trace.jsonl"))?;
    let w = trace(&treatment.join("trace.jsonl"))?;
    let mut config = cp["config"].clone();
    config["first_target_weight"] = json!(1.);
    if config != wp["config"]
        || cp["parent"] != wp["parent"]
        || cp["tape"] != wp["tape"]
        || cp["source_id"] != wp["source_id"]
        || cp["binary_hash"] != wp["binary_hash"]
        || c.len() != 50
        || w.len() != 50
    {
        return Err(Error::Corrupt(
            "one-factor policy/tape budget mismatch".into(),
        ));
    }
    let keys = [
        "new_update",
        "cumulative_model_step",
        "optimizer_step",
        "schedule_step",
        "lr",
        "indices",
        "ids",
        "sampler_state",
        "input_tokens",
        "target_tokens",
    ];
    for (i, (a, b)) in c.iter().zip(&w).enumerate() {
        if keys.iter().any(|k| a[*k] != b[*k])
            || a["new_update"] != i + 1
            || a["first_target_weight"] != 8.
            || b["first_target_weight"] != 1.
        {
            return Err(Error::Corrupt("actual trace one-factor mismatch".into()));
        }
    }
    std::fs::create_dir(output)?;
    let tok = ByteBpe::load(legacy)?;
    let mut reports = Vec::new();
    let mut worker_checks = Vec::new();
    for (name, directory) in [("C", control), ("W", treatment)] {
        let checkpoint = directory.join("final");
        let loaded = checkpoint::load(&checkpoint, Device::Cpu, false)?;
        if loaded.tokenizer.semantic_id() != tok.semantic_id() {
            return Err(Error::Corrupt("legacy/native tokenizer mapping".into()));
        }
        let replay_path = output.join(format!("{name}-fresh.jsonl"));
        replay(fixture, &checkpoint, &replay_path, "watch", Some(source_id))?;
        let (_, actual) = rows(&replay_path)?;
        let scored = read_json(&directory.join("eval-050.json"))?;
        for row in &actual {
            let before = scored["watch_rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == row["id"])
                .ok_or_else(|| Error::Corrupt("fresh case missing".into()))?;
            if [
                "actual",
                "raw_tokens",
                "error",
                "prompt_digest",
                "exact_match",
            ]
            .iter()
            .any(|k| row[*k] != before[*k])
            {
                return Err(Error::Corrupt("fresh-process generation mismatch".into()));
            }
            let ids: Vec<u32> = serde_json::from_value(row["raw_tokens"].clone())?;
            let ids: Vec<_> = ids
                .into_iter()
                .take_while(|&id| id >= neural::SPECIALS as u32)
                .collect();
            if tok.decode_bytes(&ids)? != loaded.tokenizer.decode_bytes(&ids)? {
                return Err(Error::Corrupt("generated byte mapping mismatch".into()));
            }
        }
        let case = &f.watch[0];
        let direct = evaluate_one(&loaded, case, &case.request);
        worker_checks.push(check_worker(worker, &checkpoint, case, &direct)?);
        let mut stats = summarize(&actual)?;
        let mut fields: BTreeMap<String, [usize; 2]> = BTreeMap::new();
        for row in &actual {
            for key in [
                "entity",
                "context",
                "value",
                "citation_exact",
                "citation_in_provided",
            ] {
                if let Some(correct) = row["components"][key].as_bool() {
                    let n = fields.entry(key.into()).or_default();
                    n[0] += usize::from(correct);
                    n[1] += 1;
                }
            }
        }
        stats["components"] = json!(fields);
        reports.push(json!({"arm":name,"result":read_json(&directory.join("result.json"))?,"fresh_watch":stats,"physical_hash":file_hash(&checkpoint)?,"model_tensor_content_digest":loaded.model.weights_content_id()?,"legacy_architecture_weight_hash":loaded.model.weight_hash()?,"clock":loaded.manifest.training,"fresh_outputs_identical":true}));
    }
    let failed_path = PathBuf::from(
        f.registry["U2_AFTER_250"]["path"]
            .as_str()
            .ok_or_else(|| Error::Corrupt("failed path".into()))?,
    );
    let failed = checkpoint::load(&failed_path, Device::Cpu, false)?;
    for kind in ["utf8", "empty"] {
        let e = f
            .failures
            .iter()
            .find(|e| {
                f.previous_failed.iter().any(|r| {
                    r["id"] == e.id
                        && if kind == "utf8" {
                            r["error"].as_str().is_some_and(|e| e.contains("UTF-8"))
                        } else {
                            r["actual"] == ""
                        }
                })
            })
            .ok_or_else(|| Error::Corrupt("frozen failure coverage".into()))?;
        let direct = evaluate_one(&failed, e, &e.request);
        let ids: Vec<u32> = serde_json::from_value(direct["raw_tokens"].clone())?;
        let ids: Vec<_> = ids
            .into_iter()
            .take_while(|&id| id >= neural::SPECIALS as u32)
            .collect();
        if tok.decode_bytes(&ids)? != failed.tokenizer.decode_bytes(&ids)? {
            return Err(Error::Corrupt(
                "failed legacy/native mapping mismatch".into(),
            ));
        }
        worker_checks.push(check_worker(worker, &failed_path, e, &direct)?);
    }
    let base = read_json(&control.join("eval-000.json"))?["watch"]["exact_matches"]
        .as_u64()
        .ok_or_else(|| Error::Corrupt("baseline score".into()))?;
    let mut streak = 0;
    let mut regression = false;
    for n in [10, 25, 50] {
        let evaluation = read_json(&control.join(format!("eval-{n:03}.json")))?;
        let count = evaluation["watch"]["exact_matches"]
            .as_u64()
            .ok_or_else(|| Error::Corrupt("watch count".into()))?;
        let bad = base.saturating_sub(count) >= 4
            || evaluation["new_error_cases"].as_u64().unwrap_or(0) >= 2;
        streak = if bad { streak + 1 } else { 0 };
        regression |= streak >= 2;
    }
    let cs = &reports[0]["fresh_watch"];
    let ws = &reports[1]["fresh_watch"];
    let eligible = ws["exact_matches"]
        .as_u64()
        .is_some_and(|n| n >= base && n > cs["exact_matches"].as_u64().unwrap_or(0))
        && ws["generation_failures"].as_u64().unwrap_or(u64::MAX) == 0
        && ws["empty"].as_u64().unwrap_or(u64::MAX) == 0;
    let summary = json!({"one_factor_actual_trace_verified":true,"optimizer_updates_small":c.len()+w.len(),"same_sample_multiset_and_order":true,"same_clocks_and_lr":true,"reports":reports,"product_worker":worker_checks,"tokenizer_native_legacy_mapping":"PASS","candidate_eligible":eligible,"confirmation":if eligible{"REQUIRED_NOT_RUN"}else{"NOT_RUN_NO_SCREENING_EFFECT"},"regression_within_50":if regression{"REPRODUCED_ON_WATCH"}else{"NOT_REPRODUCED_WITHIN_BUDGET"},"s4_quality":"NOT_EVALUATED_HERE","goal1_ready":false});
    save(&output.join("summary.json"), &summary)?;
    println!(
        "closure report={} SMALL_updates={}",
        output.display(),
        c.len() + w.len()
    );
    Ok(())
}
fn check_worker(binary: &Path, path: &Path, e: &Episode, direct: &Value) -> Result<Value> {
    let (success, response, error) = worker_receipt(binary, path, &e.request)?;
    let expected_success = direct["error"].is_null()
        && direct["generation"]["finish"] == "stop"
        && direct["actual"].as_str().is_some_and(|s| !s.is_empty());
    if success != expected_success
        || (success
            && (response["text"] != direct["actual"]
                || response["provided"] != direct["provided"]
                || response["excluded"] != direct["excluded"]
                || response["prepared"]["token_digest"] != direct["native_prompt_digest"]))
    {
        return Err(Error::Model("product/direct generation differs".into()));
    }
    if !success {
        let expected_error = if direct["generation"]["finish"] == "length" {
            "token limit before EOS"
        } else if direct["error_class"] == "strict_utf8" {
            "invalid/incomplete output UTF-8"
        } else if direct["actual"] == "" {
            "empty native generation"
        } else {
            direct["error"].as_str().unwrap_or("UNKNOWN")
        };
        if !error.contains(expected_error) {
            return Err(Error::Model(
                "worker/direct error classification differs".into(),
            ));
        }
    }
    Ok(
        json!({"id":e.id,"success":success,"direct_worker_parity":true,"worker_error":error.trim(),"raw_bytes":direct["raw_bytes"],"raw_tokens":direct["raw_tokens"],"finish_reason":direct["finish_reason"]}),
    )
}
fn save_arm(
    l: &mut Loaded,
    state: &TrainingState,
    adam: &Adam,
    output: &Path,
    reason: &str,
) -> Result<()> {
    l.model.refresh_identity()?;
    let mut m = l.manifest.clone();
    m.training = Some(state.clone());
    // Native format/status vocabulary is frozen; the detailed diagnostic reason
    // belongs to result.json, not a new artifact schema or an invented success state.
    m.status = match reason {
        "RECOVERY_SCREENING" => "TRAINING",
        "CANCELLED" => "CANCELLED",
        "RESOURCE_LIMIT" => "RESOURCE_LIMIT",
        "SCREENING_BUDGET_REACHED" | "TOKEN_BUDGET" | "TIME_BUDGET" => "BUDGET_EXHAUSTED",
        "QUALITY_GUARD" | "INTEGRITY_FAIL" => "DIAGNOSTIC_COMPLETE",
        _ => return Err(Error::Invalid("unknown recovery termination".into())),
    }
    .into();
    m.diagnostic_only = true;
    checkpoint::save(output, &l.model, &l.tokenizer, m, &adam.moments)?;
    Ok(())
}
fn arm_run(fixture: &Path, arm: &str, source_id: &str, output: &Path) -> Result<()> {
    if source_id.len() != 64 || !source_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid("frozen source digest required".into()));
    }
    let f = load(fixture)?;
    if f.registry["U2_POLICY_START"]["backend"] != neural::cpu_backend() {
        return Err(Error::Invalid("frozen training backend mismatch".into()));
    }
    let mut l = checkpoint::load(&f.start, Device::Cpu, true)?;
    if l.model.weight_hash()? != f.registry["U2_POLICY_START"]["model_content_hash"]
        || file_hash(&f.start)? != f.registry["U2_POLICY_START"]["physical_hash"]
    {
        return Err(Error::Corrupt("arm parent changed".into()));
    }
    let (manifest, episodes, _) = data::load(&f.corpus)?;
    if manifest.train.sha256 != f.train_hash || manifest.validation.sha256 != f.validation_hash {
        return Err(Error::Corrupt("arm corpus changed".into()));
    }
    let mut state = l
        .manifest
        .training
        .clone()
        .ok_or_else(|| Error::Invalid("arm optimizer required".into()))?;
    let c = arm_config(&state.config, arm)?;
    let start_step = state.step;
    let start_input = state.consumed_tokens;
    let start_targets = state.target_tokens;
    state.config = c.clone();
    l.manifest.source_id = source_id.into();
    let s = samples(&episodes, &l.tokenizer, c.seq_len)?;
    let pool: Vec<_> = (0..s.len()).collect();
    let mut rng = neural::transformer::Rng {
        state: state.sampler_state,
    };
    let mut tape = Vec::new();
    for _ in 0..50 {
        let ids = draw_indices(&pool, &c, &mut rng)?;
        tape.push((ids, rng.state));
    }
    let mut seen = BTreeSet::new();
    let selected: Vec<usize> = tape
        .iter()
        .flat_map(|(ids, _)| ids.iter())
        .copied()
        .filter(|i| seen.insert(*i))
        .take(16)
        .collect();
    let mut adam = Adam {
        moments: std::mem::take(&mut l.optimizer),
    };
    std::fs::create_dir(output)?;
    save(
        &output.join("policy.json"),
        &json!({"arm":arm,"parent":f.registry["U2_POLICY_START"],"fixture_hash":file_hash(fixture)?,"source_id":source_id,"binary_hash":file_hash(&std::env::current_exe()?)?,"tape":tape,"tape_hash":digest(&tape)?,"config":c,"max_new_updates":50,"max_input_tokens":200000,"max_target_tokens":50000,"max_seconds":900,"max_rss_bytes":17179869184u64,"clock_policy":"moments + cumulative optimizer clock retained; saved U2 schedule unchanged"}),
    )?;
    let mut log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("trace.jsonl"))?;
    let started = Instant::now();
    let cancelled = std::sync::Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed))
        .map_err(|e| Error::Model(e.to_string()))?;
    let mut base_score = 0;
    let mut base_errors = BTreeSet::new();
    let mut bad_streak = 0;
    let mut reason = "SCREENING_BUDGET_REACHED";
    let mut exposure = BTreeMap::<usize, usize>::new();
    let outcome = (|| -> Result<()> {
        for (n, entry) in tape
            .iter()
            .map(Some)
            .chain(std::iter::once(None))
            .enumerate()
        {
            if [0, 10, 25, 50].contains(&n) {
                l.manifest.training = Some(state.clone());
                l.model.refresh_identity()?;
                let rows: Vec<_> = f
                    .watch
                    .iter()
                    .map(|e| evaluate_one(&l, e, &e.request))
                    .collect();
                let score = summarize(&rows)?;
                let errors: BTreeSet<_> = rows
                    .iter()
                    .filter(|r| !r["error"].is_null() || r["actual"] == "")
                    .map(|r| r["id"].as_str().unwrap().to_string())
                    .collect();
                let count = score["exact_matches"].as_u64().unwrap() as usize;
                if n == 0 {
                    base_score = count;
                    base_errors = errors.clone();
                }
                let new_errors = errors.difference(&base_errors).count();
                let bad = base_score.saturating_sub(count) >= 4 || new_errors >= 2;
                bad_streak = if bad { bad_streak + 1 } else { 0 };
                let train_rows: Vec<_> = selected
                    .iter()
                    .map(|&i| evaluate_one(&l, &episodes[i], &episodes[i].request))
                    .collect();
                let evaluation = json!({"new_updates":n,"model_step":state.step,"watch":score,"new_error_cases":new_errors,"bad_streak":bad_streak,"watch_rows":rows,"train_exposure_panel":train_rows,"exposure_counts":selected.iter().map(|i|(episodes[*i].id.clone(),exposure.get(i).copied().unwrap_or(0))).collect::<BTreeMap<_,_>>()});
                save(&output.join(format!("eval-{n:03}.json")), &evaluation)?;
                println!(
                    "arm={arm} new_updates={n}/50 cumulative_step={} watch={count}/32 new_errors={new_errors} elapsed_s={:.3}",
                    state.step,
                    started.elapsed().as_secs_f64()
                );
                save_arm(
                    &mut l,
                    &state,
                    &adam,
                    &output.join(format!("step-{n:03}")),
                    "RECOVERY_SCREENING",
                )?;
                if bad_streak >= 2 {
                    reason = "QUALITY_GUARD";
                    break;
                }
            }
            let Some((indices, sampler)) = entry else {
                break;
            };
            if cancelled.load(Ordering::Relaxed) {
                reason = "CANCELLED";
                break;
            }
            if started.elapsed().as_secs() >= 900 {
                reason = "TIME_BUDGET";
                break;
            }
            if rss_kib()? > 16 * 1024 * 1024 {
                reason = "RESOURCE_LIMIT";
                break;
            }
            let b = batch(&s, indices, &Device::Cpu)?;
            let target_count: usize = indices
                .iter()
                .map(|&i| s[i].tokens.len() - s[i].response_start)
                .sum();
            if state.consumed_tokens - start_input + b.tokens as u64 > 200000
                || state.target_tokens - start_targets + target_count as u64 > 50000
            {
                reason = "TOKEN_BUDGET";
                break;
            }
            let (ce, obj, targets) = response_loss(
                &l.model.forward(&b.input, Some(&b.valid))?,
                &b,
                c.first_target_weight,
            )?;
            let ce = ce.to_scalar::<f32>()?;
            let objective = obj.to_scalar::<f32>()?;
            if !ce.is_finite() || !objective.is_finite() {
                return Err(Error::Model("nonfinite recovery loss".into()));
            }
            let grads = obj.backward()?;
            // Preserve the production accumulation arithmetic even for accumulation=1.
            let gradients = l
                .model
                .vars
                .iter()
                .map(|(name, var)| -> Result<_> {
                    let g = grads
                        .get(var)
                        .ok_or_else(|| Error::Model(format!("missing {name}")))?;
                    Ok((
                        name.clone(),
                        ((g * targets as f64)?.detach() / targets as f64)?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            let mut groups: BTreeMap<String, [f64; 3]> = BTreeMap::new();
            let inspect = [1, 5, 20].contains(&(n + 1));
            let (norm, delta) = adam.step_observed(
                &l.model.vars,
                &gradients,
                &c,
                state.step + 1,
                |name, g, old, next| {
                    if inspect {
                        let group = if name == "embedding" {
                            "embedding_tied_output"
                        } else if name.ends_with("q_norm") || name.ends_with("k_norm") {
                            "qk_norm"
                        } else if name.ends_with("gate")
                            || name.ends_with("up")
                            || name.ends_with("down")
                        {
                            "ffn"
                        } else if name.ends_with("norm") {
                            "norm"
                        } else {
                            "attention"
                        };
                        let a = groups.entry(group.into()).or_default();
                        a[0] += g.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
                        a[1] += (next - old)?.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
                        a[2] += old.sqr()?.sum_all()?.to_scalar::<f32>()? as f64;
                    }
                    Ok(())
                },
            )?;
            state.step += 1;
            state.consumed_tokens += b.tokens as u64;
            state.target_tokens += targets as u64;
            state.sampler_state = *sampler;
            state.train_loss = Some(ce as f64);
            state.validation_loss = None;
            for &i in indices {
                *exposure.entry(i).or_default() += 1;
            }
            let stats: BTreeMap<_,_>=groups.into_iter().map(|(k,v)|(k,json!({"gradient_norm":v[0].sqrt(),"update_norm":v[1].sqrt(),"weight_norm":v[2].sqrt(),"update_to_weight":v[1].sqrt()/v[2].sqrt().max(1e-30)}))).collect();
            let row = json!({"arm":arm,"new_update":n+1,"cumulative_model_step":state.step,"optimizer_step":state.step,"schedule_step":state.step-c.budget_start_step,"lr":c.learning_rate(state.step),"indices":indices,"ids":indices.iter().map(|&i|&episodes[i].id).collect::<Vec<_>>(),"sampler_state":sampler,"input_tokens":b.tokens,"target_tokens":targets,"ce":ce,"objective":objective,"first_target_weight":c.first_target_weight,"grad_norm":norm,"update_norm":delta,"parameter_groups":stats,"elapsed_seconds":started.elapsed().as_secs_f64(),"rss_kib":rss_kib()?});
            writeln!(log, "{row}")?;
            log.flush()?;
            if n + 1 == 20 && arm == "C" {
                let hash = l.model.weight_hash()?;
                let expected = &f.registry["U2_AFTER_20"]["model_content_hash"];
                save(
                    &output.join("control-step20-parity.json"),
                    &json!({"actual":hash,"recorded_u2_after20":expected,"equal":hash==*expected}),
                )?;
                if hash != *expected {
                    return Err(Error::Model(
                        "control differs from historical 20-update replay".into(),
                    ));
                }
            }
        }
        Ok(())
    })();
    if outcome.is_err() {
        reason = "INTEGRITY_FAIL";
    }
    save_arm(&mut l, &state, &adam, &output.join("final"), reason)?;
    let result = json!({"arm":arm,"reason":reason,"new_updates":state.step-start_step,"cumulative_model_step":state.step,"additional_input_tokens":state.consumed_tokens-start_input,"additional_target_tokens":state.target_tokens-start_targets,"model_content_hash":l.model.weight_hash()?,"elapsed_seconds":started.elapsed().as_secs_f64(),"error":outcome.as_ref().err().map(ToString::to_string),"goal1_ready":false});
    save(&output.join("result.json"), &result)?;
    log.sync_all()?;
    println!("{result}");
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_generation_never_uses_gold_or_gold_length() {
        let tok = ByteBpe::train(&[b"abc".to_vec()], &neural::hash(b"fixture"), 264).unwrap();
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 17, Device::Cpu).unwrap();
        let manifest = checkpoint::initialized(&model, &tok, 17, neural::hash(b"fixture")).unwrap();
        let loaded = Loaded {
            model,
            tokenizer: tok,
            manifest,
            optimizer: BTreeMap::new(),
        };
        let request = ModelRequest {
            request_id: "gold-independence".into(),
            system: String::new(),
            input: "abc".into(),
            evidence: Default::default(),
            limits: replica_v3::event::GenerationLimits {
                context_tokens: 64,
                max_tokens: 8,
                timeout_ms: 30000,
            },
        };
        let mut e = Episode {
            id: "one/0".into(),
            category: 0,
            family: "fixture".into(),
            binding: String::new(),
            sequence: String::new(),
            request,
            answer: "a".into(),
        };
        let a = evaluate_one(&loaded, &e, &e.request);
        e.answer = "b".repeat(512); // Deliberately cannot fit teacher forcing; generation is unaffected.
        let b = evaluate_one(&loaded, &e, &e.request);
        assert!(
            a["raw_tokens"]
                .as_array()
                .is_some_and(|ids| !ids.is_empty())
        );
        for key in [
            "request_digest",
            "prompt_digest",
            "raw_tokens",
            "finish_reason",
            "actual",
            "error",
        ] {
            assert_eq!(a[key], b[key], "{key}");
        }
        assert!(b["teacher_forced_diagnostic_after_generation"]["error"].is_string());
    }
    fn independent_request() -> ModelRequest {
        use replica_v3::retrieval::{Evidence, EvidenceBundle};
        let record = |id, entity: &str, context: &str, value: &str, status: &str| Evidence {
            event_id: id,
            original_excerpt: format!("{entity}의 {context} 이동 지시는 {value}이다."),
            version_status: status.into(),
            recorded_at: id,
            observed_at: None,
            source: "fixture".into(),
            retrieval_reason: "fixture".into(),
            relation_path: vec![],
            excerpt_truncated: false,
        };
        ModelRequest {
            request_id: "test".into(),
            system: String::new(),
            input: "센서31의 구역1 현재 방향은?".into(),
            limits: Default::default(),
            evidence: EvidenceBundle {
                items: vec![
                    record(19, "센서310", "구역1", "왼쪽", "current"),
                    record(7, "센서31", "구역1", "북쪽", "superseded"),
                    record(23, "센서31", "구역1", "동쪽", "current"),
                    record(5, "센서31", "구역10", "남쪽", "current"),
                ],
                ..Default::default()
            },
        }
    }
    #[test]
    fn recovery_support_uses_question_boundaries_status_and_not_record_position() {
        let mut r = independent_request();
        assert_eq!(support(&r).unwrap(), vec![23]);
        r.evidence.items.reverse();
        assert_eq!(support(&r).unwrap(), vec![23]);
        r.input = "센서31 구역1의 현재 말고 처음 기록한 방향은?".into();
        assert_eq!(support(&r).unwrap(), vec![7]);
        let mut tied = r.clone();
        tied.evidence
            .items
            .iter_mut()
            .find(|e| e.event_id == 23)
            .unwrap()
            .recorded_at = 7;
        assert!(support(&tied).is_err());
        r.input = "센서31의 구역10 말고 구역1 방향을 알려줘.".into();
        assert_eq!(support(&r).unwrap(), vec![23]);
        r.evidence.items.push(
            r.evidence
                .items
                .iter()
                .find(|e| e.event_id == 23)
                .unwrap()
                .clone(),
        );
        assert!(support(&r).is_err());
    }
    #[test]
    fn recovery_causal_richer_answer_is_supported_not_a_conflicting_fact() {
        let mut r = independent_request();
        r.input = "센서31의 시간 순서만으로 사고 원인을 알 수 있나?".into();
        r.evidence.items.truncate(3);
        r.evidence.items[0].original_excerpt = "센서31의 구역1 이동 지시는 동쪽이다.".into();
        for (i, e) in r.evidence.items.iter_mut().enumerate() {
            e.recorded_at = i as i64;
        }
        r.evidence.items[1].original_excerpt = "센서31는 동쪽 지시를 실행했다.".into();
        r.evidence.items[2].original_excerpt =
            "이후 센서31의 사고가 기록되었다. 원인은 확인되지 않았다.".into();
        assert_eq!(support(&r).unwrap(), vec![23]);
        assert_eq!(chronology_citations(&r), Some(vec![19, 7, 23]));
        r.evidence.items.reverse();
        assert_eq!(chronology_citations(&r), Some(vec![19, 7, 23]));
        r.evidence.items[2].original_excerpt = "센서310의 구역1 이동 지시는 동쪽이다.".into();
        assert!(chronology_citations(&r).is_none());
    }
    #[test]
    fn recovery_one_factor_preserves_tape_and_both_existing_clocks() {
        let base = TrainConfig {
            first_target_weight: 8.,
            microbatch: 8,
            sample_group_size: 8,
            accumulation: 1,
            budget_start_step: 19750,
            max_steps: 20750,
            ..Default::default()
        };
        let a = arm_config(&base, "C").unwrap();
        let b = arm_config(&base, "W").unwrap();
        let mut av = serde_json::to_value(&a).unwrap();
        let bv = serde_json::to_value(&b).unwrap();
        av["first_target_weight"] = json!(1.);
        assert_eq!(av, bv);
        let (mut r1, mut r2) = (Rng::new(73), Rng::new(73));
        let pool: Vec<_> = (0..64).collect();
        for step in 19751..=19800 {
            assert_eq!(
                draw_indices(&pool, &a, &mut r1).unwrap(),
                draw_indices(&pool, &b, &mut r2).unwrap()
            );
            assert_eq!(a.learning_rate(step), b.learning_rate(step));
        }
        assert_eq!(r1.state, r2.state);
    }
    #[test]
    fn recovery_unequal_target_accumulation_matches_combined_batch() {
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 91, Device::Cpu).unwrap();
        let samples = vec![
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, 12, 13, 14, 15, EOS],
                response_start: 1,
                curriculum: false,
            },
        ];
        let combined = batch(&samples, &[0, 1], &Device::Cpu).unwrap();
        let (_, obj, n) = response_loss(
            &model
                .forward(&combined.input, Some(&combined.valid))
                .unwrap(),
            &combined,
            8.,
        )
        .unwrap();
        assert_eq!(n, 8);
        let reference = obj.backward().unwrap();
        let mut summed: BTreeMap<String, Tensor> = BTreeMap::new();
        for i in 0..2 {
            let b = batch(&samples, &[i], &Device::Cpu).unwrap();
            let (_, obj, n) =
                response_loss(&model.forward(&b.input, Some(&b.valid)).unwrap(), &b, 8.).unwrap();
            let g = obj.backward().unwrap();
            for (name, v) in &model.vars {
                let term = (g.get(v).unwrap() * n as f64).unwrap();
                let next = match summed.remove(name) {
                    Some(old) => (old + term).unwrap(),
                    None => term,
                };
                summed.insert(name.clone(), next);
            }
        }
        for (name, v) in &model.vars {
            compare(reference.get(v).unwrap(), &(&summed[name] / 8.).unwrap()).unwrap();
        }
    }
    #[test]
    fn recovery_ledger_counts_decode_errors_and_rejects_duplicates() {
        let good = json!({"id":"a","family":"qa/test","category":0,"actual":"ok","expected":"ok","error":null,"generation":{"finish":"stop","generated":2},"exact_match":true});
        let bad = json!({"id":"b","family":"copy/test","category":0,"actual":null,"expected":"x","error":"invalid UTF-8","generation":{"finish":"stop","generated":3},"exact_match":false});
        let s = summarize(&[good.clone(), bad]).unwrap();
        assert_eq!(s["denominator"], 2);
        assert_eq!(s["qa"], json!([1, 1]));
        assert_eq!(s["auxiliary"], json!([0, 1]));
        assert_eq!(s["invalid_utf8"], 1);
        assert!(summarize(&[good.clone(), good]).is_err());
    }
    #[test]
    fn recovery_raw_bytes_classify_utf8_without_lossy_repair() {
        let t = ByteBpe::train(&[b"abc".to_vec()], &neural::hash(b"fixture"), 264).unwrap();
        let incomplete = t.encode(&[0xea, 0xb0]).unwrap();
        let invalid = t.encode(&[0xea, 0x20]).unwrap();
        assert_eq!(
            bytes_receipt(&t, &incomplete)["utf8_error_class"],
            "incomplete_tail"
        );
        assert_eq!(bytes_receipt(&t, &incomplete)["valid_up_to"], 0);
        assert_eq!(
            bytes_receipt(&t, &invalid)["utf8_error_class"],
            "invalid_sequence"
        );
        assert_eq!(
            bytes_receipt(&t, &t.encode("가".as_bytes()).unwrap())["utf8_valid"],
            true
        );
        assert!(t.decode(&incomplete).is_err());
    }
    #[test]
    fn recovery_observer_sees_control_without_changing_product_error() {
        let model =
            Transformer::init(neural::transformer::Config::tiny(264), 17, Device::Cpu).unwrap();
        for var in model.vars.values() {
            var.set(&Tensor::zeros(var.dims(), DType::F32, &Device::Cpu).unwrap())
                .unwrap();
        }
        let mut raw = Vec::new();
        let actual =
            model.generate_observed(&[BOS, 8], 4, 30000, &AtomicBool::new(false), "x", |id| {
                raw.push(id)
            });
        let product = model.generate(&[BOS, 8], 4, 30000, &AtomicBool::new(false), "x");
        assert_eq!(raw, vec![PAD]);
        assert_eq!(
            actual.unwrap_err().to_string(),
            product.unwrap_err().to_string()
        );
    }
    #[test]
    fn recovery_adam_three_steps_match_f64_clip_moments_and_clocks() {
        let vars = BTreeMap::from([("x".into(), Var::new(&[1f32, -2.], &Device::Cpu).unwrap())]);
        let mut adam = Adam::new(&vars).unwrap();
        let c = TrainConfig {
            warmup: 2,
            max_steps: 10,
            clip: 1.,
            ..Default::default()
        };
        let (mut weights, mut m, mut v) = ([1f64, -2.], [0f64; 2], [0f64; 2]);
        for (i, g) in [[3f32, 4.], [-2., 1.], [0.5, -0.25]].iter().enumerate() {
            let step = i + 1;
            let norm = (g.iter().map(|v| (*v as f64).powi(2)).sum::<f64>()).sqrt();
            let clip = (1. / (norm + 1e-12)).min(1.);
            for j in 0..2 {
                let g = g[j] as f64 * clip;
                m[j] = 0.9 * m[j] + 0.1 * g;
                v[j] = 0.999 * v[j] + 0.001 * g * g;
                let rate = if step <= 2 {
                    0.001 * step as f64 / 2.
                } else {
                    0.001
                        * (0.1
                            + 0.45 * (1. + (std::f64::consts::PI * (step - 2) as f64 / 8.).cos()))
                };
                weights[j] = weights[j] * (1. - rate * 0.01)
                    - rate * (m[j] / (1. - 0.9f64.powi(step as i32)))
                        / ((v[j] / (1. - 0.999f64.powi(step as i32))).sqrt() + 1e-8);
            }
            adam.step(
                &vars,
                &BTreeMap::from([("x".into(), Tensor::new(g, &Device::Cpu).unwrap())]),
                &c,
                step,
            )
            .unwrap();
            for (actual, expected) in vars["x"].to_vec1::<f32>().unwrap().iter().zip(weights) {
                assert!((*actual as f64 - expected).abs() < 1e-6);
            }
            for (name, expected) in [("adam.m.x", m), ("adam.v.x", v)] {
                for (a, b) in adam.moments[name]
                    .to_vec1::<f32>()
                    .unwrap()
                    .iter()
                    .zip(expected)
                {
                    assert!((*a as f64 - b).abs() < 1e-6);
                }
            }
        }
        let continued = TrainConfig {
            budget_start_step: 100,
            max_steps: 110,
            ..c
        };
        assert_eq!(continued.learning_rate(101), 0.0005);
        assert_eq!(continued.learning_rate(102), 0.001);
    }
}
