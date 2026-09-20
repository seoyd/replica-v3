//! Bounded, training-only data audit and identifiable baseline preparation.
//! Uses the existing native corpus, request resolver, publisher and trainer plan.
use super::*;
use std::collections::BTreeSet;
const DATASET: &str = "joint-binding-balanced-v1";
const CONTRACT: &str = "R3-IDENTIFIABLE-BASELINE-1.0";

fn bump(map: &mut BTreeMap<String, usize>, key: impl ToString) {
    *map.entry(key.to_string()).or_default() += 1;
}
fn number(s: &str) -> Result<u64> {
    s.trim_start_matches("구역")
        .parse()
        .map_err(|_| bad("numeric context"))
}
fn summary(values: &[usize]) -> [usize; 3] {
    if values.is_empty() {
        return [0; 3];
    }
    let mut v = values.to_vec();
    v.sort_unstable();
    [v[0], v[v.len() / 2], *v.last().unwrap()]
}
// Scene identity deliberately omits values, wording, order and selected side.
// It retains record/entity/context/time/status; exact evidence has a separate digest.
fn scene(e: &Episode) -> Result<String> {
    let mut rows = vec![];
    for r in &e.request.evidence.items {
        let (entity, context, _) = parsed_record(r)?;
        rows.push((
            r.event_id,
            entity,
            context,
            r.observed_at,
            &r.version_status,
        ));
    }
    rows.sort();
    digest(&rows)
}
fn rule_choice<'a>(items: &'a [Evidence], rule: usize) -> Result<Option<&'a Evidence>> {
    if items.is_empty() {
        return Ok(None);
    }
    if rule == 0 {
        return Ok(items.first());
    }
    if rule == 1 {
        return Ok(items.last());
    }
    let mut keys = vec![];
    for (i, r) in items.iter().enumerate() {
        let (entity, context, _) = parsed_record(r)?;
        let value = match rule {
            2 | 3 => number(context)? as i64,
            4 | 5 => entity.chars().count() as i64,
            6 | 7 => r.event_id,
            8 => r.observed_at.unwrap_or(r.recorded_at),
            _ => return Err(bad("audit rule")),
        };
        keys.push((value, i));
    }
    let key = if [2, 4, 6].contains(&rule) {
        keys.iter().map(|x| x.0).min()
    } else {
        keys.iter().map(|x| x.0).max()
    }
    .unwrap();
    // A tied rule abstains; never breaks ties using the label or question.
    let mut matches = keys.iter().filter(|x| x.0 == key);
    let index = matches.next().unwrap().1;
    Ok(matches.next().is_none().then_some(&items[index]))
}
fn positions(e: &Episode, tok: &ByteBpe, sample: &Sample) -> Result<binary::Value> {
    let ids = &sample.tokens[..sample.response_start];
    let user = ids
        .iter()
        .position(|&x| x == neural::USER_ROLE)
        .ok_or_else(|| bad("missing user role"))?
        + 1;
    let end = ids[user..]
        .iter()
        .position(|&x| x == neural::END_ROLE)
        .ok_or_else(|| bad("user end"))?
        + user;
    let mut records = vec![];
    for start in ids
        .iter()
        .enumerate()
        .filter_map(|(i, &x)| (x == neural::EVIDENCE_ROLE).then_some(i + 1))
    {
        let finish = ids[start..]
            .iter()
            .position(|&x| x == neural::END_ROLE)
            .ok_or_else(|| bad("record end"))?
            + start;
        let r = &e.request.evidence.items[records.len()];
        let (_, _, value) = parsed_record(r)?;
        // Match byte spans against the actual decoded token prefixes, not an
        // independently tokenized substring which could have different BPE merges.
        let bytes = tok.decode_bytes(&ids[start..finish])?;
        let mut offsets = vec![0];
        for &id in &ids[start..finish] {
            offsets.push(offsets.last().unwrap() + tok.decode_bytes(&[id])?.len());
        }
        let locate = |needle: &[u8]| -> Result<Vec<[usize; 2]>> {
            Ok(bytes
                .windows(needle.len())
                .enumerate()
                .filter_map(|(i, w)| {
                    if w != needle {
                        return None;
                    }
                    let left = offsets.iter().rposition(|&x| x <= i)?;
                    let right = offsets.iter().position(|&x| x >= i + needle.len())?;
                    Some([start + left, start + right])
                })
                .collect())
        };
        records.push(binary::record!({"event":r.event_id,"tokens":[start,finish],"value_spans":locate(value.as_bytes())?,"citation_source_spans":locate(r.event_id.to_string().as_bytes())?}));
    }
    let length = sample.tokens.len() - 1; // teacher input excludes the final EOS target
    let lost = length.saturating_sub(256);
    Ok(
        binary::record!({"id":e.id,"question":[user,end],"records":records,"prompt":sample.response_start,"input":length,"target":sample.tokens.len()-sample.response_start,"local256_removed_edges_per_layer":lost*(lost+1)/2}),
    )
}
fn audit_set(
    es: &[Episode],
    ms: &[Meta],
    tok: &ByteBpe,
    counts: Option<&[usize]>,
    location: &Path,
) -> Result<binary::Value> {
    if es.len() != ms.len() || counts.is_some_and(|v| v.len() != es.len()) {
        return Err(bad("audit metadata/exposure shape"));
    }
    let mut scenes = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    let mut bindings = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut values = BTreeSet::new();
    let mut assignments = BTreeSet::new();
    let mut prompts: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut questions = BTreeMap::new();
    let mut lengths = vec![];
    let mut rows = vec![];
    let mut mismatches = 0;
    let mut missing = 0;
    let mut distributions = BTreeMap::new();
    let mut rules: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    let mut exposed_rules: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    let mut exposed_bases = BTreeSet::new();
    let mut exposed_bindings = BTreeSet::new();
    let mut bucket_exposures: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let rule_names = [
        "first",
        "last",
        "lower_context",
        "higher_context",
        "shorter_entity",
        "longer_entity",
        "lower_event",
        "higher_event",
        "max_observed",
    ];
    let mut d_legacy = [0usize; 2];
    let mut removed = 0usize;
    for (index, ((e, m), s)) in es.iter().zip(ms).zip(samples(es, tok, 512)?).enumerate() {
        let exposure = counts.map_or(0, |c| c[index]);
        if counts.is_some() {
            bucket_exposures.entry(m.bucket).or_default().push(exposure);
        }
        if exposure > 0 {
            exposed_bases.insert(&m.base);
            exposed_bindings.insert(digest(&(&e.request.input, &e.request.evidence, &e.answer))?);
        }
        mismatches += usize::from(resolve(&e.request)? != e.answer);
        let mut q = e.request.clone();
        q.limits.context_tokens = 512;
        q.limits.max_tokens = (tok.encode(e.answer.as_bytes())?.len() + 1) as u32;
        let p = tok.prepare(&q, 512, "audit")?;
        missing += usize::from(
            p.provided.len() != e.request.evidence.items.len() || !p.excluded.is_empty(),
        );
        let generation = tok.prepare(&e.request, 2048, "audit")?;
        if p.token_ids != generation.token_ids {
            bump(&mut distributions, "train_generation_prompt_difference");
        }
        if s.tokens.len() > 512 {
            bump(&mut distributions, "over_512");
        }
        scenes.insert(scene(e)?);
        evidence.insert(digest(&e.request.evidence)?);
        bindings.insert(digest(&(
            &e.request.input,
            &e.request.evidence,
            e.answer.as_str(),
        ))?);
        prompts
            .entry(p.token_digest)
            .or_default()
            .insert(e.answer.clone());
        bump(&mut questions, &e.request.input);
        let (qe, qc, _) = question_intent(&e.request.input)?;
        let mut assignment = vec![];
        for r in &e.request.evidence.items {
            let (entity, context, value) = parsed_record(r)?;
            names.insert(entity.to_string());
            values.insert(value.to_string());
            let digits = entity.trim_start_matches("장치");
            bump(
                &mut distributions,
                format!(
                    "bucket{}/leading_zero/{}",
                    m.bucket,
                    digits.starts_with('0')
                ),
            );
            bump(
                &mut distributions,
                format!(
                    "bucket{}/repeated_digits/{}",
                    m.bucket,
                    digits.bytes().all(|n| Some(n) == digits.bytes().next())
                ),
            );
            assignment.push((r.event_id, value));
            bump(
                &mut distributions,
                format!(
                    "bucket{}/name_digits/{}",
                    m.bucket,
                    entity.trim_start_matches("장치").len()
                ),
            );
            bump(
                &mut distributions,
                format!(
                    "bucket{}/context_range/{}",
                    m.bucket,
                    number(context)? / 1000
                ),
            );
        }
        assignment.sort();
        assignments.insert(digest(&assignment)?);
        bump(
            &mut distributions,
            format!("bucket{}/view{}", m.bucket, m.view),
        );
        bump(
            &mut distributions,
            format!(
                "bucket{}/wording/{}",
                m.bucket,
                e.request.input.splitn(3, ' ').nth(2).unwrap_or("")
            ),
        );
        let gold = citations(&e.answer)?;
        if m.bucket == 3 {
            d_legacy[1] += 1;
            d_legacy[0] += usize::from(
                number(qc)? < 1000
                    && e.request.evidence.items.iter().any(|r| {
                        parsed_record(r).is_ok_and(|(a, c, _)| {
                            a == qe && c != qc && number(c).is_ok_and(|n| n >= 1000)
                        })
                    }),
            );
        }
        if m.bucket < 6 && gold.len() == 1 {
            for (r, name) in rule_names.iter().enumerate() {
                let v = rules
                    .entry(format!("bucket{}/view{}/{name}", m.bucket, m.view))
                    .or_default();
                v[0] += 1;
                let weighted = exposed_rules
                    .entry(format!("bucket{}/view{}/{name}", m.bucket, m.view))
                    .or_default();
                weighted[0] += exposure;
                if let Some(pick) = rule_choice(&e.request.evidence.items, r)? {
                    v[1] += 1;
                    v[2] += usize::from(pick.event_id == gold[0]);
                    weighted[1] += exposure;
                    weighted[2] += exposure * usize::from(pick.event_id == gold[0]);
                    let candidate = if m.bucket == 0 {
                        format!("{} [event:{}]", pick.original_excerpt, pick.event_id)
                    } else {
                        format!(
                            "{}입니다. [event:{}]",
                            parsed_record(pick)?.2,
                            pick.event_id
                        )
                    };
                    v[3] += usize::from(candidate == e.answer);
                    weighted[3] += exposure * usize::from(candidate == e.answer);
                }
            }
            if let Some(slot) = e
                .request
                .evidence
                .items
                .iter()
                .position(|r| r.event_id == gold[0])
            {
                bump(
                    &mut distributions,
                    format!("bucket{}/selected_slot/{slot}", m.bucket),
                );
            }
        }
        let pos = positions(e, tok, &s)?;
        removed += pos["local256_removed_edges_per_layer"].as_u64().unwrap() as usize;
        lengths.push(s.tokens.len());
        rows.push(pos);
    }
    replica_v3::codec::publish_new(location, |f, _| {
        for row in &rows {
            binary::write_value_record(f, row)?;
        }
        Ok(())
    })?;
    let conflicts = prompts.values().filter(|s| s.len() > 1).count();
    let result = binary::record!({"rows":es.len(),"semantic_bases":ms.iter().map(|m|&m.base).collect::<BTreeSet<_>>().len(),"scenes_without_values":scenes.len(),"evidence_tuples":evidence.len(),"query_bindings":bindings.len(),"value_assignments":assignments.len(),"full_ids":names,"unique_values":values.len(),"unique_questions":questions.len(),"repeated_question_rows":es.len()-questions.len(),"prompt_conflicts":conflicts,"resolver_disagreements":mismatches,"evidence_excluded":missing,"sequence_min_median_max":summary(&lengths),"removed_edges_per_local_layer":removed,"context_legacy_pattern_numerator_denominator":d_legacy,"distributions":distributions,"rules_applicable_decided_event_full":rules,"exposure_min_median_max":counts.map(summary),"exposed_rows":counts.map(|c|c.iter().filter(|&&n|n>0).count()),"executed_draws":counts.map(|c|c.iter().sum::<usize>()),"label_integrity":if conflicts==0&&mismatches==0&&missing==0 {"VERIFIED"}else{"FAILED"},"token_positions_hash":file_hash(location)?});
    let mut result = result;
    result["rules_weighted_by_actual_exposure"] = binary::record!(exposed_rules);
    result["exposed_bases"] = binary::record!(counts.map(|_| exposed_bases.len()));
    result["exposed_bindings"] = binary::record!(counts.map(|_| exposed_bindings.len()));
    result["exposure_by_bucket_min_median_max"] = binary::record!(
        bucket_exposures
            .iter()
            .map(|(b, c)| (b.to_string(), summary(c)))
            .collect::<BTreeMap<_, _>>()
    );
    result["active_mask_cases"] = binary::record!(lengths.iter().filter(|&&n| n - 1 > 256).count());
    Ok(result)
}
pub(super) fn audit(roots: &[PathBuf], diagnostics: &[PathBuf], output: &Path) -> Result<()> {
    std::fs::create_dir(output)?;
    let mut results = vec![];
    for (index, root) in roots.iter().enumerate() {
        let p: Plan = read(&root.join("plan.r3b"))?;
        if file_hash(&root.join("initial.r3m"))? != p.initial {
            return Err(bad("audit initial native binding"));
        }
        let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
        let tok = ByteBpe::load(&root.join("tokenizer.r3b"))?;
        if tok.id() != p.tokenizer {
            return Err(bad("audit tokenizer binding"));
        }
        let (tm, dm, xm) = verified_metadata(root, &p)?;
        let mut es = c.train.clone();
        let mut ms = tm.clone();
        if let Some(f) = &p.fork {
            if let Some(h) = &f.variants {
                es.extend(verified_corpus(&root.join("variants.r3cor"), h)?.train);
                ms.extend(tm.clone());
            }
            if let Some(h) = &f.selector {
                es.extend(verified_corpus(&root.join("selectors.r3cor"), h)?.train);
                let sm: Vec<Meta> = read(&root.join("selector-metadata.r3b"))?;
                if f.selector_metadata.as_ref()
                    != Some(&file_hash(&root.join("selector-metadata.r3b"))?)
                {
                    return Err(bad("audit selector metadata"));
                }
                ms.extend(sm);
            }
        }
        if let Some(h) = &p.training_values {
            es = verified_corpus(&root.join("training-values.r3cor"), h)?.train;
        }
        let mut counts = vec![0; es.len()];
        let mut updates = 0;
        let mut trace_hashes = vec![];
        for n in 0..128 {
            let start = root.join(format!("segment-{n:04}-started.r3b"));
            if !start.exists() {
                break;
            }
            let path = root.join(format!("segment-{n:04}/updates.r3rows"));
            if !path.exists() {
                continue;
            }
            for row in binary::read_value_records(&path)? {
                let step = row["step"]
                    .as_u64()
                    .ok_or_else(|| bad("audit update clock"))? as usize;
                let actual: Vec<usize> = binary::from_value(row["sample_indices"].clone())?;
                if actual != p.training_draw(step - 1) {
                    return Err(bad("actual draws differ from frozen policy"));
                }
                for i in actual {
                    *counts
                        .get_mut(i)
                        .ok_or_else(|| bad("audit sample bounds"))? += 1;
                }
                updates += 1;
            }
            trace_hashes.push(file_hash(&path)?);
        }
        let x = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?;
        let mut panels = BTreeMap::new();
        for (name, episodes, meta, exposure) in [
            (
                "train",
                es.as_slice(),
                ms.as_slice(),
                Some(counts.as_slice()),
            ),
            ("primary", c.validation.as_slice(), dm.as_slice(), None),
            ("transfer", x.validation.as_slice(), xm.as_slice(), None),
        ] {
            let report = audit_set(
                episodes,
                meta,
                &tok,
                exposure,
                &output.join(format!("{index}-{name}-positions.r3rows")),
            )?;
            println!(
                "IDENTIFIABLE_AUDIT root={} panel={name} rows={} scenes={} bindings={} conflicts={} label_errors={} legacy_D={} length={} edges={} updates={updates}",
                root.display(),
                report["rows"],
                report["scenes_without_values"],
                report["query_bindings"],
                report["prompt_conflicts"],
                report["resolver_disagreements"],
                report["context_legacy_pattern_numerator_denominator"],
                report["sequence_min_median_max"],
                report["removed_edges_per_local_layer"]
            );
            panels.insert(name, report);
        }
        let (selector, sm) = selector_panel(&c.validation, &dm, p.tiny)?;
        panels.insert(
            "selector",
            audit_set(
                &selector,
                &sm,
                &tok,
                None,
                &output.join(format!("{index}-selector-positions.r3rows")),
            )?,
        );
        results.push(binary::record!({"root":root,"policy":file_hash(&root.join("plan.r3b"))?,"source":p.source,"corpus":p.corpus,"training_values":p.training_values,"tape":p.train_order,"actual_updates_this_fork":updates,"traces":trace_hashes,"panels":panels}));
    }
    for (i, path) in diagnostics.iter().enumerate() {
        let tok = ByteBpe::load(
            &roots
                .first()
                .ok_or_else(|| bad("audit tokenizer root"))?
                .join("tokenizer.r3b"),
        )?;
        let (es, ms): (Vec<Episode>, Vec<Meta>) = read(path)?;
        let start: binary::Value = read(
            &path
                .parent()
                .ok_or_else(|| bad("diagnostic parent"))?
                .join("started.r3b"),
        )?;
        if start["cases"] != digest(&es)? {
            return Err(bad("historical diagnostic case binding"));
        }
        let report = audit_set(
            &es,
            &ms,
            &tok,
            None,
            &output.join(format!("diagnostic-{i}-positions.r3rows")),
        )?;
        results.push(binary::record!({"diagnostic_cases":path,"physical":file_hash(path)?,"start":start,"report":report,"exposure":"NOT_A_TRAINING_RUN"}));
    }
    publish_confirmed(
        &output.join("audit.r3b"),
        &binary::record!({"contract":CONTRACT,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"results":results,"optimizer":0,"generation":0,"teacher":0,"evidence":"DERIVED_EXISTING_RAW"}),
    )?;
    Ok(())
}

fn stream(seed: u64, field: &str) -> Rng {
    let h = neural::hash(format!("{seed}/{field}").as_bytes());
    Rng::new(u64::from_str_radix(&h[..16], 16).unwrap())
}
fn shuffle<T>(values: &mut [T], rng: &mut Rng) {
    for i in (1..values.len()).rev() {
        let j = rng.next_u64() as usize % (i + 1);
        values.swap(i, j);
    }
}
fn balanced_factor(n: usize, levels: usize, seed: u64, label: &str) -> Vec<usize> {
    let mut values = (0..n).map(|i| i % levels).collect::<Vec<_>>();
    shuffle(&mut values, &mut stream(seed, label));
    values
}
fn different_pair(rng: &mut Rng, capacity: u64) -> Result<[u64; 2]> {
    if capacity < 2 {
        return Err(bad("finite field namespace exhausted"));
    }
    let a = rng.next_u64() % capacity;
    let b = rng.next_u64() % (capacity - 1);
    Ok([a, if b >= a { b + 1 } else { b }])
}
fn generate_balanced(bases: usize, split: usize, seed: u64) -> Result<(Vec<Episode>, Vec<Meta>)> {
    if bases == 0 || !bases.is_multiple_of(8) || split > 2 {
        return Err(bad("balanced base count/split"));
    }
    let mut out = vec![];
    let mut meta = vec![];
    // Split the finite context-key namespace before constructing families.
    // Both sides use the same pool; full key/value bindings cannot cross splits.
    let context_pool = (0..2000)
        .filter(|n| {
            let h = neural::hash(format!("{DATASET}/context/{n}").as_bytes());
            u64::from_str_radix(&h[..16], 16).unwrap() % 3 == split as u64
        })
        .collect::<Vec<_>>();
    let low_contexts = context_pool
        .iter()
        .copied()
        .filter(|&n| n < 1000)
        .collect::<Vec<_>>();
    let high_contexts = context_pool
        .iter()
        .copied()
        .filter(|&n| n >= 1000)
        .collect::<Vec<_>>();
    for bucket in 0..8 {
        let group_seed = seed
            ^ (split as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15)
            ^ (bucket as u64 + 1).wrapping_mul(0x517cc1b727220a95);
        let lengths = balanced_factor(bases, 8, group_seed, "entity-length");
        let value_lengths = balanced_factor(bases, 8, group_seed, "value-length");
        let types = balanced_factor(bases, 2, group_seed, "value-type");
        let orders = balanced_factor(bases, 2, group_seed, "physical-order");
        let event_sides = balanced_factor(bases, 2, group_seed, "event-rank");
        let contexts = balanced_factor(bases, 2, group_seed, "context-rank");
        let subtypes = balanced_factor(bases, 3, group_seed, "no-evidence-kind");
        let mut names = stream(group_seed, "entity-digits");
        let mut ctx = stream(group_seed, "context-digits");
        let mut vals = stream(group_seed, "value-digits");
        let mut events = stream(group_seed, "event-digits");
        let mut times = stream(group_seed, "observed-time");
        let mut recorded = stream(group_seed, "recorded-time");
        for base in 0..bases {
            let width = lengths[base] + 1;
            let ns = different_pair(&mut names, 10u64.pow(width as u32))?;
            let entities = ns.map(|n| format!("장치{n:0width$}"));
            // Stratified draws from the same context-key namespace. Assign
            // lower/higher to either side in equal counts before shuffling.
            let mut cs = [
                low_contexts[ctx.next_u64() as usize % low_contexts.len()],
                high_contexts[ctx.next_u64() as usize % high_contexts.len()],
            ];
            if contexts[base] == 1 {
                cs.reverse();
            }
            let context = cs.map(|n| format!("구역{n}"));
            let pair = if types[base] == 0 {
                let v = different_pair(&mut vals, 4)?;
                v.map(|i| ["왼쪽", "오른쪽", "직진", "대기"][i as usize].to_owned())
            } else {
                let width = value_lengths[base] + 1;
                different_pair(&mut vals, 10u64.pow(width as u32))?.map(|n| format!("{n:0width$}"))
            };
            // Event magnitudes and timestamps use different streams; no adjacent-ID rule.
            let mut ids = different_pair(&mut events, 99_999_999)?.map(|n| n as i64 + 1);
            ids.sort();
            if event_sides[base] == 1 {
                ids.reverse();
            }
            let time = 1000 + (times.next_u64() % 100000) as i64;
            let rt = different_pair(&mut recorded, 100000)?.map(|n| n as i64 + time + 100);
            let family = format!("{DATASET}/split{split}/bucket{bucket}/base{base}");
            let mut records = vec![record(
                &entities[0],
                &context[0],
                &pair[0],
                ids[0],
                time,
                "current",
            )];
            match bucket {
                0 | 1 => {}
                2 => records.push(record(
                    &entities[1],
                    &context[0],
                    &pair[1],
                    ids[1],
                    time,
                    "current",
                )),
                3 => records.push(record(
                    &entities[0],
                    &context[1],
                    &pair[1],
                    ids[1],
                    time,
                    "current",
                )),
                4 | 7 => records.push(record(
                    &entities[0],
                    &context[0],
                    &pair[1],
                    ids[1],
                    time + 1,
                    "current",
                )),
                5 => {
                    records[0].version_status = "superseded".into();
                    records.push(record(
                        &entities[0],
                        &context[0],
                        &pair[1],
                        ids[1],
                        time + 1,
                        "current",
                    ));
                }
                6 => match subtypes[base] {
                    0 => records.clear(),
                    1 => {
                        records[0] =
                            record(&entities[1], &context[0], &pair[0], ids[0], time, "current")
                    }
                    _ => records.push(record(
                        &entities[0],
                        &context[0],
                        &pair[1],
                        ids[1],
                        time,
                        "current",
                    )),
                },
                _ => unreachable!(),
            }
            for (i, r) in records.iter_mut().enumerate() {
                r.recorded_at = rt[i];
            }
            for view in 0..2 {
                let mut items = records.clone();
                let entity = if bucket == 6 && subtypes[base] == 1 && view == 1 {
                    &entities[1]
                } else if bucket == 2 {
                    &entities[view]
                } else {
                    &entities[0]
                };
                let context_query = if bucket == 3 {
                    &context[view]
                } else {
                    &context[0]
                };
                if (bucket == 4 || bucket == 7) && view == 1 {
                    let a = items[0].observed_at;
                    items[0].observed_at = items[1].observed_at;
                    items[1].observed_at = a;
                }
                if bucket == 6 && view == 1 {
                    match subtypes[base] {
                        0 => {
                            let mut r = record(entity, context_query, &pair[0], ids[0], time, "current");
                            r.recorded_at = rt[0];
                            items.push(r);
                        }
                        1 => {} // Query now names the otherwise unchanged supplied record.
                        _ => items[0].observed_at = Some(time + 1),
                    }
                }
                if bucket < 2 && view == 1 {
                    items[0].original_excerpt =
                        record(entity, context_query, &pair[1], ids[0], time, "current")
                            .original_excerpt;
                }
                if orders[base] == 1 {
                    items.reverse();
                }
                let intent = match bucket {
                    0 => Intent::Full,
                    5 if view == 0 => Intent::Previous,
                    5 => Intent::Restored,
                    7 => Intent::Cause,
                    _ => Intent::Current,
                };
                let id = format!("{family}/view{view}");
                let request = ModelRequest {
                    request_id: id.clone(),
                    system: SYSTEM.into(),
                    input: format!("{entity} {context_query} {}", familiar(intent)),
                    evidence: EvidenceBundle {
                        items,
                        ..Default::default()
                    },
                    limits: GenerationLimits {
                        context_tokens: 2048,
                        max_tokens: 128,
                        timeout_ms: 120000,
                    },
                };
                // Generator selects the answer algebraically; resolver independently
                // consumes only the serialized request during validation below.
                let selected = match bucket {
                    2 | 3 => view,
                    4 => 1 - view,
                    5 => view,
                    _ => 0,
                };
                let value = if bucket < 2 {
                    &pair[view]
                } else {
                    &pair[selected]
                };
                let answer = match bucket {
                    0 => format!(
                        "{} [event:{}]",
                        request.evidence.items[0].original_excerpt, ids[0]
                    ),
                    6 if view == 0 => [
                        "근거가 없습니다.",
                        "요청한 대상의 근거가 없습니다.",
                        "근거가 모호하여 확정할 수 없습니다.",
                    ][subtypes[base]]
                        .into(),
                    7 => "시간순서만으로 원인은 확정되지 않습니다.".into(),
                    _ => format!("{value}입니다. [event:{}]", ids[selected]),
                };
                out.push(Episode {
                    id: id.clone(),
                    category: [3, 3, 0, 2, 1, 1, 4, 4][bucket],
                    family: family.clone(),
                    binding: format!("{}/{}/{}", entities[0], context[0], ids[0]),
                    sequence: family.clone(),
                    request,
                    answer,
                });
                meta.push(Meta {
                    id,
                    base: family.clone(),
                    template: format!("{intent:?}/familiar"),
                    bucket,
                    view,
                    split: ["train", "primary", "transfer"][split].into(),
                    entities: entities.to_vec(),
                    source_id: None,
                    query_context: Some(context_query.clone()),
                });
            }
        }
    }
    validate_balanced(&out, &meta)?;
    Ok((out, meta))
}
fn validate_balanced(es: &[Episode], ms: &[Meta]) -> Result<()> {
    if es.len() != ms.len() || !es.len().is_multiple_of(2) {
        return Err(bad("balanced shape"));
    }
    data::validate_episodes(es)?;
    for (pair, meta) in es.chunks_exact(2).zip(ms.chunks_exact(2)) {
        let [a, b] = [&pair[0], &pair[1]];
        let bucket = meta[0].bucket;
        if meta[0].base != meta[1].base
            || meta[0].split != meta[1].split
            || meta[0].view != 0
            || meta[1].view != 1
            || meta[1].bucket != bucket
        {
            return Err(bad("complementary family/split"));
        }
        for e in pair {
            if e.request
                .evidence
                .items
                .iter()
                .any(|r| r.observed_at.is_none_or(|t| t > r.recorded_at))
            {
                return Err(bad("observed/recorded time contract"));
            }
            if resolve(&e.request)? != e.answer {
                return Err(bad("request-only resolver disagrees"));
            }
        }
        if a.request.input == b.request.input && a.request.evidence == b.request.evidence {
            return Err(bad("complementary views have identical model inputs"));
        }
        if (2..=4).contains(&bucket) {
            if a.answer == b.answer || citations(&a.answer)? == citations(&b.answer)? {
                return Err(bad("same-target complementary pair"));
            }
            if bucket < 4 && a.request.evidence != b.request.evidence {
                return Err(bad("query flip changed evidence"));
            }
            if bucket == 4 {
                if a.request.input != b.request.input {
                    return Err(bad("time exchange changed query"));
                }
                let mut flipped = a.request.evidence.clone();
                let time = flipped.items[0].observed_at;
                flipped.items[0].observed_at = flipped.items[1].observed_at;
                flipped.items[1].observed_at = time;
                if flipped != b.request.evidence {
                    return Err(bad("time exchange changed other fields"));
                }
            }
        }
    }
    Ok(())
}
fn validate_splits(sets: &[(&[Episode], &[Meta])]) -> Result<()> {
    let mut family_owner = BTreeMap::new();
    let mut scene_owner = BTreeMap::new();
    let mut binding_owner = BTreeMap::new();
    for (split, (es, ms)) in sets.iter().enumerate() {
        validate_balanced(es, ms)?;
        for (e, m) in es.iter().zip(*ms) {
            if family_owner
                .insert(&m.base, split)
                .is_some_and(|old| old != split)
            {
                return Err(bad("family split leakage"));
            }
            if !e.request.evidence.items.is_empty()
                && scene_owner
                    .insert(scene(e)?, split)
                    .is_some_and(|old| old != split)
            {
                return Err(bad("scene split leakage"));
            }
            for r in &e.request.evidence.items {
                let key = parsed_record(r)?;
                if binding_owner
                    .insert(
                        (key.0.to_owned(), key.1.to_owned(), key.2.to_owned()),
                        split,
                    )
                    .is_some_and(|old| old != split)
                {
                    return Err(bad("full key/value binding split leakage"));
                }
            }
        }
    }
    Ok(())
}
fn new_corpus(train: Vec<Episode>, dev: Vec<Episode>, seed: u64) -> Result<data::native::Corpus> {
    let manifest=data::CorpusManifest{version:1,scope:"project-owned synthetic record QA".into(),permission:"synthetic project-owned".into(),generator:DATASET.into(),seed,split_rule:"family assigned before fields; independent streams; complementary views stay together; request-only labels; disjoint scenes and full key/value bindings".into(),train:data::native::split("train",&train),validation:data::native::split("validation",&dev)};
    data::native::from_episodes(manifest, train, dev)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Policy {
    study: PathBuf,
    arm: String,
    dataset: String,
    pub(super) rows: Vec<[usize; 8]>,
}
fn config() -> TrainConfig {
    TrainConfig {
        lr: 3e-4,
        warmup: 128,
        max_steps: 4096,
        max_tokens: 20_000_000,
        microbatch: 8,
        accumulation: 1,
        validate_every: 512,
        seed: 29,
        ..Default::default()
    }
}
pub(super) fn evaluation() -> EvaluationPolicy {
    EvaluationPolicy {
        screen_steps: vec![0, 256, 2048],
        primary_steps: vec![512, 1024, 4096],
        transfer_steps: vec![512, 1024, 4096],
        train_steps: vec![0, 512, 1024, 4096],
        teacher_steps: vec![],
        generation_limit: 6400,
        teacher_limit: 6400,
        active_seconds: 14400,
        ..Default::default()
    }
}
fn tape(ms: &[Meta], steps: usize) -> Result<Vec<[usize; 8]>> {
    let mut orders = vec![];
    for b in 0..8 {
        let pairs = ms
            .chunks_exact(2)
            .enumerate()
            .filter_map(|(i, m)| (m[0].bucket == b).then_some(2 * i))
            .collect::<Vec<_>>();
        if pairs.len() != 512 {
            return Err(bad("balanced training base count"));
        }
        orders.push(pairs);
    }
    let mut rows = vec![[0; 8]; steps];
    for epoch in 0..steps.div_ceil(1024) {
        for (bucket, bases) in orders.iter().enumerate() {
            let mut bases = bases.clone();
            shuffle(
                &mut bases,
                &mut stream(29 ^ epoch as u64, &format!("tape/{bucket}")),
            );
            let sides = balanced_factor(
                bases.len(),
                2,
                29 ^ epoch as u64,
                &format!("tape-side/{bucket}"),
            );
            for (i, &base) in bases.iter().enumerate() {
                for view in 0..2 {
                    let step = epoch * 1024 + i * 2 + view;
                    if step < steps {
                        rows[step][bucket] = base + (view ^ sides[i]);
                    }
                }
            }
        }
    }
    Ok(rows)
}
fn cross_tables(es: &[Episode], ms: &[Meta]) -> Result<BTreeMap<String, [usize; 2]>> {
    let mut table: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    for (e, m) in es
        .iter()
        .zip(ms)
        .filter(|(_, m)| (2..=4).contains(&m.bucket))
    {
        let gold = citations(&e.answer)?[0];
        let records = &e.request.evidence.items;
        let slot = records
            .iter()
            .position(|r| r.event_id == gold)
            .ok_or_else(|| bad("selected event"))?;
        let selected = &records[slot];
        let other = &records[1 - slot];
        let (name, context, value) = parsed_record(selected)?;
        for factor in [
            format!("slot/{slot}"),
            format!("context_range/{}", number(context)? / 1000),
            format!(
                "context_rank/{:?}",
                number(context)?.cmp(&number(parsed_record(other)?.1)?)
            ),
            format!("entity_digits/{}", name.trim_start_matches("장치").len()),
            format!("value_type/{}", value.bytes().all(|b| b.is_ascii_digit())),
            format!("event_rank/{}", selected.event_id > other.event_id),
            format!("template/{}", m.template),
        ] {
            table
                .entry(format!("bucket{}/{factor}", m.bucket))
                .or_default()[m.view] += 1;
        }
    }
    Ok(table)
}
fn prepare_model(local: &Transformer, global: bool) -> Result<Transformer> {
    let mut c = local.config.clone();
    if global {
        c.local_layers = 0;
        c.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
    }
    Transformer::from_tensors(
        c,
        local
            .vars
            .iter()
            .map(|(name, var)| (name.clone(), var.as_detached_tensor()))
            .collect(),
        Device::Cpu,
    )
}
pub(super) fn verify_plan(root: &Path, p: &Plan) -> Result<()> {
    let own = p
        .identifiable
        .as_ref()
        .ok_or_else(|| bad("balanced policy absent"))?;
    if own.dataset != DATASET
        || !["LOCAL5", "GLOBAL6"].contains(&own.arm.as_str())
        || p.fork.is_some()
        || p.paired.is_some()
        || p.training_values.is_some()
        || p.grounding.is_some()
        || p.config != config()
        || p.tiny
        || p.model_seed != 17
        || p.data_seed != 20260921
        || root != own.study.join(&own.arm)
    {
        return Err(bad("balanced policy scope"));
    }
    let (tm, _, _) = verified_metadata(root, p)?;
    let order: Vec<Vec<usize>> = (0..8)
        .map(|b| {
            tm.iter()
                .enumerate()
                .filter_map(|(i, m)| (m.bucket == b).then_some(i))
                .collect()
        })
        .collect();
    if own.rows != tape(&tm, 4096)? || p.order != order {
        return Err(bad("balanced tape mismatch"));
    }
    let mut c = Config::small(p.architecture.vocab);
    if own.arm == "GLOBAL6" {
        c.local_layers = 0;
        c.profile = "NATIVE_TRPP_EXPERIMENTAL_V1".into();
    }
    if p.architecture != c {
        return Err(bad("only local_layers treatment permitted"));
    }
    Ok(())
}
pub(super) fn prepare(parent: &Path, output: &Path) -> Result<()> {
    if cfg!(feature = "test-support") {
        return Err(bad("research requires production binary"));
    }
    let parent_plan: Plan = read(&parent.join("plan.r3b"))?;
    let tok = ByteBpe::load(&parent.join("tokenizer.r3b"))?;
    if tok.id() != parent_plan.tokenizer {
        return Err(bad("preserved tokenizer policy mismatch"));
    }
    std::fs::create_dir(output)?;
    let seed = 20260921;
    let (train, tm) = generate_balanced(512, 0, seed)?;
    let (dev, dm) = generate_balanced(32, 1, seed)?;
    let (transfer, xm) = generate_balanced(8, 2, seed)?;
    validate_splits(&[(&train, &tm), (&dev, &dm), (&transfer, &xm)])?;
    let corpus = new_corpus(train.clone(), dev.clone(), seed)?;
    let tx = new_corpus(train.clone(), transfer.clone(), seed)?;
    let mut reports = BTreeMap::new();
    for (name, es, ms) in [
        ("train", &train, &tm),
        ("primary", &dev, &dm),
        ("transfer", &transfer, &xm),
    ] {
        let report = audit_set(
            es,
            ms,
            &tok,
            None,
            &output.join(format!("{name}-positions.r3rows")),
        )?;
        if report["label_integrity"] != "VERIFIED"
            || report["sequence_min_median_max"][2]
                .as_u64()
                .is_none_or(|n| n > 512)
            || !report["distributions"]["train_generation_prompt_difference"].is_null()
        {
            return Err(bad("balanced full input/labels"));
        }
        println!(
            "BALANCED_DATA split={name} rows={} length={} masked_edges={}",
            report["rows"],
            report["sequence_min_median_max"],
            report["removed_edges_per_local_layer"]
        );
        let table = cross_tables(es, ms)?;
        if table.values().any(|n| n[0] != n[1]) {
            return Err(bad("declared marginal query-side balance"));
        }
        reports.insert(
            name,
            binary::record!({"audit":report,"query_side_cross_table":table}),
        );
    }
    let active = reports["train"]["audit"]["removed_edges_per_local_layer"]
        .as_u64()
        .unwrap()
        > 0;
    let local = Transformer::init(Config::small(tok.vocab_size()), 17, Device::Cpu)?;
    let global = prepare_model(&local, true)?;
    if local.weights_content_id()? != global.weights_content_id()?
        || local.config.semantic_id()? == global.config.semantic_id()?
    {
        return Err(bad("initial tensor/architecture identity"));
    }
    let adam = Adam::new(&local.vars)?;
    for tensor in adam.moments.values() {
        if tensor
            .flatten_all()?
            .to_vec1::<f32>()?
            .iter()
            .any(|&x| x.to_bits() != 0)
        {
            return Err(bad("Adam not fresh zero"));
        }
    }
    let rows = tape(&tm, 4096)?;
    let mut inputs = vec![];
    let mut targets = vec![];
    let framed = samples(&train, &tok, 512)?;
    write(
        &output.join("numerical-started.r3b"),
        &binary::record!({"source":source_digest()?,"planned_forward":8,"planned_backward":2,"optimizer":0,"generation":0,"teacher":0}),
    )?;
    let numerical = verify_models(&local, &global, &framed)?;
    publish_confirmed(&output.join("numerical-result.r3b"), &numerical)?;
    for row in &rows {
        inputs.push(
            row.iter()
                .map(|&i| framed[i].tokens.len() - 1)
                .sum::<usize>(),
        );
        targets.push(
            row.iter()
                .map(|&i| framed[i].tokens.len() - framed[i].response_start)
                .sum::<usize>(),
        );
    }
    let mut arms = BTreeMap::new();
    for (arm, model) in [("LOCAL5", &local), ("GLOBAL6", &global)] {
        if arm == "GLOBAL6" && !active {
            continue;
        }
        let root = output.join(arm);
        std::fs::create_dir(&root)?;
        data::native::write(&root.join("corpus.r3cor"), &corpus, true)?;
        data::native::write(&root.join("transfer.r3cor"), &tx, true)?;
        let loaded = data::native::read(&root.join("corpus.r3cor"))?;
        let lx = data::native::read(&root.join("transfer.r3cor"))?;
        validate_splits(&[
            (&loaded.train, &tm),
            (&loaded.validation, &dm),
            (&lx.validation, &xm),
        ])?;
        if loaded.semantic != corpus.semantic || lx.semantic != tx.semantic {
            return Err(bad("native dataset roundtrip"));
        }
        tok.save(&root.join("tokenizer.r3b"))?;
        write(
            &root.join("metadata.r3b"),
            &(tm.clone(), dm.clone(), xm.clone()),
        )?;
        let manifest = checkpoint::initialized(model, &tok, 17, source_digest()?)?;
        checkpoint::save(
            &root.join("initial.r3m"),
            model,
            &tok,
            manifest,
            &BTreeMap::new(),
        )?;
        let loaded = checkpoint::load(&root.join("initial.r3m"), Device::Cpu, false)?;
        if loaded.manifest.training.is_some()
            || loaded.model.weights_content_id()? != model.weights_content_id()?
        {
            return Err(bad("initial checkpoint mismatch"));
        }
        let order = (0..8)
            .map(|b| {
                tm.iter()
                    .enumerate()
                    .filter_map(|(i, m)| (m.bucket == b).then_some(i))
                    .collect()
            })
            .collect();
        let policy = Plan {
            identifiable: Some(Policy {
                study: output.to_owned(),
                arm: arm.into(),
                dataset: DATASET.into(),
                rows: rows.clone(),
            }),
            schema: Some(2),
            fork: None,
            paired: None,
            training_values: None,
            grounding: None,
            revision: REVISION.into(),
            source: source_digest()?,
            binary: file_hash(&std::env::current_exe()?)?,
            corpus: file_hash(&root.join("corpus.r3cor"))?,
            transfer: file_hash(&root.join("transfer.r3cor"))?,
            tokenizer: tok.id(),
            initial: file_hash(&root.join("initial.r3m"))?,
            initial_weights: model.weight_hash()?,
            config: config(),
            architecture: model.config.clone(),
            sampler: "bucket-base-permutation-v1".into(),
            order,
            train_order: digest(&rows)?,
            split_policy: corpus.manifest.split_rule.clone(),
            data_seed: seed,
            model_seed: 17,
            tiny: false,
            metadata: file_hash(&root.join("metadata.r3b"))?,
            evaluation: evaluation(),
        };
        write(&root.join("plan.r3b"), &policy)?;
        verify_plan(&root, &policy)?;
        arms.insert(arm,binary::record!({"policy":file_hash(&root.join("plan.r3b"))?,"initial":policy.initial,"weights":model.weights_content_id()?,"architecture":model.config.semantic_id()?,"corpus":policy.corpus,"transfer":policy.transfer,"metadata":policy.metadata}));
    }
    publish_confirmed(
        &output.join("preparation.r3b"),
        &binary::record!({"contract":CONTRACT,"dataset":DATASET,"source":source_digest()?,"binary":file_hash(&std::env::current_exe()?)?,"arms":arms,"tokenizer":tok.id(),"tokenizer_parent_policy":file_hash(&parent.join("plan.r3b"))?,"vocab":tok.vocab_size(),"parameters":local.config.parameters(),"adam_clock":0,"adam_zero_hash":optimizer_hash(&adam.moments)?,"tape":digest(&rows)?,"planned_input":inputs.iter().sum::<usize>(),"planned_target":targets.iter().sum::<usize>(),"active_mask_difference":active,"numerical":numerical,"reports":reports,"updates":0,"generation":0,"teacher":0,"review_A":"PENDING","model_quality":"NOT_RUN","final200":"NOT_OPENED"}),
    )?;
    println!(
        "IDENTIFIABLE_PREPARED vocab={} parameters={} active_mask={active} weight_content={} NEW_SMALL_UPDATES=0 REVIEW_A=PENDING",
        tok.vocab_size(),
        local.config.parameters(),
        local.weights_content_id()?
    );
    Ok(())
}
// A review is an external authorization bound to this exact preparation. This
// command never writes a PASS or treats elapsed time as approval.
pub(super) fn authorize_run(_root: &Path, p: &Plan) -> Result<()> {
    let own = p.identifiable.as_ref().unwrap();
    let prep: binary::Value = read_confirmed(&own.study.join("preparation.r3b"))?;
    let review: binary::Value = read_confirmed(&own.study.join("review-a.r3b"))
        .map_err(|_| bad("independent review A required before any SMALL training"))?;
    if review["verdict"] != "PASS"
        || review["preparation"] != file_hash(&own.study.join("preparation.r3b"))?
        || review["source"] != p.source
        || review["report_hash"].as_str().is_none_or(|h| h.len() != 64)
        || prep["arms"][own.arm.as_str()]["policy"] != file_hash(&_root.join("plan.r3b"))?
    {
        return Err(bad("independent review A binding"));
    }
    let report = Path::new(
        review["report_path"]
            .as_str()
            .ok_or_else(|| bad("review report path missing"))?,
    );
    if review["report_hash"] != file_hash(report)? {
        return Err(bad("review report content changed"));
    }
    let _ = usage(p)?;
    let mut histories = vec![];
    for arm in ["LOCAL5", "GLOBAL6"] {
        if prep["arms"][arm].is_null() {
            continue;
        }
        let root = own.study.join(arm);
        let plan = plan_read(&root)?;
        histories.push((root.clone(), plan.clone(), history(&root, &plan)?));
    }
    let current = histories
        .iter()
        .find(|(r, _, _)| r == _root)
        .ok_or_else(|| bad("unknown balanced arm"))?;
    if current
        .2
        .last()
        .is_some_and(|s| s.step >= 1024 && s.phase.as_deref() != Some("EvaluationPending"))
    {
        if histories.iter().any(|(_, _, h)| {
            h.last()
                .is_none_or(|s| s.step < 1024 || s.phase.as_deref() == Some("EvaluationPending"))
        }) {
            return Err(bad("matched 1024 comparison pending"));
        }
        let mut signal = false;
        for (root, plan, _) in &histories {
            let (_, scores) = audit_panels_range(root, plan, 512, 1024)?;
            let a = &scores["eval-0512-dev512"];
            let b = &scores["eval-1024-dev512"];
            let both = |s: &PanelResult| {
                s.paired_both
                    .map(|n| n[2..=4].iter().sum::<usize>())
                    .unwrap_or(0)
            };
            let ta = &scores["eval-0512-train64"];
            let tb = &scores["eval-1024-train64"];
            signal |= b.exact >= a.exact + 16
                || both(b) >= both(a) + 8
                || ta.ce.zip(tb.ce).is_some_and(|(a, b)| b <= a * 0.9);
        }
        if !signal {
            return Err(bad(
                "INCONCLUSIVE_AT_REGISTERED_BUDGET; no automatic extension",
            ));
        }
    }
    Ok(())
}
pub(super) fn usage(p: &Plan) -> Result<(f64, usize, usize)> {
    let own = p.identifiable.as_ref().unwrap();
    let prep: binary::Value = read_confirmed(&own.study.join("preparation.r3b"))?;
    let review: binary::Value = read_confirmed(&own.study.join("review-a.r3b"))?;
    let parity = Path::new(
        review["parity_result"]
            .as_str()
            .ok_or_else(|| bad("parent parity result missing"))?,
    );
    let parity: binary::Value = read_confirmed(parity)?;
    if parity["matched"] != 16
        || parity["originals_unchanged"] != true
        || !parity["error"].is_null()
    {
        return Err(bad("parent parity failed"));
    }
    let mut elapsed = prep["numerical"]["elapsed_seconds"]
        .as_f64()
        .ok_or_else(|| bad("numeric duration UNKNOWN"))?;
    elapsed += parity["control"]["elapsed_seconds"]
        .as_f64()
        .ok_or_else(|| bad("parity duration UNKNOWN"))?;
    let mut generations = parity["control"]["generation_calls"]
        .as_u64()
        .ok_or_else(|| bad("parity generation UNKNOWN"))? as usize;
    let mut teachers = parity["control"]["teacher_calls"]
        .as_u64()
        .ok_or_else(|| bad("parity teacher UNKNOWN"))? as usize;
    for arm in ["LOCAL5", "GLOBAL6"] {
        if prep["arms"][arm].is_null() {
            continue;
        }
        let root = own.study.join(arm);
        let plan = plan_read(&root)?;
        for s in history(&root, &plan)? {
            if !s.resume && (s.step != 4096 || s.phase.as_deref() != Some("Finished")) {
                return Err(bad("balanced peer terminal blocks learning"));
            }
            elapsed += s.elapsed;
            generations += s.generations;
            teachers += s.teachers;
        }
    }
    Ok((elapsed, generations, teachers))
}
pub(super) fn remaining_input(p: &Plan) -> Result<u64> {
    let own = p.identifiable.as_ref().unwrap();
    let root = own.study.join(&own.arm);
    let mut used = 0u64;
    // The current segment start exists but is not completed; only previously
    // confirmed terminals contribute to the remaining allowance passed to the trainer.
    for n in 0..128 {
        let end = root.join(format!("segment-{n:04}-finished.r3b"));
        if !end.exists() {
            break;
        }
        let _: Segment = read_confirmed(&end)?;
        let receipt: binary::Value = read(&root.join(format!("segment-{n:04}/train-control.r3b")))?;
        used = used
            .checked_add(
                receipt["executed_input_tokens_including_uncommitted"]
                    .as_u64()
                    .ok_or_else(|| bad("balanced input usage UNKNOWN"))?,
            )
            .ok_or_else(|| bad("balanced input overflow"))?;
    }
    20_000_000u64
        .checked_sub(used)
        .ok_or_else(|| bad("balanced per-arm input exhausted"))
}
// Preserve the existing attained-best screen64 / two-consecutive-regressions
// rule. The step0 screen has only32 rows and cannot enter this comparison.
fn regression_streak(e: &EvaluationPolicy, screens: &[(usize, Option<f64>)]) -> usize {
    let mut best = 0;
    let mut best_ce = None;
    let mut streak = 0;
    for &(exact, ce) in screens {
        if best >= 32
            && exact + e.regression_exact_loss <= best
            && ce.zip(best_ce).is_some_and(|(a, b)| a >= b * e.regression_ce_ratio)
        {
            streak += 1;
        } else {
            streak = 0;
        }
        if exact > best {
            best = exact;
            best_ce = ce;
        }
    }
    streak
}
pub(super) fn evaluate(
    p: &Plan,
    root: &Path,
    path: &Path,
    step: usize,
    control: &mut recovery::RunControl,
) -> Result<Option<String>> {
    let c = verified_corpus(&root.join("corpus.r3cor"), &p.corpus)?;
    let x = verified_corpus(&root.join("transfer.r3cor"), &p.transfer)?;
    let (tm, dm, xm) = verified_metadata(root, p)?;
    let count = if step == 0 { 4 } else { 8 };
    if p.evaluation.train_steps.contains(&step) {
        let (es, ms) = subset(&c.train, &tm, count);
        evaluate_panel(p, root, path, step, "train64", &es, &ms, control)?;
    }
    if p.evaluation.screen_steps.contains(&step) {
        let (es, ms) = subset(&c.validation, &dm, count);
        evaluate_panel(p, root, path, step, "screen64", &es, &ms, control)?;
    }
    if p.evaluation.primary_steps.contains(&step) {
        let a = evaluate_panel(p, root, path, step, "dev512", &c.validation, &dm, control)?;
        let b = evaluate_panel(
            p,
            root,
            path,
            step,
            "transfer128",
            &x.validation,
            &xm,
            control,
        )?;
        if step == 4096
            && a.exact >= 487
            && a.buckets.iter().all(|&n| n >= 58)
            && b.exact >= 116
            && a.errors == 0
            && b.errors == 0
            && a.paired_both
                .is_some_and(|n| n[2..=4].iter().sum::<usize>() >= 87)
        {
            return Ok(Some(
                "BALANCED_DEVELOPMENT_PASS_OLD_REFERENCE_REQUIRED".into(),
            ));
        }
    }
    let mut screens = vec![];
    for n in [256, 512, 1024, 2048, 4096].into_iter().filter(|&n| n <= step) {
        // Reuse the existing raw-panel verifier/subsetter. Full evaluations
        // contain the same fixed64; no new generation or teacher calls occur.
        let r = paired_screen(p, root, n)?;
        screens.push((r.exact, r.ce));
    }
    if regression_streak(&p.evaluation, &screens) >= 2 {
        return Ok(Some("QUALITY_REGRESSION".into()));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiable_complementary_native_and_split() -> Result<()> {
        let (train, tm) = generate_balanced(8, 0, 20260921)?;
        let (dev, dm) = generate_balanced(8, 1, 20260921)?;
        for (es, ms) in [(&train, &tm), (&dev, &dm)] {
            assert!(cross_tables(es, ms)?.values().all(|n| n[0] == n[1]));
        }
        validate_splits(&[(&train, &tm), (&dev, &dm)])?;
        assert!(validate_splits(&[(&train, &tm), (&train, &tm)]).is_err());
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("corpus.r3cor");
        let c = new_corpus(train.clone(), dev.clone(), 20260921)?;
        data::native::write(&path, &c, true)?;
        let read = data::native::read(&path)?;
        assert_eq!(read.semantic, c.semantic);
        validate_balanced(&read.train, &tm)?;
        for bucket in 2..=4 {
            for pair in train
                .iter()
                .zip(&tm)
                .filter(|(_, m)| m.bucket == bucket)
                .collect::<Vec<_>>()
                .chunks_exact(2)
            {
                let a = pair[0].0;
                let b = pair[1].0;
                assert_ne!(citations(&a.answer)?, citations(&b.answer)?);
                let mut q = a.request.clone();
                q.evidence.items.reverse();
                assert_eq!(resolve(&q)?, a.answer);
                if bucket < 4 {
                    assert_eq!(a.request.evidence, b.request.evidence);
                }
                for rule in 0..9 {
                    if bucket == 4 {
                        continue;
                    }
                    let picks = [a, b].map(|e| {
                        rule_choice(&e.request.evidence.items, rule)
                            .unwrap()
                            .map(|r| r.event_id)
                    });
                    assert!(
                        !(picks[0] == Some(citations(&a.answer)?[0])
                            && picks[1] == Some(citations(&b.answer)?[0]))
                    );
                }
            }
        }
        let mut broken = train.clone();
        let i = tm.iter().position(|m| m.bucket == 2).unwrap();
        broken[i + 1].answer = broken[i].answer.clone();
        assert!(validate_balanced(&broken, &tm).is_err());
        for bucket in [6, 7] {
            let i = tm.iter().position(|m| m.bucket == bucket).unwrap();
            let mut duplicate = train.clone();
            duplicate[i + 1].request.input = duplicate[i].request.input.clone();
            duplicate[i + 1].request.evidence = duplicate[i].request.evidence.clone();
            duplicate[i + 1].answer = duplicate[i].answer.clone();
            assert!(validate_balanced(&duplicate, &tm).is_err());
            for pair in train.iter().zip(&tm).filter(|(_, m)| m.bucket == bucket)
                .collect::<Vec<_>>().chunks_exact(2) {
                let a = pair[0].0; let b = pair[1].0;
                if bucket == 6 { assert_ne!(a.answer, b.answer); }
                else {
                    assert_eq!(a.answer, b.answer);
                    assert_eq!(a.request.input, b.request.input);
                    let mut swapped = a.request.evidence.clone();
                    let t = swapped.items[0].observed_at;
                    swapped.items[0].observed_at = swapped.items[1].observed_at;
                    swapped.items[1].observed_at = t;
                    assert_eq!(swapped, b.request.evidence);
                }
            }
        }
        assert!(different_pair(&mut Rng::new(17), 1).is_err());
        Ok(())
    }
    #[test]
    fn identifiable_hand_label_boundaries() -> Result<()> {
        let mut q = ModelRequest {
            request_id: "hand".into(),
            system: SYSTEM.into(),
            input: format!("장치12 구역1700 {}", familiar(Intent::Current)),
            evidence: EvidenceBundle {
                items: vec![
                    record("장치12", "구역1700", "37", 91, 1, "current"),
                    record("장치34", "구역1700", "86", 22, 1, "current"),
                ],
                ..Default::default()
            },
            limits: GenerationLimits {
                context_tokens: 2048,
                max_tokens: 128,
                timeout_ms: 120000,
            },
        };
        assert_eq!(resolve(&q)?, "37입니다. [event:91]");
        q.input = q.input.replacen("장치12", "장치34", 1);
        assert_eq!(resolve(&q)?, "86입니다. [event:22]");
        q.evidence.items[1] = record("장치12", "구역12", "86", 22, 1, "current");
        q.input = q.input.replacen("장치34", "장치12", 1);
        assert_eq!(resolve(&q)?, "37입니다. [event:91]");
        q.input = q.input.replacen("구역1700", "구역12", 1);
        assert_eq!(resolve(&q)?, "86입니다. [event:22]");
        q.evidence.items[1] = record("장치12", "구역1700", "86", 22, 2, "current");
        q.input = q.input.replacen("구역12", "구역1700", 1);
        assert_eq!(resolve(&q)?, "86입니다. [event:22]");
        q.evidence.items[0].observed_at = Some(3);
        assert_eq!(resolve(&q)?, "37입니다. [event:91]");
        q.evidence.items[0].original_excerpt =
            record("장치12", "구역1700", "86", 91, 3, "current").original_excerpt;
        assert_eq!(resolve(&q)?, "86입니다. [event:91]");
        q.input = format!("장치12 구역1700 {}", familiar(Intent::Cause));
        assert_eq!(resolve(&q)?, "시간순서만으로 원인은 확정되지 않습니다.");
        Ok(())
    }
    #[test]
    fn identifiable_attained_best_consecutive_guard() {
        let e = evaluation();
        let r = |xs: &[(usize, f64)]| regression_streak(&e, &xs.iter().map(|&(n, c)| (n, Some(c))).collect::<Vec<_>>());
        //512 was weak,1024 improves,2048 and4096 both regress: must stop.
        assert_eq!(r(&[(4, 3.0), (8, 2.0), (48, 0.5), (36, 0.6), (35, 0.7)]), 2);
        assert_eq!(r(&[(48, 0.5), (36, 0.6)]), 1);
        assert_eq!(r(&[(48, 0.5), (36, 0.6), (40, 0.7), (35, 0.7)]), 1);
        assert_eq!(r(&[(31, 0.5), (0, 1.0), (0, 1.0)]), 0);
        assert_eq!(r(&[(48, 0.5), (36, 0.59), (35, 0.59)]), 0);
    }
    #[test]
    fn identifiable_finite_tape_and_schedule() -> Result<()> {
        let (es, ms) = generate_balanced(512, 0, 20260921)?;
        let rows = tape(&ms, 4096)?;
        let mut counts = vec![0; es.len()];
        for pair in rows.chunks_exact(2) {
            for bucket in 0..8 {
                let a = pair[0][bucket];
                let b = pair[1][bucket];
                assert_eq!(ms[a].bucket, bucket);
                assert_eq!(ms[b].bucket, bucket);
                assert_eq!(ms[a].base, ms[b].base);
                assert_ne!(a, b);
                counts[a] += 1;
                counts[b] += 1;
            }
        }
        assert!(counts.iter().all(|&n| n == 4));
        let c = config();
        assert_eq!(c.learning_rate(128), 3e-4);
        assert!((c.learning_rate(4096) - 3e-5).abs() < 1e-15);
        let e = evaluation();
        assert_eq!(32 + 32 + 64 + 3 * (64 + 512 + 128) + 64, 2304);
        assert!(e.teacher_steps.is_empty());
        Ok(())
    }
    #[test]
    fn identifiable_short_mask_gradient_and_long_cache() -> Result<()> {
        let local = Transformer::init(Config::tiny(264), 17, Device::Cpu)?;
        let global = prepare_model(&local, true)?;
        assert_eq!(local.weights_content_id()?, global.weights_content_id()?);
        assert_ne!(local.config.semantic_id()?, global.config.semantic_id()?);
        let short = Tensor::new(&[[8u32, 9, 10, 11, 12, 13, 14, 15]], &Device::Cpu)?;
        let a = local.forward(&short, None)?;
        let b = global.forward(&short, None)?;
        assert_eq!(
            a.flatten_all()?.to_vec1::<f32>()?,
            b.flatten_all()?.to_vec1::<f32>()?
        );
        let ga = a.sqr()?.sum_all()?.backward()?;
        let gb = b.sqr()?.sum_all()?.backward()?;
        for (name, v) in &local.vars {
            assert_eq!(
                ga.get(v).unwrap().flatten_all()?.to_vec1::<f32>()?,
                gb.get(&global.vars[name])
                    .unwrap()
                    .flatten_all()?
                    .to_vec1::<f32>()?
            );
        }
        for model in [&local, &global] {
            let mut cache = model.cache("identifiable");
            model.forward_cached(&short, &mut cache, "identifiable")?;
            let next = Tensor::new(&[[16u32]], &Device::Cpu)?;
            let cached = model.forward_cached(&next, &mut cache, "identifiable")?;
            let all = Tensor::new(&[[8u32, 9, 10, 11, 12, 13, 14, 15, 16]], &Device::Cpu)?;
            let full = model.forward(&all, None)?.narrow(1, 8, 1)?;
            let error = cached.sub(&full)?.abs()?.max_all()?.to_scalar::<f32>()?;
            assert!(error < 1e-4, "{error}");
        }
        let mask = |window| {
            neural::transformer::attention_mask(0..257, 0..257, window, None, 1, &Device::Cpu)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
        };
        let local = mask(Some(256));
        let global = mask(None);
        assert_eq!(
            local.iter().zip(global).filter(|(a, b)| **a != *b).count(),
            1
        );
        println!("IDENTIFIABLE_NUMERIC optimizer=0 generation=0 teacher=0 backward=2");
        Ok(())
    }
}

fn verify_models(
    local: &Transformer,
    global: &Transformer,
    samples: &[Sample],
) -> Result<binary::Value> {
    let timer = Instant::now();
    let short = &samples
        .iter()
        .min_by_key(|s| s.tokens.len())
        .ok_or_else(|| bad("empty numeric input"))?
        .tokens;
    let length = (short.len() - 1).min(local.config.window);
    let ids = Tensor::new(&short[..length], &Device::Cpu)?.unsqueeze(0)?;
    let a = local.forward(&ids, None)?;
    let b = global.forward(&ids, None)?;
    if a.flatten_all()?.to_vec1::<f32>()? != b.flatten_all()?.to_vec1::<f32>()? {
        return Err(bad("short model forward differs"));
    }
    let ga = a.sqr()?.sum_all()?.backward()?;
    let gb = b.sqr()?.sum_all()?.backward()?;
    for (name, v) in &local.vars {
        if ga.get(v).unwrap().flatten_all()?.to_vec1::<f32>()?
            != gb
                .get(&global.vars[name])
                .unwrap()
                .flatten_all()?
                .to_vec1::<f32>()?
        {
            return Err(bad("short model gradient differs"));
        }
    }
    let longest = samples.iter().max_by_key(|s| s.tokens.len()).unwrap();
    let mut cache_errors = vec![];
    let seq = &longest.tokens[..longest.tokens.len() - 1];
    for model in [local, global] {
        let mut cache = model.cache("identifiable-numeric");
        let prefix = Tensor::new(&seq[..seq.len() - 1], &Device::Cpu)?.unsqueeze(0)?;
        model.forward_cached(&prefix, &mut cache, "identifiable-numeric")?;
        let last = Tensor::new(&seq[seq.len() - 1..], &Device::Cpu)?.unsqueeze(0)?;
        let incremental = model.forward_cached(&last, &mut cache, "identifiable-numeric")?;
        let full = model
            .forward(&Tensor::new(seq, &Device::Cpu)?.unsqueeze(0)?, None)?
            .narrow(1, seq.len() - 1, 1)?;
        let error = incremental
            .sub(&full)?
            .abs()?
            .max_all()?
            .to_scalar::<f32>()?;
        if !error.is_finite() || error > 1e-4 {
            return Err(bad("model internal cached/full parity"));
        }
        cache_errors.push(error);
    }
    let mut masks = vec![];
    for n in [257, seq.len()] {
        let a = neural::transformer::attention_mask(0..n, 0..n, Some(256), None, 1, &Device::Cpu)?
            .flatten_all()?
            .to_vec1::<f32>()?;
        let b = neural::transformer::attention_mask(0..n, 0..n, None, None, 1, &Device::Cpu)?
            .flatten_all()?
            .to_vec1::<f32>()?;
        let delta = a.iter().zip(&b).filter(|(a, b)| a != b).count();
        if n > 256 && delta == 0 {
            return Err(bad("long mask not active"));
        }
        masks.push((n, delta));
    }
    Ok(
        binary::record!({"short_forward_exact":true,"short_gradient_exact":true,"short_length":length,"long_length":seq.len(),"cached_max_absolute_error":cache_errors,"actual_masks":masks,"forward_calls":8,"backward_calls":2,"optimizer":0,"generation":0,"teacher":0,"elapsed_seconds":timer.elapsed().as_secs_f64()}),
    )
}
