//! Train-only answer-byte annotations and the single normalized role-loss experiment.
//! No renderer or annotation is linked into the product inference library.
use super::*;

pub const REVISION: &str = "target-byte-union-alpha1-mass-normalized-v1";
pub const ROLE_NAMES: [&str; 6] = ["format", "entity", "context", "value", "citation", "status"];
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub role: u8,
}
#[derive(Clone, Debug)]
pub struct Annotation {
    pub spans: Vec<Span>,
    pub fractions: Vec<f64>,
    pub roles: Vec<[f64; 6]>,
    pub supported: bool,
}
fn invalid(message: &str) -> Error {
    Error::Invalid(format!("DATA_CONTRACT_FAIL: {message}"))
}
fn span(spans: &mut Vec<Span>, start: usize, end: usize, role: u8) -> Result<()> {
    if start >= end || end > u32::MAX as usize || !(1..=5).contains(&role) {
        return Err(invalid("answer span bounds"));
    }
    spans.push(Span {
        start: start as u32,
        end: end as u32,
        role,
    });
    Ok(())
}
// Delimiters come from the existing training renderer, never from question IDs.
// Positions are obtained by consuming the full grammar, not searching for a value.
fn full_fields(text: &str) -> Option<(&str, &str, &str)> {
    let body = text.strip_suffix("이다.")?;
    let (entity, rest) = body.split_once("의 ")?;
    let (context, value) = rest.split_once(" 이동 지시는 ")?;
    if [entity, context, value].iter().any(|v| v.is_empty()) {
        return None;
    }
    Some((entity, context, value))
}
pub fn annotate(e: &Episode, sample: &Sample, tok: &ByteBpe, focus: bool) -> Result<Annotation> {
    let mut spans = Vec::new();
    let answer = e.answer.as_str();
    let (body, citation) = if let Some((body, tail)) = answer.rsplit_once(" [event:") {
        if let Some(digits) = tail.strip_suffix(']') {
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                let id: i64 = digits.parse().map_err(|_| invalid("citation overflow"))?;
                let cited = e
                    .request
                    .evidence
                    .items
                    .iter()
                    .filter(|v| v.event_id == id)
                    .collect::<Vec<_>>();
                if cited.len() != 1 {
                    return Err(invalid("ambiguous or absent cited event"));
                }
                (body, Some((digits, cited[0])))
            } else {
                (answer, None)
            }
        } else {
            (answer, None)
        }
    } else {
        (answer, None)
    };
    let mut supported = false;
    if let Some((entity, context, value)) = full_fields(body) {
        let exact_source = citation
            .as_ref()
            .is_some_and(|(_, r)| r.original_excerpt == body);
        if !exact_source {
            return Err(invalid("full-copy label differs from cited original"));
        }
        if focus && e.binding != format!("{entity}/{context}/{value}") {
            return Err(invalid("focus typed binding differs from answer grammar"));
        }
        let context_start = entity.len() + "의 ".len();
        let value_start = context_start + context.len() + " 이동 지시는 ".len();
        span(&mut spans, 0, entity.len(), 1)?;
        span(&mut spans, context_start, context_start + context.len(), 2)?;
        span(&mut spans, value_start, value_start + value.len(), 3)?;
        supported = true;
    } else if let Some(value) = body.strip_suffix("입니다.") {
        // Only annotate the supported short-answer grammar with a matching source value.
        if let Some((_, record)) = &citation
            && let Some((_, _, original)) = full_fields(&record.original_excerpt)
        {
            if original != value {
                return Err(invalid("short answer differs from cited value"));
            }
            span(&mut spans, 0, value.len(), 3)?;
            supported = true;
        }
    } else if e.category == 4
        && matches!(
            body,
            "원인은 확정되지 않았습니다." | "근거가 없어 알 수 없습니다."
        )
    {
        span(&mut spans, 0, body.len() - ".".len(), 5)?;
        supported = true;
    } else if e.family.starts_with("copy/")
        && citation.is_none()
        && e.request.evidence.items.len() == 1
    {
        let record = &e.request.evidence.items[0];
        if answer == record.event_id.to_string() {
            span(&mut spans, 0, answer.len(), 4)?;
            supported = true;
        } else if full_fields(&record.original_excerpt).is_some_and(|(_, _, v)| answer == v) {
            span(&mut spans, 0, answer.len(), 3)?;
            supported = true;
        }
    }
    if focus && (!supported || full_fields(body).is_none() || citation.is_none()) {
        return Err(invalid("unsupported focus grammar"));
    }
    if supported {
        if let Some((digits, _)) = citation {
            let start = body.len() + " [event:".len();
            span(&mut spans, start, start + digits.len(), 4)?;
        }
    } else {
        spans.clear();
    }
    let (fractions, roles) = token_roles(answer, sample, tok, &spans)?;
    Ok(Annotation {
        spans,
        fractions,
        roles,
        supported,
    })
}
pub(super) fn token_roles(
    answer: &str,
    sample: &Sample,
    tok: &ByteBpe,
    spans: &[Span],
) -> Result<(Vec<f64>, Vec<[f64; 6]>)> {
    if sample.response_start >= sample.tokens.len() || sample.tokens.last() != Some(&EOS) {
        return Err(invalid("sample answer/EOS boundary"));
    }
    let mut marks = vec![0u8; answer.len()];
    for s in spans {
        if s.start >= s.end || s.end as usize > marks.len() || !(1..=5).contains(&s.role) {
            return Err(invalid("sparse role span"));
        }
        for b in &mut marks[s.start as usize..s.end as usize] {
            *b |= 1 << s.role;
        }
    }
    let mut fractions = Vec::new();
    let mut roles = Vec::new();
    let mut offset = 0usize;
    for &token in &sample.tokens[sample.response_start..sample.tokens.len() - 1] {
        let bytes = tok.decode_bytes(&[token])?;
        let end = offset
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("byte offset overflow"))?;
        if bytes.is_empty() || answer.as_bytes().get(offset..end) != Some(bytes.as_slice()) {
            return Err(invalid("full-answer token bytes changed"));
        }
        let mut role = [0.; 6];
        for bits in &marks[offset..end] {
            if *bits == 0 {
                role[0] += 1.;
            } else {
                for (i, r) in role.iter_mut().enumerate().skip(1) {
                    *r += f64::from(bits & (1 << i) != 0);
                }
            }
        }
        for r in &mut role {
            *r /= bytes.len() as f64;
        }
        fractions.push(1. - role[0]);
        roles.push(role);
        offset = end;
    }
    if offset != answer.len() {
        return Err(invalid("answer byte coverage"));
    }
    fractions.push(0.); // EOS keeps baseline supervision; it is not a semantic span.
    roles.push([1., 0., 0., 0., 0., 0.]);
    Ok((fractions, roles))
}
pub fn annotations(
    episodes: &[Episode],
    framed: &[Sample],
    tok: &ByteBpe,
    focus: &[u32],
) -> Result<Vec<Annotation>> {
    if episodes.len() != framed.len() {
        return Err(invalid("episode/sample cardinality"));
    }
    episodes
        .iter()
        .zip(framed)
        .enumerate()
        .map(|(i, (e, s))| {
            annotate(e, s, tok, focus.contains(&(i as u32)))
                .map_err(|err| Error::Invalid(format!("{err}; episode={}", e.id)))
        })
        .collect()
}
pub fn annotation_bytes(annotations: &[Annotation]) -> Vec<u8> {
    use replica_v3::codec::put_varint;
    let mut b = REVISION.as_bytes().to_vec();
    put_varint(&mut b, annotations.len() as u64);
    for a in annotations {
        b.push(u8::from(a.supported));
        put_varint(&mut b, a.spans.len() as u64);
        for s in &a.spans {
            put_varint(&mut b, u64::from(s.start));
            put_varint(&mut b, u64::from(s.end));
            b.push(s.role);
        }
    }
    b
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Probe {
    pub targets: u32,
    pub correct: u32,
    pub first_error_role: Option<u8>,
    pub base: f64,
    pub span: f64,
    pub mass: [f64; 6],
    pub nll: [f64; 6],
    pub weighted_nll: [f64; 6],
}
pub fn probe(
    model: &Transformer,
    sample: &Sample,
    annotation: &Annotation,
    weight: f64,
) -> Result<Probe> {
    let framed = std::slice::from_ref(sample);
    let b = batch(framed, &[0], &model.device)?;
    let logits = model.forward(&b.input, Some(&b.valid))?;
    let (_, baseline, n) = response_loss(&logits, &b, weight)?;
    let (_, objective, _) = loss(
        &logits,
        &b,
        framed,
        &[0],
        std::slice::from_ref(annotation),
        weight,
    )?;
    let log_probs = candle_nn::ops::log_softmax(&logits.squeeze(0)?, 1)?.to_vec2::<f32>()?;
    let mut p = Probe {
        targets: n as u32,
        correct: 0,
        first_error_role: None,
        base: f64::from(baseline.to_scalar::<f32>()?),
        span: f64::from(objective.to_scalar::<f32>()?),
        mass: [0.; 6],
        nll: [0.; 6],
        weighted_nll: [0.; 6],
    };
    for (j, roles) in annotation.roles.iter().enumerate() {
        let gold = sample.tokens[sample.response_start + j] as usize;
        let row = &log_probs[sample.response_start - 1 + j];
        let chosen = row
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        let nll = -f64::from(row[gold]);
        let a = if j == 0 { weight } else { 1. };
        p.correct += u32::from(chosen == gold);
        if chosen != gold && p.first_error_role.is_none() {
            p.first_error_role = Some(
                roles
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap()
                    .0 as u8,
            );
        }
        for (i, r) in roles.iter().enumerate() {
            p.mass[i] += a * r;
            p.nll[i] += nll * r;
            p.weighted_nll[i] += a * nll * r;
        }
    }
    Ok(p)
}
fn coefficients(fractions: &[f64], weight: f64) -> Result<Vec<f64>> {
    if fractions.is_empty()
        || !weight.is_finite()
        || !(1.0..=16.0).contains(&weight)
        || fractions
            .iter()
            .any(|r| !r.is_finite() || !(0.0..=1.0).contains(r))
    {
        return Err(invalid("role fraction/first-target weight"));
    }
    let mass = fractions.len() as f64 + weight - 1.;
    let weighted = fractions
        .iter()
        .enumerate()
        .map(|(i, r)| if i == 0 { weight * (1. + r) } else { 1. + r })
        .sum::<f64>();
    let z = mass / weighted;
    Ok(fractions
        .iter()
        .enumerate()
        .map(|(i, r)| z * (if i == 0 { weight } else { 1. }) * (1. + r))
        .collect())
}
pub(super) fn loss(
    logits: &Tensor,
    batch: &Batch,
    framed: &[Sample],
    indices: &[usize],
    annotations: &[Annotation],
    weight: f64,
) -> Result<(Tensor, Tensor, usize)> {
    let base = response_loss(logits, batch, weight)?;
    if indices
        .iter()
        .all(|i| annotations[*i].fractions.iter().all(|r| *r == 0.))
    {
        return Ok(base);
    }
    let (b, t, v) = logits.dims3()?;
    if b != indices.len() {
        return Err(invalid("weighted batch cardinality"));
    }
    let mut weights = vec![0f32; b * t];
    for (row, &i) in indices.iter().enumerate() {
        let sample = &framed[i];
        let a = &annotations[i];
        if a.fractions.len() != sample.tokens.len() - sample.response_start {
            return Err(invalid("role target denominator"));
        }
        for (j, coefficient) in coefficients(&a.fractions, weight)?.iter().enumerate() {
            // Keep the legacy objective as the base, including unsupported rows
            // in mixed batches; only the exact normalized change is added.
            weights[row * t + sample.response_start - 1 + j] =
                (*coefficient - if j == 0 { weight } else { 1. }) as f32;
        }
    }
    let log_probs = candle_nn::ops::log_softmax(&logits.reshape((b * t, v))?, 1)?;
    let selected = log_probs
        .gather(&batch.target.reshape((b * t, 1))?, 1)?
        .reshape((b, t))?;
    let weights = Tensor::from_vec(weights, (b, t), logits.device())?;
    let adjustment = ((selected * weights)?.sum_all()? / -(base.2 as f64))?;
    let objective = (&base.1 + adjustment)?;
    Ok((base.0, objective, base.2))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_loss_training_grammar_repeated_ids_citations_and_fallback() {
        use replica_v3::{
            event::GenerationLimits,
            model::{ModelRequest, SYSTEM},
            retrieval::{Evidence, EvidenceBundle},
        };
        let original = "장치000의 구역000 이동 지시는 경로000이다.";
        let mut e = Episode {
            id: "fixture".into(),
            category: 0,
            family: "skill/H3/train".into(),
            binding: "장치000/구역000/경로000".into(),
            sequence: "fixture".into(),
            request: ModelRequest {
                request_id: "fixture".into(),
                system: SYSTEM.into(),
                input: "기록을 인용해줘".into(),
                limits: GenerationLimits {
                    context_tokens: 2048,
                    max_tokens: 128,
                    timeout_ms: 30000,
                },
                evidence: EvidenceBundle {
                    items: vec![Evidence {
                        event_id: 7,
                        original_excerpt: original.into(),
                        excerpt_truncated: false,
                        source: "synthetic".into(),
                        recorded_at: 1,
                        observed_at: None,
                        version_status: "current".into(),
                        retrieval_reason: "fixture".into(),
                        relation_path: vec![],
                    }],
                    ..Default::default()
                },
            },
            answer: format!("{original} [event:0007]"),
        };
        let tok = ByteBpe::train(
            &[e.answer.as_bytes().to_vec()],
            &neural::hash(b"fixture"),
            300,
        )
        .unwrap();
        let sample = |e: &Episode| {
            let mut tokens = vec![BOS];
            tokens.extend(tok.encode(e.answer.as_bytes()).unwrap());
            tokens.push(EOS);
            Sample {
                tokens,
                response_start: 1,
                curriculum: false,
            }
        };
        let a = annotate(&e, &sample(&e), &tok, true).unwrap();
        assert_eq!(
            a.spans
                .iter()
                .map(|s| (&e.answer[s.start as usize..s.end as usize], s.role))
                .collect::<Vec<_>>(),
            vec![("장치000", 1), ("구역000", 2), ("경로000", 3), ("0007", 4)]
        );
        e.binding = "장치1/구역000/경로000".into();
        assert!(annotate(&e, &sample(&e), &tok, true).is_err());
        e.family = "qa/unsupported".into();
        e.answer = "설명할 근거를 더 확인해야 합니다.".into();
        let a = annotate(&e, &sample(&e), &tok, false).unwrap();
        assert!(!a.supported && a.fractions.iter().all(|r| *r == 0.));
        assert!(annotate(&e, &sample(&e), &tok, true).is_err());
        e.category = 4;
        e.answer = "근거가 없어 알 수 없습니다.".into();
        assert_eq!(
            annotate(&e, &sample(&e), &tok, false).unwrap().spans[0].role,
            5
        );
    }
    #[test]
    fn target_loss_tiny_optimizer_uses_weighted_full_response() {
        let sample = Sample {
            tokens: vec![BOS, 8, 9, 10, EOS],
            response_start: 2,
            curriculum: false,
        };
        let annotations = vec![Annotation {
            spans: vec![],
            fractions: vec![1., 0.25, 0.],
            roles: vec![],
            supported: true,
        }];
        let framed = vec![sample];
        let b = batch(&framed, &[0], &Device::Cpu).unwrap();
        let mut hashes = Vec::new();
        for weighted in [false, true] {
            let mut model =
                Transformer::init(neural::transformer::Config::tiny(264), 137, Device::Cpu)
                    .unwrap();
            let logits = model.forward(&b.input, Some(&b.valid)).unwrap();
            let (_, obj, n) = if weighted {
                loss(&logits, &b, &framed, &[0], &annotations, 8.).unwrap()
            } else {
                response_loss(&logits, &b, 8.).unwrap()
            };
            assert_eq!(n, 3);
            let g = obj.backward().unwrap();
            let grads = model
                .vars
                .iter()
                .map(|(k, v)| (k.clone(), g.get(v).unwrap().detach()))
                .collect();
            let mut adam = Adam::new(&model.vars).unwrap();
            let (norm, delta) = adam
                .step_constant(&model.vars, &grads, &TrainConfig::default(), 1, 1e-4)
                .unwrap();
            assert!(norm.is_finite() && delta > 0.);
            model.refresh_identity().unwrap();
            hashes.push(model.weight_hash().unwrap());
        }
        assert_ne!(hashes[0], hashes[1]);
        println!("OBJECTIVE_TINY_UPDATES=2 SMALL_UPDATES=0 SCALAR_OPTIMIZER=0");
    }
    #[test]
    fn target_loss_scalar_gradient_mass_padding_and_accumulation() {
        let samples = vec![
            Sample {
                tokens: vec![BOS, 8, 9, 10, EOS],
                response_start: 3,
                curriculum: false,
            },
            Sample {
                tokens: vec![BOS, 11, 9, EOS],
                response_start: 1,
                curriculum: false,
            },
        ];
        let mut annotations = vec![
            Annotation {
                spans: vec![],
                fractions: vec![0.25, 0.],
                roles: vec![],
                supported: true,
            },
            Annotation {
                spans: vec![],
                fractions: vec![1., 0.5, 0.],
                roles: vec![],
                supported: true,
            },
        ];
        let values: Vec<f32> = (0..96).map(|i| (i % 19) as f32 * 0.1 - 0.9).collect();
        let logits = Var::from_vec(values.clone(), (2, 4, 12), &Device::Cpu).unwrap();
        let batch = batch(&samples, &[0, 1], &Device::Cpu).unwrap();
        let targets = batch
            .target
            .flatten_all()
            .unwrap()
            .to_vec1::<u32>()
            .unwrap();
        for weight in [1., 8.] {
            let (_, obj, count) =
                loss(&logits, &batch, &samples, &[0, 1], &annotations, weight).unwrap();
            assert_eq!(count, 5);
            let grads = obj
                .backward()
                .unwrap()
                .get(&logits)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            // Independent scalar softmax and the explicitly expanded per-episode formula.
            let z0 = (weight + 1.) / (weight * 1.25 + 1.);
            let z1 = (weight + 2.) / (weight * 2. + 2.5);
            let weights = [
                0.,
                0.,
                z0 * weight * 1.25,
                z0,
                z1 * weight * 2.,
                z1 * 1.5,
                z1,
                0.,
            ];
            let mut expected = 0.;
            for (row, x) in values.as_chunks::<12>().0.iter().enumerate() {
                let sum = x.iter().map(|x| f64::from(*x).exp()).sum::<f64>();
                expected += (sum.ln() - f64::from(x[targets[row] as usize])) * weights[row] / 5.;
                for (col, x) in x.iter().enumerate() {
                    let g = (f64::from(*x).exp() / sum - f64::from(col == targets[row] as usize))
                        * weights[row]
                        / 5.;
                    assert!((f64::from(grads[row * 12 + col]) - g).abs() < 1e-6);
                }
            }
            assert!((f64::from(obj.to_scalar::<f32>().unwrap()) - expected).abs() < 2e-6);
            for a in &annotations {
                assert!(
                    (coefficients(&a.fractions, weight)
                        .unwrap()
                        .iter()
                        .sum::<f64>()
                        - (weight + a.fractions.len() as f64 - 1.))
                        .abs()
                        < 1e-12
                );
            }
            let mut micro_total = 0.;
            let mut accumulated = Tensor::new(0f32, &Device::Cpu).unwrap();
            for i in 0..2 {
                let mb = super::super::batch(&samples, &[i], &Device::Cpu).unwrap();
                let ml = logits
                    .narrow(0, i, 1)
                    .unwrap()
                    .narrow(1, 0, mb.input.dims()[1])
                    .unwrap();
                let (_, o, n) = loss(&ml, &mb, &samples, &[i], &annotations, weight).unwrap();
                micro_total += f64::from(o.to_scalar::<f32>().unwrap()) * n as f64 / 5.;
                accumulated = (&accumulated + (&o * (n as f64 / 5.)).unwrap()).unwrap();
            }
            assert!((micro_total - expected).abs() < 2e-6);
            let micro_gradients = accumulated.backward().unwrap();
            let micro_gradients = micro_gradients
                .get(&logits)
                .unwrap()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap();
            assert!(
                micro_gradients
                    .iter()
                    .zip(&grads)
                    .all(|(a, b)| (a - b).abs() < 1e-6)
            );
            // Central difference of an independent f64 scalar expression, fixed h/tolerance.
            for index in [0, 24, 31, 48, 59, 95] {
                let scalar = |delta: f64| {
                    values
                        .as_chunks::<12>()
                        .0
                        .iter()
                        .enumerate()
                        .map(|(row, x)| {
                            let x = x
                                .iter()
                                .enumerate()
                                .map(|(col, v)| {
                                    f64::from(*v) + if row * 12 + col == index { delta } else { 0. }
                                })
                                .collect::<Vec<_>>();
                            (x.iter().map(|v| v.exp()).sum::<f64>().ln() - x[targets[row] as usize])
                                * weights[row]
                                / 5.
                        })
                        .sum::<f64>()
                };
                let fd = (scalar(1e-4) - scalar(-1e-4)) / 2e-4;
                assert!((fd - f64::from(grads[index])).abs() < 1e-6);
            }
        }
        // A zero-span episode in a mixed batch keeps the original gradient;
        // the other episode still receives the normalized adjustment.
        annotations[0].fractions.fill(0.);
        let (_, mixed, _) = loss(&logits, &batch, &samples, &[0, 1], &annotations, 8.).unwrap();
        let mixed = mixed
            .backward()
            .unwrap()
            .get(&logits)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let baseline = response_loss(&logits, &batch, 8.)
            .unwrap()
            .1
            .backward()
            .unwrap()
            .get(&logits)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        assert!(
            mixed[..48]
                .iter()
                .zip(&baseline[..48])
                .all(|(a, b)| (a - b).abs() < 1e-6)
        );
        assert!(
            mixed[48..]
                .iter()
                .zip(&baseline[48..])
                .any(|(a, b)| (a - b).abs() > 1e-6)
        );
        for a in &mut annotations {
            a.fractions.fill(0.);
        }
        let base = response_loss(&logits, &batch, 8.).unwrap();
        let span = loss(&logits, &batch, &samples, &[0, 1], &annotations, 8.).unwrap();
        assert_eq!(
            base.1.to_scalar::<f32>().unwrap().to_bits(),
            span.1.to_scalar::<f32>().unwrap().to_bits()
        );
        assert_eq!(
            coefficients(&[0.5, 0.5, 0.5], 8.).unwrap(),
            vec![8., 1., 1.]
        );
    }
    #[test]
    fn target_loss_answer_byte_union_preserves_full_tokenization() {
        let answer = "장치000의 구역000 이동 지시는 경로000이다. [event:0007]";
        let tok = ByteBpe::train(
            &[answer.as_bytes().to_vec()],
            &neural::hash(b"fixture"),
            300,
        )
        .unwrap();
        let mut tokens = vec![BOS];
        tokens.extend(tok.encode(answer.as_bytes()).unwrap());
        tokens.push(EOS);
        let sample = Sample {
            tokens,
            response_start: 1,
            curriculum: false,
        };
        let spans = vec![
            Span {
                start: 0,
                end: 9,
                role: 1,
            },
            Span {
                start: 3,
                end: 9,
                role: 1,
            },
        ];
        let (fractions, _) = token_roles(answer, &sample, &tok, &spans).unwrap();
        assert_eq!(*fractions.last().unwrap(), 0.);
        assert!(fractions.iter().all(|r| (0.0..=1.0).contains(r)));
        let mut byte_count = 0.;
        for (&token, &r) in sample.tokens[1..sample.tokens.len() - 1]
            .iter()
            .zip(&fractions)
        {
            byte_count += tok.decode_bytes(&[token]).unwrap().len() as f64 * r;
        }
        assert!((byte_count - 9.).abs() < 1e-12);
        let mut wrong = sample.clone();
        wrong.tokens[1] = EOS;
        assert!(token_roles(answer, &wrong, &tok, &spans).is_err());
        assert!(
            token_roles(
                answer,
                &sample,
                &tok,
                &[Span {
                    start: 0,
                    end: 999,
                    role: 1
                }]
            )
            .is_err()
        );
    }
}
