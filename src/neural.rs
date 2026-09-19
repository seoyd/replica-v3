//! Project-owned byte tokenizer and native neural model. No pretrained artifacts.
use crate::{
    Error, Result,
    event::{MAX_PAYLOAD, check_refs},
    model::{MAX_REQUEST, ModelRequest, PreparedPrompt},
    retrieval::Evidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};
use tokenizers::{
    AddedToken, Model, Trainer,
    models::bpe::{BPE, BpeTrainer},
};

pub const PAD: u32 = 0;
pub const BOS: u32 = 1;
pub const EOS: u32 = 2;
pub const SYSTEM_ROLE: u32 = 3;
pub const USER_ROLE: u32 = 4;
pub const EVIDENCE_ROLE: u32 = 5;
pub const ASSISTANT_ROLE: u32 = 6;
pub const END_ROLE: u32 = 7;
pub const SPECIALS: usize = 8;
pub const MAX_VOCAB: usize = 4096;
pub const MAX_TOKENIZER_BYTES: usize = 2 * 1024 * 1024;
pub const PROMPT_FORMAT: &str = "native-role-bytes-v1";

pub fn cpu_backend() -> &'static str {
    if cfg!(feature = "accelerate") {
        "CPU/Accelerate"
    } else {
        "CPU/gemm"
    }
}

/// Shared byte framing: tokenizer training may learn only the train split's headers.
pub fn evidence_text(e: &Evidence) -> String {
    format!(
        "[event:{}] source={} recorded={} observed={:?} status={} excerpt_truncated={}\n{}",
        e.event_id,
        e.source,
        e.recorded_at,
        e.observed_at,
        e.version_status,
        e.excerpt_truncated,
        e.original_excerpt
    )
}

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(Error::Invalid("artifact exceeds byte budget".into()));
    }
    Ok(bytes)
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn token_error(e: impl std::fmt::Display) -> Error {
    Error::Model(format!("native tokenizer: {e}"))
}
fn alphabet(byte: u8) -> char {
    char::from_u32(0x100 + u32::from(byte)).expect("byte alphabet")
}
fn special(id: usize) -> String {
    char::from_u32(0xe000 + id as u32)
        .expect("reserved alphabet")
        .to_string()
}
// Numeric and ASCII separator bytes are independent segments. No normalization:
// every byte is retained, and arbitrary non-UTF8 input can roundtrip as bytes.
fn segments(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    for &b in bytes {
        if b.is_ascii() && !b.is_ascii_alphabetic() {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            out.push(alphabet(b).to_string());
        } else {
            word.push(alphabet(b));
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenizerFile {
    version: u32,
    train_hash: String,
    vocab: BTreeMap<String, u32>,
    merges: Vec<(String, String)>,
}
pub struct ByteBpe {
    model: BPE,
    bytes: Vec<u8>,
    tokens: Vec<Vec<u8>>,
    merges: Vec<(u32, u32)>,
    wire_id: String,
    pub train_hash: String,
}
impl ByteBpe {
    pub fn train(documents: &[Vec<u8>], train_hash: &str, vocab: usize) -> Result<Self> {
        if !(256 + SPECIALS..=MAX_VOCAB).contains(&vocab)
            || documents.is_empty()
            || documents.iter().any(|d| d.len() > MAX_PAYLOAD)
        {
            return Err(Error::Invalid("tokenizer corpus/vocab bounds".into()));
        }
        let docs: Vec<String> = documents.iter().flat_map(|d| segments(d)).collect();
        let mut trainer = BpeTrainer::builder()
            .vocab_size(vocab)
            .min_frequency(2)
            .show_progress(false)
            .max_token_length(Some(32))
            .initial_alphabet((0..=255).map(alphabet).collect())
            .special_tokens(
                (0..SPECIALS)
                    .map(|i| AddedToken::from(special(i), true))
                    .collect(),
            )
            .build();
        trainer
            .feed(docs.iter(), |s| Ok(vec![s.to_string()]))
            .map_err(token_error)?;
        let mut model = BPE::default();
        trainer.train(&mut model).map_err(token_error)?;
        let value = crate::binary::to_value(&model)?;
        let file = TokenizerFile {
            version: 1,
            train_hash: train_hash.into(),
            vocab: model.get_vocab().into_iter().collect(),
            merges: crate::binary::from_value(value["merges"].clone())?,
        };
        Self::from_bytes(&crate::binary::to_vec(&file)?)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_TOKENIZER_BYTES {
            return Err(token_error("oversized metadata"));
        }
        let file: TokenizerFile = crate::binary::from_slice(bytes)?;
        Self::from_file(file, bytes.to_vec(), hash(bytes))
    }
    fn from_file(file: TokenizerFile, bytes: Vec<u8>, wire_id: String) -> Result<Self> {
        let n = file.vocab.len();
        if file.version != 1
            || !(256 + SPECIALS..=MAX_VOCAB).contains(&n)
            || file.merges.len() > MAX_VOCAB - 256 - SPECIALS
            || file.train_hash.len() != 64
            || !file.train_hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(token_error("version/vocab/lineage bounds"));
        }
        let mut tokens = vec![Vec::new(); n];
        let mut seen = vec![false; n];
        for (text, &id) in &file.vocab {
            let id = id as usize;
            if id >= n || seen[id] {
                return Err(token_error("noncontiguous/duplicate token ID"));
            }
            seen[id] = true;
            if id < SPECIALS {
                if *text != special(id) {
                    return Err(token_error("reserved token mismatch"));
                }
            } else {
                let raw = text
                    .chars()
                    .map(|c| u8::try_from((c as u32).wrapping_sub(0x100)))
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(token_error)?;
                if raw.is_empty() || raw.len() > 32 {
                    return Err(token_error("token length"));
                }
                tokens[id] = raw;
            }
        }
        for b in 0..=255 {
            if !file.vocab.contains_key(&alphabet(b).to_string()) {
                return Err(token_error("missing byte fallback"));
            }
        }
        let mut available: std::collections::BTreeSet<String> =
            (0..=255).map(|b| alphabet(b).to_string()).collect();
        for (a, b) in &file.merges {
            if !file.vocab.contains_key(a)
                || !file.vocab.contains_key(b)
                || !file.vocab.contains_key(&format!("{a}{b}"))
                || a.chars()
                    .chain(b.chars())
                    .any(|c| !(0x100..=0x1ff).contains(&(c as u32)))
                || !available.contains(a)
                || !available.contains(b)
                || !available.insert(format!("{a}{b}"))
            {
                return Err(token_error("invalid merge"));
            }
        }
        if available.len() + SPECIALS != n {
            return Err(token_error("unreachable vocabulary token"));
        }
        let merges = file
            .merges
            .iter()
            .map(|(a, b)| (file.vocab[a], file.vocab[b]))
            .collect();
        let vocab: tokenizers::models::bpe::Vocab = file.vocab.into_iter().collect();
        let model = BPE::builder()
            .vocab_and_merges(vocab, file.merges)
            .build()
            .map_err(token_error)?;
        Ok(Self {
            model,
            bytes,
            wire_id,
            tokens,
            merges,
            train_hash: file.train_hash,
        })
    }
    pub fn load(path: &Path) -> Result<Self> {
        Self::from_bytes(&read_bounded(path, MAX_TOKENIZER_BYTES)?)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if self.bytes.is_empty() {
            return Err(token_error("embedded tokenizer has no standalone record bytes"));
        }
        write_new(path, &self.bytes)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn id(&self) -> String {
        self.wire_id.clone()
    }
    pub(crate) fn mapping(&self) -> (&[Vec<u8>], &[(u32, u32)]) {
        (&self.tokens, &self.merges)
    }
    pub(crate) fn from_mapping(
        tokens: Vec<Vec<u8>>,
        merges: Vec<(u32, u32)>,
        train_hash: String,
        wire_id: String,
    ) -> Result<Self> {
        if !(264..=MAX_VOCAB).contains(&tokens.len())
            || merges.len() > MAX_VOCAB - 264
            || wire_id.len() != 64
            || !wire_id.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(token_error("native mapping bounds"));
        }
        let mut vocab = BTreeMap::new();
        let mut strings = Vec::with_capacity(tokens.len());
        for (id, raw) in tokens.iter().enumerate() {
            if raw.len() > 32 || (id < SPECIALS && !raw.is_empty()) {
                return Err(token_error("native token bytes"));
            }
            let s = if id < SPECIALS {
                special(id)
            } else {
                raw.iter().map(|&b| alphabet(b)).collect()
            };
            if vocab.insert(s.clone(), id as u32).is_some() {
                return Err(token_error("duplicate native token"));
            }
            strings.push(s);
        }
        let pairs = merges
            .iter()
            .map(|&(a, b)| {
                Ok((
                    strings
                        .get(a as usize)
                        .ok_or_else(|| token_error("merge ID"))?
                        .clone(),
                    strings
                        .get(b as usize)
                        .ok_or_else(|| token_error("merge ID"))?
                        .clone(),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Self::from_file(
            TokenizerFile {
                version: 1,
                train_hash,
                vocab,
                merges: pairs,
            },
            Vec::new(),
            wire_id,
        )
    }
    /// Semantic byte mapping and ordered merge ranks, excluding JSON and training lineage.
    pub fn semantic_id(&self) -> String {
        let mut bytes = b"native-byte-bpe-special8-ascii-segments-v1".to_vec();
        crate::codec::put_varint(&mut bytes, self.tokens.len() as u64);
        for token in &self.tokens {
            crate::codec::put_bytes(&mut bytes, token);
        }
        crate::codec::put_varint(&mut bytes, self.merges.len() as u64);
        for &(a, b) in &self.merges {
            crate::codec::put_varint(&mut bytes, u64::from(a));
            crate::codec::put_varint(&mut bytes, u64::from(b));
        }
        hash(&bytes)
    }
    pub fn vocab_size(&self) -> usize {
        self.tokens.len()
    }
    pub fn encode(&self, bytes: &[u8]) -> Result<Vec<u32>> {
        if bytes.len() > MAX_REQUEST {
            return Err(Error::Invalid("tokenizer input byte limit".into()));
        }
        let mut ids = Vec::new();
        for part in segments(bytes) {
            for token in self.model.tokenize(&part).map_err(token_error)? {
                if token.id < SPECIALS as u32 || token.id as usize >= self.tokens.len() {
                    return Err(token_error("untrusted control token"));
                }
                ids.push(token.id);
            }
        }
        Ok(ids)
    }
    pub fn decode_bytes(&self, ids: &[u32]) -> Result<Vec<u8>> {
        if ids.len() > MAX_REQUEST {
            return Err(token_error("decode token limit"));
        }
        let mut out = Vec::new();
        for &id in ids {
            let bytes = self
                .tokens
                .get(id as usize)
                .filter(|_| id >= SPECIALS as u32)
                .ok_or_else(|| token_error("control/out-of-range output token"))?;
            if out.len() + bytes.len() > MAX_PAYLOAD {
                return Err(token_error("decode byte limit"));
            }
            out.extend_from_slice(bytes);
        }
        Ok(out)
    }
    pub fn decode(&self, ids: &[u32]) -> Result<String> {
        String::from_utf8(self.decode_bytes(ids)?)
            .map_err(|_| token_error("invalid/incomplete output UTF-8"))
    }
    pub fn prepare(
        &self,
        request: &ModelRequest,
        context: u32,
        config_id: &str,
    ) -> Result<PreparedPrompt> {
        request.limits.validate()?;
        check_refs(
            &request
                .evidence
                .items
                .iter()
                .map(|e| e.event_id)
                .collect::<Vec<_>>(),
        )?;
        if request.input.is_empty()
            || request
                .evidence
                .items
                .iter()
                .any(|e| e.original_excerpt.is_empty())
        {
            return Err(Error::Invalid("empty question/evidence".into()));
        }
        let mut evidence = request.evidence.items.clone();
        let mut excluded = Vec::new();
        if request
            .system
            .len()
            .checked_add(request.input.len())
            .is_none_or(|n| n > MAX_REQUEST)
        {
            return Err(Error::ContextTooSmall);
        }
        loop {
            let mut ids = vec![BOS, SYSTEM_ROLE];
            ids.extend(self.encode(request.system.as_bytes())?);
            ids.extend([END_ROLE, USER_ROLE]);
            ids.extend(self.encode(request.input.as_bytes())?);
            ids.push(END_ROLE);
            let mut rendered_bytes = request.system.len() + request.input.len();
            for e in &evidence {
                ids.push(EVIDENCE_ROLE);
                let text = evidence_text(e);
                rendered_bytes += text.len();
                if rendered_bytes > MAX_REQUEST {
                    break;
                }
                ids.extend(self.encode(text.as_bytes())?);
                ids.push(END_ROLE);
            }
            ids.push(ASSISTANT_ROLE);
            if rendered_bytes <= MAX_REQUEST
                && ids
                    .len()
                    .checked_add(request.limits.max_tokens as usize)
                    .is_some_and(|n| n <= context.min(request.limits.context_tokens) as usize)
            {
                let mut digest = Sha256::new();
                for id in &ids {
                    digest.update(id.to_le_bytes());
                }
                return Ok(PreparedPrompt {
                    token_ids: ids,
                    provided: evidence.iter().map(|e| e.event_id).collect(),
                    excluded,
                    tokenizer_id: self.semantic_id(),
                    config_id: config_id.into(),
                    token_digest: format!("{:x}", digest.finalize()),
                });
            }
            match evidence.pop() {
                Some(e) => excluded.push(e.event_id),
                None => return Err(Error::ContextTooSmall),
            }
        }
    }
}

pub mod artifact;
pub mod checkpoint;
pub mod transformer;
