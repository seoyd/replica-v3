use crate::{Error, Result, event::*};
use std::io::Read;
pub const MAX_BODY: usize = 1_048_576;
pub const HEADER: usize = 20;
#[derive(Debug, Clone, Copy)]
pub enum Compression {
    Raw,
    Auto,
}
fn corrupt(s: &str) -> Error {
    Error::Corrupt(s.into())
}
// Adapted from replica-v2 snapshot.rs canonical_varint; provenance in docs/REUSE.md.
pub fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}
pub fn decode_varint(bytes: &[u8]) -> Result<(u64, usize)> {
    let mut value = 0;
    for index in 0..10 {
        let byte = *bytes
            .get(index)
            .ok_or_else(|| corrupt("truncated varint"))?;
        if index == 9 && byte > 1 {
            return Err(corrupt("varint overflow"));
        }
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            if index > 0 && byte == 0 {
                return Err(corrupt("noncanonical varint"));
            }
            return Ok((value, index + 1));
        }
    }
    Err(corrupt("varint overflow"))
}
pub(crate) fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    put_varint(out, b.len() as u64);
    out.extend_from_slice(b);
}
fn text(out: &mut Vec<u8>, s: &str) {
    put_bytes(out, s.as_bytes());
}
fn signed(out: &mut Vec<u8>, n: i64) {
    put_varint(out, ((n as u64) << 1) ^ ((n >> 63) as u64));
}
fn option<T>(out: &mut Vec<u8>, v: Option<T>, f: impl FnOnce(&mut Vec<u8>, T)) {
    match v {
        None => out.push(0),
        Some(v) => {
            out.push(1);
            f(out, v);
        }
    }
}
fn id(out: &mut Vec<u8>, n: i64) {
    put_varint(out, n as u64);
}
fn ids(out: &mut Vec<u8>, ns: &[i64]) {
    put_varint(out, ns.len() as u64);
    for n in ns {
        id(out, *n);
    }
}
fn limits(out: &mut Vec<u8>, l: &GenerationLimits) {
    put_varint(out, l.max_tokens.into());
    put_varint(out, l.context_tokens.into());
    put_varint(out, l.timeout_ms);
}
pub fn encode(event: &Event, compression: Compression) -> Result<Vec<u8>> {
    event.validate(true)?;
    let mut b = Vec::new();
    id(&mut b, event.id);
    signed(&mut b, event.recorded_at);
    option(&mut b, event.observed_at, signed);
    for s in [&event.source, &event.scope, &event.session] {
        text(&mut b, s);
    }
    option(&mut b, event.request_key, |b, k| b.extend_from_slice(&k));
    put_bytes(&mut b, &event.payload);
    b.push(event.kind.tag());
    match &event.kind {
        Kind::Observation { question } => option(&mut b, question.as_ref(), limits),
        Kind::Fact {
            slot,
            previous,
            restored_from,
            valid_from,
            valid_until,
        } => {
            for s in [&slot.entity, &slot.predicate, &slot.context] {
                text(&mut b, s);
            }
            option(&mut b, *previous, id);
            option(&mut b, *restored_from, id);
            option(&mut b, *valid_from, signed);
            option(&mut b, *valid_until, signed);
        }
        Kind::Relation {
            relation,
            from,
            to,
            evidence,
        } => {
            b.push(*relation as u8);
            id(&mut b, *from);
            id(&mut b, *to);
            ids(&mut b, evidence);
        }
        Kind::AssistantAnswer {
            input,
            evidence,
            provided,
            excluded,
            generation: g,
            retrieval_truncated,
        } => {
            id(&mut b, *input);
            for r in [evidence, provided, excluded] {
                ids(&mut b, r);
            }
            for s in [
                &g.model_id,
                &g.model_revision,
                &g.runtime_revision,
                &g.quantization,
                &g.license,
                &g.finish_reason,
            ] {
                text(&mut b, s);
            }
            option(&mut b, g.input_tokens, put_varint);
            option(&mut b, g.output_tokens, put_varint);
            limits(&mut b, &g.limits);
            put_varint(&mut b, g.load_ms);
            option(&mut b, g.first_token_ms, put_varint);
            put_varint(&mut b, g.generation_ms);
            b.push(u8::from(*retrieval_truncated));
        }
        Kind::Failure { input, code } => {
            id(&mut b, *input);
            text(&mut b, code);
        }
    }
    envelope(&b, compression)
}
fn envelope(body: &[u8], compression: Compression) -> Result<Vec<u8>> {
    if body.len() > MAX_BODY {
        return Err(corrupt("body too large"));
    }
    let compressed = if matches!(compression, Compression::Auto) && body.len() >= 256 {
        Some(zstd::bulk::compress(body, 3)?)
    } else {
        None
    };
    let (codec, stored) = match compressed.as_ref() {
        Some(c) if c.len() < body.len() => (1, c.as_slice()),
        _ => (0, body),
    };
    let mut out = Vec::with_capacity(HEADER + stored.len());
    out.extend_from_slice(b"RPV3");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.push(codec);
    out.push(0);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&(stored.len() as u32).to_le_bytes());
    out.extend_from_slice(&crc32c::crc32c(body).to_le_bytes());
    out.extend_from_slice(stored);
    Ok(out)
}
struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| corrupt("length overflow"))?;
        let out = self
            .b
            .get(self.pos..end)
            .ok_or_else(|| corrupt("truncated body"))?;
        self.pos = end;
        Ok(out)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn var(&mut self) -> Result<u64> {
        let (v, n) = decode_varint(&self.b[self.pos..])?;
        self.pos += n;
        Ok(v)
    }
    fn signed(&mut self) -> Result<i64> {
        let n = self.var()?;
        Ok(((n >> 1) as i64) ^ -((n & 1) as i64))
    }
    fn id(&mut self) -> Result<i64> {
        let n = i64::try_from(self.var()?).map_err(|_| corrupt("ID overflow"))?;
        if n <= 0 {
            Err(corrupt("nonpositive ID"))
        } else {
            Ok(n)
        }
    }
    fn opt<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<Option<T>> {
        match self.byte()? {
            0 => Ok(None),
            1 => f(self).map(Some),
            _ => Err(corrupt("option tag")),
        }
    }
    fn bytes(&mut self, max: usize) -> Result<Vec<u8>> {
        let n = usize::try_from(self.var()?).map_err(|_| corrupt("length overflow"))?;
        if n > max {
            return Err(corrupt("field too large"));
        }
        Ok(self.take(n)?.to_vec())
    }
    fn text(&mut self) -> Result<String> {
        String::from_utf8(self.bytes(MAX_TEXT)?).map_err(|_| corrupt("UTF-8"))
    }
    fn ids(&mut self) -> Result<Vec<i64>> {
        let n = self.var()?;
        if n > 8 {
            return Err(corrupt("too many refs"));
        }
        (0..n).map(|_| self.id()).collect()
    }
    fn limits(&mut self) -> Result<GenerationLimits> {
        Ok(GenerationLimits {
            max_tokens: u32::try_from(self.var()?).map_err(|_| corrupt("token overflow"))?,
            context_tokens: u32::try_from(self.var()?).map_err(|_| corrupt("context overflow"))?,
            timeout_ms: self.var()?,
        })
    }
    fn bool(&mut self) -> Result<bool> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(corrupt("bool tag")),
        }
    }
}
pub fn decode(bytes: &[u8]) -> Result<Event> {
    if bytes.len() < HEADER || bytes.len() > HEADER + MAX_BODY {
        return Err(corrupt("envelope length"));
    }
    if &bytes[..4] != b"RPV3" || bytes[4..6] != 1u16.to_le_bytes() || bytes[7] != 0 {
        return Err(corrupt("magic/version/flags"));
    }
    let u32_at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("checked header"));
    let raw = u32_at(8) as usize;
    let stored = u32_at(12) as usize;
    if raw > MAX_BODY || stored > MAX_BODY || HEADER + stored != bytes.len() {
        return Err(corrupt("declared length"));
    }
    let body = match bytes[6] {
        0 => {
            if raw != stored {
                return Err(corrupt("raw length"));
            }
            bytes[HEADER..].to_vec()
        }
        1 => {
            let frame_size = zstd::zstd_safe::find_frame_compressed_size(&bytes[HEADER..])
                .map_err(|_| corrupt("zstd frame"))?;
            if frame_size != stored {
                return Err(corrupt("trailing zstd frame/bytes"));
            }
            let mut decoder = zstd::stream::read::Decoder::new(&bytes[HEADER..])
                .map_err(|_| corrupt("zstd header"))?;
            decoder
                .window_log_max(20)
                .map_err(|_| corrupt("zstd window"))?;
            let mut b = Vec::new();
            decoder
                .take((raw + 1) as u64)
                .read_to_end(&mut b)
                .map_err(|_| corrupt("zstd decode"))?;
            if b.len() != raw {
                return Err(corrupt("zstd expansion length"));
            }
            b
        }
        _ => return Err(corrupt("unknown codec")),
    };
    if crc32c::crc32c(&body) != u32_at(16) {
        return Err(corrupt("checksum"));
    }
    let mut r = Reader { b: &body, pos: 0 };
    let id = r.id()?;
    let recorded_at = r.signed()?;
    let observed_at = r.opt(Reader::signed)?;
    let source = r.text()?;
    let scope = r.text()?;
    let session = r.text()?;
    let request_key = r.opt(|r| Ok(r.take(16)?.try_into().expect("16 bytes")))?;
    let payload = r.bytes(MAX_PAYLOAD)?;
    let kind = match r.byte()? {
        0 => Kind::Observation {
            question: r.opt(Reader::limits)?,
        },
        1 => Kind::Fact {
            slot: Slot {
                entity: r.text()?,
                predicate: r.text()?,
                context: r.text()?,
            },
            previous: r.opt(Reader::id)?,
            restored_from: r.opt(Reader::id)?,
            valid_from: r.opt(Reader::signed)?,
            valid_until: r.opt(Reader::signed)?,
        },
        2 => Kind::Relation {
            relation: RelationKind::from_tag(r.byte()?)?,
            from: r.id()?,
            to: r.id()?,
            evidence: r.ids()?,
        },
        3 => Kind::AssistantAnswer {
            input: r.id()?,
            evidence: r.ids()?,
            provided: r.ids()?,
            excluded: r.ids()?,
            generation: GenerationInfo {
                model_id: r.text()?,
                model_revision: r.text()?,
                runtime_revision: r.text()?,
                quantization: r.text()?,
                license: r.text()?,
                finish_reason: r.text()?,
                input_tokens: r.opt(Reader::var)?,
                output_tokens: r.opt(Reader::var)?,
                limits: r.limits()?,
                load_ms: r.var()?,
                first_token_ms: r.opt(Reader::var)?,
                generation_ms: r.var()?,
            },
            retrieval_truncated: r.bool()?,
        },
        4 => Kind::Failure {
            input: r.id()?,
            code: r.text()?,
        },
        _ => return Err(corrupt("unknown kind")),
    };
    if r.pos != body.len() {
        return Err(corrupt("trailing bytes"));
    }
    let e = Event {
        id,
        recorded_at,
        observed_at,
        source,
        scope,
        session,
        request_key,
        payload,
        kind,
    };
    e.validate(true).map_err(|e| corrupt(&e.to_string()))?;
    Ok(e)
}
