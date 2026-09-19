//! Train-only, lossless source episodes. Compression and physical provenance are not content identity.
use super::*;
use replica_v3::{
    codec::{Reader, publish_new, put_bytes, put_varint},
    event::RelationKind,
    retrieval::RelationStep,
};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
const PREFIX: usize = 160;
const MAX_FILE: usize = 128 * 1024 * 1024;
const MAX_RECORD: usize = 64 * 1024 * 1024;
const BLOCK_TARGET: usize = 256 * 1024;
type Hash = [u8; 32];
fn bad(s: &str) -> Error {
    Error::Corrupt(format!("R3CORP: {s}"))
}
fn digest(b: &[u8]) -> Hash {
    Sha256::digest(b).into()
}
fn string(b: &mut Vec<u8>, s: &str) {
    put_bytes(b, s.as_bytes());
}
fn text(r: &mut Reader<'_>) -> Result<String> {
    String::from_utf8(r.bytes(262144)?).map_err(|_| bad("UTF-8"))
}
fn count(r: &mut Reader<'_>, max: usize) -> Result<usize> {
    usize::try_from(r.var()?)
        .ok()
        .filter(|n| *n <= max)
        .ok_or_else(|| bad("count"))
}
fn u32_read(r: &mut Reader<'_>) -> Result<u32> {
    u32::try_from(r.var()?).map_err(|_| bad("u32"))
}
fn signed(b: &mut Vec<u8>, n: i64) {
    put_varint(b, ((n as u64) << 1) ^ ((n >> 63) as u64));
}
fn signed_read(r: &mut Reader<'_>) -> Result<i64> {
    let n = r.var()?;
    Ok(((n >> 1) as i64) ^ -((n & 1) as i64))
}
fn optional<T>(b: &mut Vec<u8>, v: Option<&T>, f: impl FnOnce(&mut Vec<u8>, &T)) {
    b.push(u8::from(v.is_some()));
    if let Some(v) = v {
        f(b, v);
    }
}
#[derive(Clone, Debug)]
pub struct Origin {
    pub role: String,
    pub path: String,
    pub physical: Hash,
    pub bytes: u64,
}
#[derive(Clone, Debug)]
pub struct Corpus {
    pub manifest: CorpusManifest,
    pub train: Vec<Episode>,
    pub validation: Vec<Episode>,
    pub origins: Vec<Origin>,
    pub legacy: Option<CorpusManifest>,
    pub converted_at: u64,
    pub converter: Hash,
    pub semantic: Hash,
    pub physical: Hash,
}
fn manifest_encode(m: &CorpusManifest, b: &mut Vec<u8>) {
    put_varint(b, u64::from(m.version));
    for s in [&m.scope, &m.permission, &m.generator, &m.split_rule] {
        string(b, s);
    }
    put_varint(b, m.seed);
    for s in [&m.train, &m.validation] {
        string(b, &s.file);
        string(b, &s.sha256);
        put_varint(b, s.bytes as u64);
        put_varint(b, s.documents as u64);
        optional(b, s.tokens.as_ref(), |b, n| put_varint(b, *n as u64));
    }
}
fn manifest_decode(r: &mut Reader<'_>) -> Result<CorpusManifest> {
    let version = u32_read(r)?;
    let scope = text(r)?;
    let permission = text(r)?;
    let generator = text(r)?;
    let split_rule = text(r)?;
    let seed = r.var()?;
    let mut split = || -> Result<Split> {
        Ok(Split {
            file: text(r)?,
            sha256: text(r)?,
            bytes: count(r, MAX_FILE)?,
            documents: count(r, 100000)?,
            tokens: r.opt(|r| count(r, usize::MAX))?,
        })
    };
    Ok(CorpusManifest {
        version,
        scope,
        permission,
        generator,
        seed,
        split_rule,
        train: split()?,
        validation: split()?,
    })
}
pub fn ordered_bytes(episodes: &[Episode]) -> Vec<u8> {
    let mut b = b"R3CORP-ORDERED-EPISODES-v1\0".to_vec();
    put_varint(&mut b, episodes.len() as u64);
    for e in episodes {
        episode_encode(e, &mut b);
    }
    b
}
pub fn split(name: &str, episodes: &[Episode]) -> Split {
    let bytes = ordered_bytes(episodes);
    Split {
        file: name.into(),
        sha256: hash(&bytes),
        bytes: bytes.len(),
        documents: episodes.len(),
        tokens: None,
    }
}
fn semantic(m: &CorpusManifest, train: &[Episode], dev: &[Episode]) -> Hash {
    let mut b = b"R3CORP-SEMANTIC-v1\0".to_vec();
    for s in [&m.scope, &m.permission, &m.generator, &m.split_rule] {
        string(&mut b, s);
    }
    b.extend(m.version.to_le_bytes());
    b.extend(m.seed.to_le_bytes());
    b.extend(ordered_bytes(train));
    b.extend(ordered_bytes(dev));
    digest(&b)
}
pub fn from_episodes(
    mut m: CorpusManifest,
    train: Vec<Episode>,
    validation: Vec<Episode>,
) -> Result<Corpus> {
    validate_episodes(&train)?;
    validate_episodes(&validation)?;
    check_split(&train, &validation)?;
    m.train = split("train", &train);
    m.validation = split("validation", &validation);
    let semantic = semantic(&m, &train, &validation);
    Ok(Corpus {
        manifest: m,
        train,
        validation,
        origins: vec![],
        legacy: None,
        converted_at: replica_v3::event::now_ms()
            .try_into()
            .map_err(|_| bad("time"))?,
        converter: digest(include_bytes!("native_corpus.rs")),
        semantic,
        physical: [0; 32],
    })
}
pub fn import_legacy(source: &Path, output: &Path, compressed: bool) -> Result<()> {
    let (m, train, dev) = load_legacy(source)?;
    let mut c = from_episodes(m.clone(), train, dev)?;
    for (role, name) in [
        ("manifest", "manifest.r3b"),
        ("train", m.train.file.as_str()),
        ("validation", m.validation.file.as_str()),
    ] {
        let path = source.join(name);
        let bytes = read_bounded(&path, MAX_FILE)?;
        let expected = match role {
            "train" => Some((&m.train.sha256, m.train.bytes)),
            "validation" => Some((&m.validation.sha256, m.validation.bytes)),
            _ => None,
        };
        if expected.is_some_and(|(h, n)| hash(&bytes) != *h || bytes.len() != n) {
            return Err(bad("import source changed"));
        }
        if role == "manifest" {
            let observed: CorpusManifest = replica_v3::binary::from_slice(&bytes)?;
            let mut a = Vec::new();
            let mut b = Vec::new();
            manifest_encode(&observed, &mut a);
            manifest_encode(&m, &mut b);
            if a != b {
                return Err(bad("import manifest changed"));
            }
        }
        c.origins.push(Origin {
            role: role.into(),
            path: path.canonicalize()?.display().to_string(),
            physical: digest(&bytes),
            bytes: bytes.len() as u64,
        });
    }
    c.legacy = Some(m);
    write(output, &c, compressed)?;
    let reloaded = read(output)?;
    if ordered_bytes(&reloaded.train) != ordered_bytes(&c.train)
        || ordered_bytes(&reloaded.validation) != ordered_bytes(&c.validation)
    {
        return Err(bad("import logical equality"));
    }
    println!(
        "NATIVE_CORPUS_IMPORT=VERIFIED train={} dev={} semantic={} physical={} ORIGINALS_PRESERVED=true",
        c.train.len(),
        c.validation.len(),
        hash_text(&reloaded.semantic),
        hash_text(&reloaded.physical)
    );
    Ok(())
}
fn hash_text(h: &Hash) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn encode(c: &Corpus, compressed: bool) -> Result<Vec<u8>> {
    validate_episodes(&c.train)?;
    validate_episodes(&c.validation)?;
    check_split(&c.train, &c.validation)?;
    let mut meta = Vec::new();
    manifest_encode(&c.manifest, &mut meta);
    optional(&mut meta, c.legacy.as_ref(), |b, m| manifest_encode(m, b));
    meta.extend(c.converted_at.to_le_bytes());
    meta.extend(c.converter);
    put_varint(&mut meta, c.origins.len() as u64);
    for o in &c.origins {
        string(&mut meta, &o.role);
        string(&mut meta, &o.path);
        meta.extend(o.physical);
        meta.extend(o.bytes.to_le_bytes());
    }
    let mut blocks: Vec<Vec<u8>> = vec![Vec::new()];
    let mut index = Vec::new();
    for e in c.train.iter().chain(&c.validation) {
        let mut record = Vec::new();
        episode_encode(e, &mut record);
        if record.len() > MAX_RECORD {
            return Err(bad("record bound"));
        }
        if !blocks.last().unwrap().is_empty()
            && blocks.last().unwrap().len() + record.len() > BLOCK_TARGET
        {
            blocks.push(Vec::new());
        }
        index.push((blocks.len() - 1, blocks.last().unwrap().len(), record.len()));
        blocks.last_mut().unwrap().extend(record);
    }
    let header_len = PREFIX + meta.len() + blocks.len() * 57 + index.len() * 12;
    let mut directory = Vec::new();
    let mut payload = Vec::new();
    for raw in blocks.iter() {
        let z = if compressed {
            zstd::bulk::compress(raw, 3)?
        } else {
            Vec::new()
        };
        let codec = u8::from(compressed && z.len() < raw.len());
        let stored = if codec == 1 { &z } else { raw };
        directory.extend(((header_len + payload.len()) as u64).to_le_bytes());
        directory.extend((stored.len() as u64).to_le_bytes());
        directory.extend((raw.len() as u64).to_le_bytes());
        directory.push(codec);
        directory.extend(digest(raw));
        payload.extend(stored);
    }
    let mut b = vec![0; PREFIX];
    b[..8].copy_from_slice(b"R3CORP\0\0");
    b[8..10].copy_from_slice(&1u16.to_le_bytes());
    b[10] = 1;
    b[12..16].copy_from_slice(&(header_len as u32).to_le_bytes());
    b[16..24].copy_from_slice(&((header_len + payload.len()) as u64).to_le_bytes());
    for (at, n) in [
        (24, c.train.len()),
        (28, c.validation.len()),
        (32, blocks.len()),
        (36, index.len()),
    ] {
        b[at..at + 4].copy_from_slice(&(n as u32).to_le_bytes());
    }
    b[40..72].copy_from_slice(&semantic(&c.manifest, &c.train, &c.validation));
    b.extend(meta);
    b.extend(directory);
    for (block, offset, len) in index {
        for n in [block, offset, len] {
            b.extend((n as u32).to_le_bytes());
        }
    }
    let header_hash = digest(&b[PREFIX..]);
    b[72..104].copy_from_slice(&header_hash);
    b[104..136].copy_from_slice(&digest(&payload));
    b.extend(payload);
    if b.len() > MAX_FILE {
        return Err(bad("file bound"));
    }
    Ok(b)
}
fn fixed_u64(r: &mut Reader<'_>) -> Result<u64> {
    Ok(u64::from_le_bytes(r.take(8)?.try_into().unwrap()))
}
fn fixed_u32(r: &mut Reader<'_>) -> Result<usize> {
    Ok(u32::from_le_bytes(r.take(4)?.try_into().unwrap()) as usize)
}
pub fn decode(bytes: &[u8]) -> Result<Corpus> {
    if bytes.len() < PREFIX || bytes.len() > MAX_FILE {
        return Err(bad("file size"));
    }
    let mut prefix = Reader::new(&bytes[..PREFIX]);
    if prefix.take(8)? != b"R3CORP\0\0" || prefix.take(4)? != [1, 0, 1, 0] {
        return Err(bad("magic/version/endian/flags"));
    }
    let header = fixed_u32(&mut prefix)?;
    let total = fixed_u64(&mut prefix)?;
    let train_n = fixed_u32(&mut prefix)?;
    let dev_n = fixed_u32(&mut prefix)?;
    let block_n = fixed_u32(&mut prefix)?;
    let records = fixed_u32(&mut prefix)?;
    let sem: Hash = prefix.take(32)?.try_into().unwrap();
    let mh = prefix.take(32)?;
    let ph = prefix.take(32)?;
    if prefix.take(24)?.iter().any(|b| *b != 0)
        || total != bytes.len() as u64
        || !(PREFIX..=bytes.len()).contains(&header)
        || train_n == 0
        || dev_n == 0
        || train_n > 100000
        || dev_n > 100000
        || records != train_n + dev_n
        || block_n == 0
        || block_n > records
        || mh != digest(&bytes[PREFIX..header])
        || ph != digest(&bytes[header..])
    {
        return Err(bad("header/count/hash"));
    }
    let mut r = Reader::new(&bytes[PREFIX..header]);
    let manifest = manifest_decode(&mut r)?;
    let legacy = r.opt(manifest_decode)?;
    let converted_at = fixed_u64(&mut r)?;
    let converter = r.take(32)?.try_into().unwrap();
    let origin_n = count(&mut r, 64)?;
    let mut origins = Vec::with_capacity(origin_n);
    for _ in 0..origin_n {
        origins.push(Origin {
            role: text(&mut r)?,
            path: text(&mut r)?,
            physical: r.take(32)?.try_into().unwrap(),
            bytes: fixed_u64(&mut r)?,
        });
    }
    let mut blocks = Vec::new();
    let mut next = header;
    let mut raw_total = 0usize;
    for _ in 0..block_n {
        let offset = usize::try_from(fixed_u64(&mut r)?).map_err(|_| bad("offset"))?;
        let len = usize::try_from(fixed_u64(&mut r)?).map_err(|_| bad("length"))?;
        let raw_len = usize::try_from(fixed_u64(&mut r)?).map_err(|_| bad("raw length"))?;
        let codec = r.byte()?;
        let expected = r.take(32)?;
        next = next
            .checked_add(len)
            .filter(|n| *n <= bytes.len())
            .ok_or_else(|| bad("block extent"))?;
        raw_total = raw_total
            .checked_add(raw_len)
            .filter(|n| *n <= MAX_FILE)
            .ok_or_else(|| bad("raw allocation bound"))?;
        if offset != next - len || len == 0 || raw_len == 0 || raw_len > MAX_RECORD {
            return Err(bad("block offset/gap/length"));
        }
        let raw = match codec {
            0 if len == raw_len => bytes[offset..next].to_vec(),
            1 => {
                if zstd::zstd_safe::find_frame_compressed_size(&bytes[offset..next])
                    .map_err(|_| bad("zstd frame"))?
                    != len
                {
                    return Err(bad("zstd trailing"));
                }
                zstd::bulk::decompress(&bytes[offset..next], raw_len)?
            }
            _ => return Err(bad("codec/size")),
        };
        if raw.len() != raw_len || digest(&raw) != expected {
            return Err(bad("block digest/length"));
        }
        blocks.push(raw);
    }
    if next != bytes.len() {
        return Err(bad("trailing bytes"));
    }
    let mut episodes = Vec::with_capacity(records);
    let mut block = 0;
    let mut offset = 0;
    for _ in 0..records {
        let bi = fixed_u32(&mut r)?;
        let at = fixed_u32(&mut r)?;
        let len = fixed_u32(&mut r)?;
        if offset == blocks[block].len() {
            block += 1;
            offset = 0;
        }
        if bi != block || at != offset || block >= blocks.len() || len == 0 || len > MAX_RECORD {
            return Err(bad("record index/order"));
        }
        let end = at
            .checked_add(len)
            .filter(|n| *n <= blocks[block].len())
            .ok_or_else(|| bad("record extent"))?;
        let mut record = Reader::new(&blocks[block][at..end]);
        let e = episode_decode(&mut record)?;
        if !record.finished() {
            return Err(bad("record trailing"));
        }
        episodes.push(e);
        offset = end;
    }
    if !r.finished() {
        return Err(bad("header trailing"));
    }
    if block + 1 != blocks.len() || offset != blocks[block].len() {
        return Err(bad("unused records"));
    }
    let validation = episodes.split_off(train_n);
    let train = episodes;
    validate_episodes(&train)?;
    validate_episodes(&validation)?;
    check_split(&train, &validation)?;
    if manifest.version != 1
        || manifest.train.sha256 != split("train", &train).sha256
        || manifest.validation.sha256 != split("validation", &validation).sha256
        || manifest.train.documents != train_n
        || manifest.validation.documents != dev_n
        || sem != semantic(&manifest, &train, &validation)
    {
        return Err(bad("semantic/split identity"));
    }
    Ok(Corpus {
        manifest,
        train,
        validation,
        origins,
        legacy,
        converted_at,
        converter,
        semantic: sem,
        physical: digest(bytes),
    })
}
pub fn read(path: &Path) -> Result<Corpus> {
    decode(&read_bounded(path, MAX_FILE)?)
}
pub fn write(path: &Path, c: &Corpus, compressed: bool) -> Result<()> {
    let bytes = encode(c, compressed)?;
    publish_new(path, |file, _| {
        file.write_all(&bytes)?;
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0))?;
        let mut readback = Vec::new();
        file.take((MAX_FILE + 1) as u64)
            .read_to_end(&mut readback)?;
        let loaded = decode(&readback)?;
        if loaded.semantic != c.semantic {
            return Err(bad("publication semantic"));
        }
        Ok(())
    })
}
pub fn inspect(path: &Path) -> Result<()> {
    let c = read(path)?;
    println!(
        "R3CORP=VERIFIED bytes={} train={} dev={} semantic={} physical={} origins={} legacy_mapping={} JSON_READS=0",
        std::fs::metadata(path)?.len(),
        c.train.len(),
        c.validation.len(),
        hash_text(&c.semantic),
        hash_text(&c.physical),
        c.origins.len(),
        c.legacy.is_some()
    );
    Ok(())
}

pub(crate) fn episode_encode(e: &Episode, b: &mut Vec<u8>) {
    string(b, &e.id);
    put_varint(b, e.category as u64);
    for s in [
        &e.family,
        &e.binding,
        &e.sequence,
        &e.request.request_id,
        &e.request.system,
        &e.request.input,
        &e.answer,
    ] {
        string(b, s);
    }
    let limits = &e.request.limits;
    for n in [
        u64::from(limits.max_tokens),
        u64::from(limits.context_tokens),
        limits.timeout_ms,
    ] {
        put_varint(b, n);
    }
    let bundle = &e.request.evidence;
    b.push(u8::from(bundle.truncated));
    for n in [
        bundle.visited,
        bundle.candidates_fetched,
        bundle.edges_fetched,
        bundle.eligible,
    ] {
        put_varint(b, n as u64);
    }
    put_varint(b, bundle.items.len() as u64);
    for item in &bundle.items {
        signed(b, item.event_id);
        string(b, &item.original_excerpt);
        b.push(u8::from(item.excerpt_truncated));
        string(b, &item.source);
        signed(b, item.recorded_at);
        optional(b, item.observed_at.as_ref(), |b, n| signed(b, *n));
        string(b, &item.version_status);
        string(b, &item.retrieval_reason);
        put_varint(b, item.relation_path.len() as u64);
        for relation in &item.relation_path {
            signed(b, relation.from);
            signed(b, relation.to);
            b.push(relation.relation as u8);
            signed(b, relation.origin);
        }
    }
}
pub(crate) fn episode_decode(r: &mut Reader<'_>) -> Result<Episode> {
    let id = text(r)?;
    let category = count(r, 1024)?;
    let family = text(r)?;
    let binding = text(r)?;
    let sequence = text(r)?;
    let request_id = text(r)?;
    let system = text(r)?;
    let input = text(r)?;
    let answer = text(r)?;
    let limits = GenerationLimits {
        max_tokens: u32_read(r)?,
        context_tokens: u32_read(r)?,
        timeout_ms: r.var()?,
    };
    let truncated = r.bool()?;
    let visited = count(r, 1_000_000)?;
    let candidates_fetched = count(r, 1_000_000)?;
    let edges_fetched = count(r, 1_000_000)?;
    let eligible = count(r, 1_000_000)?;
    let n = count(r, 256)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        let event_id = signed_read(r)?;
        let original_excerpt = text(r)?;
        let excerpt_truncated = r.bool()?;
        let source = text(r)?;
        let recorded_at = signed_read(r)?;
        let observed_at = r.opt(signed_read)?;
        let version_status = text(r)?;
        let retrieval_reason = text(r)?;
        let count = count(r, 256)?;
        let mut relation_path = Vec::with_capacity(count);
        for _ in 0..count {
            relation_path.push(RelationStep {
                from: signed_read(r)?,
                to: signed_read(r)?,
                relation: RelationKind::from_tag(r.byte()?)?,
                origin: signed_read(r)?,
            });
        }
        items.push(Evidence {
            event_id,
            original_excerpt,
            excerpt_truncated,
            source,
            recorded_at,
            observed_at,
            version_status,
            retrieval_reason,
            relation_path,
        });
    }
    Ok(Episode {
        id,
        category,
        family,
        binding,
        sequence,
        answer,
        request: ModelRequest {
            request_id,
            system,
            input,
            limits,
            evidence: EvidenceBundle {
                items,
                truncated,
                visited,
                candidates_fetched,
                edges_fetched,
                eligible,
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Corpus {
        let mut train = synthetic(5, 71, false);
        let dev = synthetic(5, 71, true);
        train[0].request.input = "  0001 e\u{301} 한글 {\"original\":true}\n".into();
        train[0].request.request_id = "".into();
        train[0].request.evidence.items[0].recorded_at = i64::MIN;
        train[0].request.evidence.items[0].observed_at = Some(i64::MAX);
        train[1].request.evidence.items[0].observed_at = None;
        let m = CorpusManifest {
            version: 1,
            scope: "synthetic".into(),
            permission: "fixture".into(),
            generator: "native-golden-v1".into(),
            seed: u64::MAX,
            split_rule: "disjoint".into(),
            train: split("train", &train),
            validation: split("validation", &dev),
        };
        let mut c = from_episodes(m, train, dev).unwrap();
        c.converted_at = 0;
        c.converter = [7; 32];
        c
    }
    #[test]
    fn native_corpus_lossless_golden_raw_zstd_moved_and_legacy_mapping() {
        let c = fixture();
        let raw = encode(&c, false).unwrap();
        let cold = encode(&c, true).unwrap();
        assert_eq!(
            &raw[..12],
            &[b'R', b'3', b'C', b'O', b'R', b'P', 0, 0, 1, 0, 1, 0]
        );
        assert_eq!(&raw[24..32], &[5, 0, 0, 0, 5, 0, 0, 0]);
        assert!(cold.len() < raw.len());
        for bytes in [&raw, &cold] {
            let d = decode(bytes).unwrap();
            assert_eq!(ordered_bytes(&d.train), ordered_bytes(&c.train));
            assert_eq!(ordered_bytes(&d.validation), ordered_bytes(&c.validation));
            assert_eq!(d.semantic, c.semantic);
            assert_eq!(d.manifest.seed, u64::MAX);
            assert_eq!(d.train[0].request.evidence.items[0].recorded_at, i64::MIN);
            assert_eq!(
                d.train[0].request.evidence.items[0].observed_at,
                Some(i64::MAX)
            );
            assert_eq!(d.train[1].request.evidence.items[0].observed_at, None);
        }
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("legacy");
        std::fs::create_dir(&legacy).unwrap();
        let mut m = c.manifest.clone();
        m.train = save_split(&legacy, "train", &c.train).unwrap();
        m.validation = save_split(&legacy, "validation", &c.validation).unwrap();
        std::fs::write(
            legacy.join("manifest.r3b"),
            replica_v3::binary::to_vec(&m).unwrap(),
        )
        .unwrap();
        let native = dir.path().join("source.r3c");
        import_legacy(&legacy, &native, true).unwrap();
        // The origin is deliberately unavailable in the isolated reader.
        std::fs::rename(&legacy, dir.path().join("historical-preserved")).unwrap();
        let moved = dir.path().join("renamed.r3c");
        std::fs::rename(native, &moved).unwrap();
        let d = read(&moved).unwrap();
        assert!(d.legacy.is_some());
        assert_eq!(d.origins.len(), 3);
        assert_eq!(ordered_bytes(&d.train), ordered_bytes(&c.train));
        assert!(write(&moved, &c, false).is_err());
        assert!(load(&dir.path().join("historical-preserved")).is_err());
    }
    #[test]
    fn native_corpus_corrupt_bounds_offsets_duplicates_and_trailing() {
        let c = fixture();
        let raw = encode(&c, false).unwrap();
        for n in [0, 7, 159, raw.len() - 1] {
            assert!(decode(&raw[..n]).is_err());
        }
        let mut extra = raw.clone();
        extra.push(0);
        assert!(decode(&extra).is_err());
        for at in [
            8,
            10,
            11,
            12,
            16,
            24,
            28,
            32,
            36,
            40,
            72,
            104,
            136,
            raw.len() - 1,
        ] {
            let mut b = raw.clone();
            b[at] ^= 0xff;
            assert!(decode(&b).is_err(), "{at}");
        }
        let mut duplicate = c.clone();
        duplicate.train[1].id = duplicate.train[0].id.clone();
        assert!(encode(&duplicate, false).is_err());
        let header = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
        let blocks = u32::from_le_bytes(raw[32..36].try_into().unwrap()) as usize;
        let records = u32::from_le_bytes(raw[36..40].try_into().unwrap()) as usize;
        let directory = header - blocks * 57 - records * 12;
        for at in [
            directory,
            directory + 8,
            directory + 16,
            directory + 24,
            header - records * 12,
            header - records * 12 + 4,
            header - records * 12 + 8,
        ] {
            let mut b = raw.clone();
            b[at] ^= 0xff;
            let h = digest(&b[PREFIX..header]);
            b[72..104].copy_from_slice(&h);
            assert!(decode(&b).is_err(), "valid-checksum malformed index {at}");
        }
        assert!(decode(b"{\"train\":[]}").is_err());
    }
}
