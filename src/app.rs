use crate::{
    Error, Result,
    event::*,
    model::{Model, ModelRequest, ModelResponse, SYSTEM},
    retrieval::Search,
    store::Store,
};
use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn ask(
    store: &mut Store,
    mut input: Event,
    limits: GenerationLimits,
    model: &mut impl Model,
    cancel: &AtomicBool,
) -> Result<Event> {
    limits.validate()?;
    input.kind = Kind::Observation {
        question: Some(limits.clone()),
    };
    input.validate(false)?;
    let input = store.append(input)?;
    if let Some(result) = store.result(input.id)? {
        return terminal(result);
    }
    let _generation_lock = store.generation_lock()?;
    if let Some(result) = store.result(input.id)? {
        return terminal(result);
    }
    let result = (|| {
        let original = std::str::from_utf8(&input.payload).expect("validated");
        let (prefix, byte_truncated) = crate::store::prefix(original, 4096);
        let terms: Vec<_> = prefix.split_whitespace().take(33).collect();
        let query = terms.iter().take(32).copied().collect::<Vec<_>>().join(" ");
        let mut search = Search::new(&input.scope, &query);
        search.snapshot_id = Some(input.id - 1);
        let mut evidence = store.search(&search)?;
        evidence.truncated |= byte_truncated || terms.len() > 32;
        let request = ModelRequest {
            request_id: input
                .request_key
                .expect("validated")
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            system: SYSTEM.into(),
            input: String::from_utf8(input.payload.clone()).expect("validated"),
            evidence,
            limits,
        };
        let response = model.generate(&request, cancel)?;
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let cited = match validate_response(&request, &response) {
            Ok(ids) => ids,
            Err(e) => {
                let mut failure = Event::observation(
                    &input.scope,
                    &input.session,
                    "model",
                    response.text.into_bytes(),
                );
                failure.kind = Kind::Failure {
                    input: input.id,
                    code: format!("INVALID_RESPONSE: {e}"),
                };
                if let Err(commit) = store.append(failure) {
                    return Err(Error::Model(format!(
                        "{e}; failure event NOT persisted: {commit}"
                    )));
                }
                return Err(e);
            }
        };
        let mut answer = Event::observation(
            &input.scope,
            &input.session,
            "model",
            response.text.into_bytes(),
        );
        answer.kind = Kind::AssistantAnswer {
            input: input.id,
            evidence: cited,
            provided: response.provided,
            excluded: response.excluded,
            generation: response.generation,
            retrieval_truncated: request.evidence.truncated,
        };
        terminal(store.append(answer)?)
    })();
    if let Err(ref error) = result {
        let mut failure = Event::observation(
            &input.scope,
            &input.session,
            "system",
            error.to_string().into_bytes(),
        );
        failure.kind = Kind::Failure {
            input: input.id,
            code: "ASK_FAILED".into(),
        };
        if let Err(commit) = store.append(failure) {
            return Err(Error::Model(format!(
                "{error}; failure event NOT persisted: {commit}"
            )));
        }
    }
    result
}
fn terminal(event: Event) -> Result<Event> {
    if let Kind::Failure { code, .. } = &event.kind {
        Err(Error::Model(format!(
            "persisted failure {}: {code}; {}",
            event.id,
            String::from_utf8_lossy(&event.payload)
        )))
    } else {
        Ok(event)
    }
}
pub fn citations(text: &str) -> Result<Vec<i64>> {
    let mut ids = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("[event:") {
        rest = &rest[start + 7..];
        let end = rest
            .find(']')
            .ok_or_else(|| Error::Model("malformed citation".into()))?;
        let id = rest[..end]
            .parse::<i64>()
            .map_err(|_| Error::Model("malformed citation ID".into()))?;
        if id <= 0 {
            return Err(Error::Model("invalid citation ID".into()));
        }
        ids.insert(id);
        rest = &rest[end + 1..];
    }
    Ok(ids.into_iter().collect())
}
fn validate_response(request: &ModelRequest, response: &ModelResponse) -> Result<Vec<i64>> {
    if response.request_id != request.request_id
        || response.text.is_empty()
        || response.text.len() > MAX_PAYLOAD
    {
        return Err(Error::Model("response ID/text/size mismatch".into()));
    }
    check_refs(&response.provided)?;
    check_refs(&response.excluded)?;
    let all: BTreeSet<_> = request.evidence.items.iter().map(|e| e.event_id).collect();
    let provided: BTreeSet<_> = response.provided.iter().copied().collect();
    let excluded: BTreeSet<_> = response.excluded.iter().copied().collect();
    if !provided.is_disjoint(&excluded)
        || provided.union(&excluded).copied().collect::<BTreeSet<_>>() != all
    {
        return Err(Error::Model("response evidence partition mismatch".into()));
    }
    let g = &response.generation;
    g.limits.validate()?;
    if g.limits.max_tokens != request.limits.max_tokens
        || g.limits.timeout_ms != request.limits.timeout_ms
        || g.limits.context_tokens > request.limits.context_tokens
        || g.output_tokens
            .is_some_and(|n| n > u64::from(g.limits.max_tokens))
        || g.input_tokens
            .zip(g.output_tokens)
            .is_some_and(|(i, o)| i + o > u64::from(g.limits.context_tokens))
    {
        return Err(Error::Model("response limits/usage mismatch".into()));
    }
    let ids = citations(&response.text)?;
    if ids.iter().any(|id| !provided.contains(id)) {
        return Err(Error::Model("citation outside provided bundle".into()));
    }
    Ok(ids)
}
