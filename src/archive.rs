//! Read-only canonical evidence snapshot. The reader never opens SQLite.
//! Export pins one SQLite read transaction and preserves each original id/body pair.
use crate::{
    Error, Result,
    codec::{self, Compression, publish_new},
    event::*,
    retrieval::{
        self, CANDIDATES, EvidenceBundle, EvidenceRead, GraphDirection, RelationStep, Search,
        VISITED,
    },
    store::{self, Store},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    time::{Duration, Instant},
};
const MAGIC: &[u8; 8] = b"R3ARCH\0\0";
const PREFIX: usize = 144;
pub const BLOCK_BYTES: usize = 2 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;
const MAX_CANONICAL: u64 = 256 * 1024 * 1024;
const MAX_DECODED: u64 = 512 * 1024 * 1024;
const MAX_DIRECTORY: usize = 8 * 1024 * 1024;
const MAX_FILE: u64 = MAX_CANONICAL + MAX_DIRECTORY as u64 + PREFIX as u64;
const BLOCK_ENTRY: usize = 52;
const RECORD_ENTRY: usize = 20;
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ArchiveInfo {
    pub snapshot_id: i64,
    pub snapshot_recorded_at: i64,
    pub events: usize,
    pub blocks: usize,
    pub canonical_bytes: u64,
    pub decoded_event_bytes: u64,
    pub original_payload_bytes: u64,
    pub stored_block_bytes: u64,
    pub prefix_bytes: usize,
    pub directory_bytes: usize,
    pub record_index_bytes: usize,
    pub compression_dictionary_bytes: usize,
    pub file_bytes: u64,
    pub source_identity: String,
    pub compressed_blocks: usize,
    pub canonical_codec_version: u8,
}
#[derive(Clone)]
struct Block {
    offset: u64,
    stored: usize,
    raw: usize,
    codec: u8,
    digest: [u8; 32],
}
#[derive(Clone, Copy)]
struct Location {
    block: usize,
    offset: usize,
    length: usize,
}
struct Meta {
    scope: String,
    session: String,
    recorded: i64,
    slot: Option<Vec<u8>>,
    kind: u8,
    question: bool,
    from: Option<i64>,
    until: Option<i64>,
}
#[derive(Default)]
struct BlockCache {
    block: Option<usize>,
    bytes: Vec<u8>,
    read_bytes: u64,
    decoded_bytes: u64,
}
pub struct Archive {
    file: RefCell<File>,
    blocks: Vec<Block>,
    index: BTreeMap<i64, Location>,
    meta: BTreeMap<i64, Meta>,
    versions: BTreeMap<Vec<u8>, Vec<i64>>,
    edges: BTreeMap<i64, Vec<RelationStep>>,
    requests: BTreeSet<(String, [u8; 16])>,
    results: BTreeSet<i64>,
    overlay: BTreeMap<i64, Vec<u8>>,
    overlay_decoded: u64,
    overlay_bytes: u64,
    cache: RefCell<BlockCache>,
    pub info: ArchiveInfo,
    pub index_rebuild_ms: f64,
}
fn bad(s: &str) -> Error {
    Error::Corrupt(format!("evidence archive: {s}"))
}
fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().expect("bounded field"))
}
fn u64_at(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(b[i..i + 8].try_into().expect("bounded field"))
}
fn i64_at(b: &[u8], i: usize) -> i64 {
    i64::from_le_bytes(b[i..i + 8].try_into().expect("bounded field"))
}
fn canonical_hash(h: &mut Sha256, id: i64, body: &[u8]) {
    h.update(id.to_le_bytes());
    h.update((body.len() as u64).to_le_bytes());
    h.update(body);
}
fn directory_digest(prefix: &[u8; PREFIX], directory: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&prefix[..104]);
    h.update(&prefix[136..]);
    h.update(directory);
    h.finalize().into()
}
fn flush_block(
    file: &mut File,
    raw: &mut Vec<u8>,
    blocks: &mut Vec<Block>,
    level: Option<i32>,
) -> Result<()> {
    if raw.is_empty() {
        return Ok(());
    }
    let compressed = level
        .map(|level| zstd::bulk::compress(raw, level))
        .transpose()?;
    let (codec, stored) = match &compressed {
        Some(c) if c.len() < raw.len() => (1, c.as_slice()),
        _ => (0, raw.as_slice()),
    };
    let offset = file.stream_position()?;
    file.write_all(stored)?;
    blocks.push(Block {
        offset,
        stored: stored.len(),
        raw: raw.len(),
        codec,
        digest: Sha256::digest(raw.as_slice()).into(),
    });
    raw.clear();
    Ok(())
}
impl Store {
    pub fn export_archive(&self, path: &Path, compression: Compression) -> Result<ArchiveInfo> {
        self.export_archive_profile(
            path,
            BLOCK_BYTES,
            matches!(compression, Compression::Auto).then_some(3),
        )
    }
    pub fn export_archive_profile(
        &self,
        path: &Path,
        block_bytes: usize,
        level: Option<i32>,
    ) -> Result<ArchiveInfo> {
        if ![64 * 1024, 256 * 1024, BLOCK_BYTES].contains(&block_bytes)
            || level.is_some_and(|v| ![1, 3].contains(&v))
        {
            return Err(Error::Invalid("archive block/compression profile".into()));
        }
        let tx = self.conn.unchecked_transaction()?;
        let (count, bound): (i64, i64) = tx.query_row(
            "SELECT count(*),coalesce(max(id),0) FROM records",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if count < 0 || count as usize > MAX_RECORDS {
            return Err(Error::Invalid("archive event count bound".into()));
        }
        #[cfg(feature = "test-support")]
        store::sync_point("archive_snapshot")?;
        publish_new(path, |file, temporary| {
            file.write_all(&[0u8; PREFIX])?;
            let mut blocks = Vec::new();
            let mut index = Vec::with_capacity(count as usize);
            let mut raw = Vec::with_capacity(block_bytes);
            let mut canonical_bytes = 0u64;
            let mut decoded_bytes = 0u64;
            let mut payload_bytes = 0u64;
            let mut digest = Sha256::new();
            let mut last_time = i64::MIN;
            let mut last_id = 0;
            let mut stmt = tx.prepare("SELECT id,body FROM records ORDER BY id")?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let body = match row.get_ref(1)? {
                    rusqlite::types::ValueRef::Blob(body) => body,
                    _ => return Err(bad("canonical body SQL type")),
                };
                if body.len() > codec::MAX_BODY + codec::HEADER {
                    return Err(bad("record byte bound"));
                }
                if body.len() < codec::HEADER {
                    return Err(bad("truncated event envelope"));
                }
                decoded_bytes += u64::from(u32_at(body, 8));
                if decoded_bytes > MAX_DECODED {
                    return Err(bad("decoded event total budget"));
                }
                let event = codec::decode(body)?;
                if id != event.id || id <= last_id || event.recorded_at < last_time {
                    return Err(bad("canonical ordering"));
                }
                last_time = event.recorded_at;
                last_id = id;
                canonical_bytes = canonical_bytes
                    .checked_add(body.len() as u64)
                    .ok_or_else(|| bad("canonical overflow"))?;
                if canonical_bytes > MAX_CANONICAL {
                    return Err(Error::Invalid("archive decoded byte budget".into()));
                }
                payload_bytes += event.payload.len() as u64;
                canonical_hash(&mut digest, id, body);
                if raw.len() + body.len() > block_bytes {
                    flush_block(file, &mut raw, &mut blocks, level)?;
                }
                index.push((
                    id,
                    Location {
                        block: blocks.len(),
                        offset: raw.len(),
                        length: body.len(),
                    },
                ));
                raw.extend_from_slice(body);
            }
            flush_block(file, &mut raw, &mut blocks, level)?;
            if index.len() != count as usize || last_id != bound {
                return Err(bad("snapshot changed"));
            }
            let directory_offset = file.stream_position()?;
            let mut directory =
                Vec::with_capacity(blocks.len() * BLOCK_ENTRY + index.len() * RECORD_ENTRY);
            for b in &blocks {
                directory.extend_from_slice(&b.offset.to_le_bytes());
                directory.extend_from_slice(&(b.stored as u32).to_le_bytes());
                directory.extend_from_slice(&(b.raw as u32).to_le_bytes());
                directory.push(b.codec);
                directory.extend_from_slice(&[0; 3]);
                directory.extend_from_slice(&b.digest);
            }
            for (id, l) in &index {
                directory.extend_from_slice(&id.to_le_bytes());
                for n in [l.block, l.offset, l.length] {
                    directory.extend_from_slice(&(n as u32).to_le_bytes());
                }
            }
            if directory.len() > MAX_DIRECTORY {
                return Err(bad("directory budget"));
            }
            file.write_all(&directory)?;
            let total = file.stream_position()?;
            let mut prefix = [0u8; PREFIX];
            prefix[..8].copy_from_slice(MAGIC);
            prefix[8..10].copy_from_slice(&1u16.to_le_bytes());
            prefix[10] = u8::from(level.is_some());
            prefix[11] = 2; // Schema2 also reads unchanged schema1 canonical bodies.
            prefix[12..16].copy_from_slice(&(directory.len() as u32).to_le_bytes());
            prefix[16..24].copy_from_slice(&total.to_le_bytes());
            prefix[24..32].copy_from_slice(&directory_offset.to_le_bytes());
            prefix[32..40].copy_from_slice(&bound.to_le_bytes());
            prefix[40..48].copy_from_slice(&last_time.to_le_bytes());
            prefix[48..52].copy_from_slice(&(count as u32).to_le_bytes());
            prefix[52..56].copy_from_slice(&(blocks.len() as u32).to_le_bytes());
            prefix[56..64].copy_from_slice(&canonical_bytes.to_le_bytes());
            prefix[64..72].copy_from_slice(&payload_bytes.to_le_bytes());
            prefix[72..104].copy_from_slice(&digest.finalize());
            prefix[136..144].copy_from_slice(&decoded_bytes.to_le_bytes());
            let digest = directory_digest(&prefix, &directory);
            prefix[104..136].copy_from_slice(&digest);
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&prefix)?;
            // The full reader rebuilds indexes and validates canonical linkage before publication.
            let archive = Archive::open(temporary)?;
            Ok(archive.info)
        })
    }
}
impl Archive {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path)?;
        let total = file.metadata()?.len();
        if !(PREFIX as u64..=MAX_FILE).contains(&total) {
            return Err(bad("file size"));
        }
        let mut p = [0u8; PREFIX];
        file.read_exact(&mut p)?;
        if &p[..8] != MAGIC || p[8..10] != 1u16.to_le_bytes() || p[10] > 1 || p[11] != 2 {
            return Err(bad("magic/version/flags"));
        }
        let n = u32_at(&p, 12) as usize;
        let directory_offset = u64_at(&p, 24);
        let count = u32_at(&p, 48) as usize;
        let block_count = u32_at(&p, 52) as usize;
        let canonical_bytes = u64_at(&p, 56);
        let payload_bytes = u64_at(&p, 64);
        let decoded_bytes = u64_at(&p, 136);
        if u64_at(&p, 16) != total
            || n > MAX_DIRECTORY
            || count > MAX_RECORDS
            || block_count > count
            || canonical_bytes > MAX_CANONICAL
            || decoded_bytes > MAX_DECODED
            || payload_bytes > decoded_bytes
            || directory_offset < PREFIX as u64
            || directory_offset.checked_add(n as u64) != Some(total)
            || n != block_count * BLOCK_ENTRY + count * RECORD_ENTRY
        {
            return Err(bad("directory/count/total bounds"));
        }
        let mut directory = vec![0; n];
        file.seek(SeekFrom::Start(directory_offset))?;
        file.read_exact(&mut directory)?;
        if directory_digest(&p, &directory)[..] != p[104..136] {
            return Err(bad("directory checksum"));
        }
        let mut blocks = Vec::with_capacity(block_count);
        let mut next = PREFIX as u64;
        let mut raw_sum = 0u64;
        for b in directory[..block_count * BLOCK_ENTRY]
            .as_chunks::<BLOCK_ENTRY>()
            .0
        {
            let block = Block {
                offset: u64_at(b, 0),
                stored: u32_at(b, 8) as usize,
                raw: u32_at(b, 12) as usize,
                codec: b[16],
                digest: b[20..52].try_into().expect("bounded block"),
            };
            if block.offset != next
                || block.stored == 0
                || block.stored > BLOCK_BYTES
                || block.raw == 0
                || block.raw > BLOCK_BYTES
                || block.codec > 1
                || b[17..20].iter().any(|&v| v != 0)
                || (block.codec == 0 && block.stored != block.raw)
                || (p[10] == 0 && block.codec != 0)
            {
                return Err(bad("block bounds/codec/overlap"));
            }
            next = next
                .checked_add(block.stored as u64)
                .ok_or_else(|| bad("block offset overflow"))?;
            raw_sum += block.raw as u64;
            if next > directory_offset || raw_sum > MAX_CANONICAL {
                return Err(bad("block totals"));
            }
            blocks.push(block);
        }
        if next != directory_offset
            || raw_sum != canonical_bytes
            || (count == 0) != (block_count == 0)
        {
            return Err(bad("block directory coverage"));
        }
        let mut index = BTreeMap::new();
        let mut covered = vec![0usize; block_count];
        let mut last_id = 0;
        let mut last_block = 0;
        for e in directory[block_count * BLOCK_ENTRY..]
            .as_chunks::<RECORD_ENTRY>()
            .0
        {
            let id = i64_at(e, 0);
            let l = Location {
                block: u32_at(e, 8) as usize,
                offset: u32_at(e, 12) as usize,
                length: u32_at(e, 16) as usize,
            };
            let block = blocks
                .get(l.block)
                .ok_or_else(|| bad("record block index"))?;
            if id <= last_id
                || l.block < last_block
                || l.length < codec::HEADER
                || l.length > codec::MAX_BODY + codec::HEADER
                || l.offset != covered[l.block]
                || l.offset.checked_add(l.length).is_none_or(|n| n > block.raw)
            {
                return Err(bad("record ID/offset/length/overlap"));
            }
            covered[l.block] += l.length;
            last_id = id;
            last_block = l.block;
            index.insert(id, l);
        }
        if last_id != i64_at(&p, 32) || covered.iter().zip(&blocks).any(|(&n, b)| n != b.raw) {
            return Err(bad("record index coverage/snapshot"));
        }
        let info = ArchiveInfo {
            snapshot_id: last_id,
            snapshot_recorded_at: i64_at(&p, 40),
            events: count,
            blocks: block_count,
            canonical_bytes,
            decoded_event_bytes: decoded_bytes,
            original_payload_bytes: payload_bytes,
            stored_block_bytes: directory_offset - PREFIX as u64,
            prefix_bytes: PREFIX,
            directory_bytes: block_count * BLOCK_ENTRY,
            record_index_bytes: count * RECORD_ENTRY,
            compression_dictionary_bytes: 0,
            file_bytes: total,
            source_identity: format!(
                "{:x}",
                sha2::digest::Output::<Sha256>::from_slice(&p[72..104])
            ),
            compressed_blocks: blocks.iter().filter(|b| b.codec == 1).count(),
            canonical_codec_version: p[11],
        };
        let mut archive = Self {
            file: RefCell::new(file),
            blocks,
            index,
            meta: BTreeMap::new(),
            versions: BTreeMap::new(),
            edges: BTreeMap::new(),
            requests: BTreeSet::new(),
            results: BTreeSet::new(),
            overlay: BTreeMap::new(),
            overlay_decoded: 0,
            overlay_bytes: 0,
            cache: RefCell::new(BlockCache::default()),
            info,
            index_rebuild_ms: 0.,
        };
        archive.rebuild_indexes()?;
        Ok(archive)
    }
    fn rebuild_indexes(&mut self) -> Result<()> {
        let start = Instant::now();
        self.meta.clear();
        self.versions.clear();
        self.edges.clear();
        let mut digest = Sha256::new();
        let mut previous_time = i64::MIN;
        let mut payload_bytes = 0u64;
        let mut decoded_bytes = 0u64;
        self.requests.clear();
        self.results.clear();
        for id in self.index.keys().copied().collect::<Vec<_>>() {
            let body = self.canonical_body(id)?;
            decoded_bytes += u64::from(u32_at(&body, 8));
            if decoded_bytes > self.info.decoded_event_bytes {
                return Err(bad("declared decoded event budget"));
            }
            canonical_hash(&mut digest, id, &body);
            let e = codec::decode(&body)?;
            if e.id != id || e.recorded_at < previous_time {
                return Err(bad("canonical event identity/order"));
            }
            previous_time = e.recorded_at;
            payload_bytes += e.payload.len() as u64;
            let slot = e.kind.slot().map(|s| s.key(&e.scope));
            let current = slot
                .as_ref()
                .and_then(|s| self.versions.get(s))
                .and_then(|v| v.last())
                .copied();
            store::validate_links_with(&e, |id| self.get(id), current)
                .map_err(|e| bad(&e.to_string()))?;
            if e.request_key
                .is_some_and(|key| !self.requests.insert((e.scope.clone(), key)))
                || e.kind
                    .input()
                    .is_some_and(|input| !self.results.insert(input))
            {
                return Err(bad("duplicate request/result"));
            }
            self.project(&e);
        }
        if format!("{:x}", digest.finalize()) != self.info.source_identity
            || previous_time != self.info.snapshot_recorded_at
            || payload_bytes != self.info.original_payload_bytes
            || decoded_bytes != self.info.decoded_event_bytes
        {
            return Err(bad("source digest/payload/snapshot time"));
        }
        for edges in self.edges.values_mut() {
            edges.sort_by_key(|e| (e.origin, e.from, e.to, e.relation as u8));
        }
        self.index_rebuild_ms = start.elapsed().as_secs_f64() * 1000.;
        Ok(())
    }
    fn project(&mut self, e: &Event) {
        let id = e.id;
        let slot = e.kind.slot().map(|s| s.key(&e.scope));
        for (from, to, relation) in e.edges() {
            let edge = RelationStep {
                from,
                to,
                relation,
                origin: e.id,
            };
            for id in [Some(from), (to != from).then_some(to)]
                .into_iter()
                .flatten()
            {
                let edges = self.edges.entry(id).or_default();
                let key = |e: &RelationStep| (e.origin, e.from, e.to, e.relation as u8);
                let at = edges.partition_point(|old| key(old) <= key(&edge));
                edges.insert(at, edge.clone());
            }
        }
        if let Some(key) = &slot {
            self.versions.entry(key.clone()).or_default().push(id);
        }
        let (from, until) = e.kind.validity();
        self.meta.insert(
            id,
            Meta {
                scope: e.scope.clone(),
                session: e.session.clone(),
                recorded: e.recorded_at,
                slot,
                kind: e.kind.tag(),
                question: matches!(e.kind, Kind::Observation { question: Some(_) }),
                from,
                until,
            },
        );
    }
    pub fn event_count(&self) -> usize {
        self.meta.len()
    }
    pub fn ids(&self) -> impl Iterator<Item = i64> + '_ {
        self.meta.keys().copied()
    }
    // Validate a bounded transaction without changing the committed graph view.
    pub(crate) fn prepare_overlay(&self, bodies: &[Vec<u8>]) -> Result<Vec<Event>> {
        if bodies.is_empty() || bodies.len() > 256 || self.meta.len() + bodies.len() > MAX_RECORDS {
            return Err(bad("overlay count"));
        }
        let mut pending = BTreeMap::<i64, Event>::new();
        let mut heads = BTreeMap::new();
        let mut requests = BTreeSet::new();
        let mut results = BTreeSet::new();
        let mut previous = self
            .meta
            .last_key_value()
            .map(|(&id, m)| (id, m.recorded))
            .unwrap_or((0, i64::MIN));
        let mut raw = self.info.canonical_bytes + self.overlay_bytes;
        let mut decoded = self.info.decoded_event_bytes + self.overlay_decoded;
        for body in bodies {
            let e = codec::decode(body)?;
            raw += body.len() as u64;
            decoded += u64::from(u32_at(body, 8));
            if raw > MAX_CANONICAL
                || decoded > MAX_DECODED
                || e.id <= previous.0
                || e.recorded_at < previous.1
            {
                return Err(bad("overlay order/byte bound"));
            }
            let slot = e.kind.slot().map(|s| s.key(&e.scope));
            let current = slot.as_ref().and_then(|key| {
                heads
                    .get(key)
                    .copied()
                    .or_else(|| self.versions.get(key).and_then(|v| v.last()).copied())
            });
            store::validate_links_with(
                &e,
                |id| {
                    pending
                        .get(&id)
                        .cloned()
                        .map(Ok)
                        .unwrap_or_else(|| self.get(id))
                },
                current,
            )?;
            if let Some(key) = e.request_key {
                let key = (e.scope.clone(), key);
                if self.requests.contains(&key) || !requests.insert(key) {
                    return Err(bad("duplicate event request"));
                }
            }
            if let Some(input) = e.kind.input()
                && (self.results.contains(&input) || !results.insert(input))
            {
                return Err(bad("duplicate result"));
            }
            if let Some(slot) = slot {
                heads.insert(slot, e.id);
            }
            previous = (e.id, e.recorded_at);
            pending.insert(e.id, e);
        }
        Ok(pending.into_values().collect())
    }
    pub(crate) fn apply_overlay(&mut self, events: Vec<Event>, bodies: Vec<Vec<u8>>) {
        for (e, body) in events.into_iter().zip(bodies) {
            self.project(&e);
            if let Some(key) = e.request_key {
                self.requests.insert((e.scope.clone(), key));
            }
            if let Some(input) = e.kind.input() {
                self.results.insert(input);
            }
            self.overlay_decoded += u64::from(u32_at(&body, 8));
            self.overlay_bytes += body.len() as u64;
            self.overlay.insert(e.id, body);
        }
    }
    pub fn io_counts(&self) -> (u64, u64) {
        let c = self.cache.borrow();
        (c.read_bytes, c.decoded_bytes)
    }
    /// Complete derived edge inventory, bounded by the archive event/reference limits.
    pub fn relation_inventory(&self) -> impl Iterator<Item = &RelationStep> {
        self.edges
            .iter()
            .flat_map(|(&id, edges)| edges.iter().filter(move |edge| edge.from == id))
    }
    pub fn canonical_body(&self, id: i64) -> Result<Vec<u8>> {
        if let Some(body) = self.overlay.get(&id) {
            return Ok(body.clone());
        }
        let l = self
            .index
            .get(&id)
            .ok_or_else(|| Error::NotFound(format!("archive event {id}")))?;
        let mut c = self.cache.borrow_mut();
        if c.block != Some(l.block) {
            let block = &self.blocks[l.block];
            let mut stored = vec![0u8; block.stored];
            let mut file = self.file.borrow_mut();
            file.seek(SeekFrom::Start(block.offset))?;
            file.read_exact(&mut stored)?;
            let raw = if block.codec == 0 {
                stored
            } else {
                let frame = zstd::zstd_safe::find_frame_compressed_size(&stored)
                    .map_err(|_| bad("zstd frame"))?;
                if frame != stored.len() {
                    return Err(bad("trailing zstd frame"));
                }
                let mut decoder = zstd::stream::read::Decoder::new(stored.as_slice())
                    .map_err(|_| bad("zstd header"))?;
                decoder.window_log_max(21).map_err(|_| bad("zstd window"))?;
                let mut raw = Vec::with_capacity(block.raw);
                decoder
                    .take(block.raw as u64 + 1)
                    .read_to_end(&mut raw)
                    .map_err(|_| bad("zstd expansion"))?;
                raw
            };
            if raw.len() != block.raw || Sha256::digest(&raw)[..] != block.digest {
                return Err(bad("block checksum/decoded length"));
            }
            c.read_bytes += block.stored as u64;
            c.decoded_bytes += block.raw as u64;
            c.bytes = raw;
            c.block = Some(l.block);
        }
        Ok(c.bytes[l.offset..l.offset + l.length].to_vec())
    }
    pub fn get(&self, id: i64) -> Result<Event> {
        let e = codec::decode(&self.canonical_body(id)?)?;
        if e.id != id {
            return Err(bad("lookup ID"));
        }
        Ok(e)
    }
    pub fn history(&self, scope: &str, slot: &Slot) -> Result<Vec<Event>> {
        check_text(scope)?;
        self.versions
            .get(&slot.key(scope))
            .into_iter()
            .flatten()
            .map(|&id| self.get(id))
            .collect()
    }
    fn latest(&self, key: &[u8], snapshot: i64, before: i64) -> Option<i64> {
        self.versions
            .get(key)?
            .iter()
            .rev()
            .find(|&&id| id <= snapshot && self.meta[&id].recorded <= before)
            .copied()
    }
    pub fn current(
        &self,
        scope: &str,
        slot: &Slot,
        as_of: Option<i64>,
        valid_at: i64,
    ) -> Result<Option<Event>> {
        check_text(scope)?;
        let e = self
            .latest(
                &slot.key(scope),
                self.meta.last_key_value().map_or(0, |(&id, _)| id),
                as_of.unwrap_or(i64::MAX),
            )
            .map(|id| self.get(id))
            .transpose()?;
        Ok(e.filter(|e| store::valid_at_time(e, valid_at)))
    }
    pub fn directed_graph(
        &self,
        q: &Search,
        seeds: &[i64],
        direction: GraphDirection,
    ) -> Result<EvidenceBundle> {
        retrieval::graph_bounds(q, seeds)?;
        retrieval::traverse(
            &ArchiveEvidence(self, direction),
            q,
            seeds.to_vec(),
            EvidenceBundle::default(),
            Instant::now() + Duration::from_millis(100),
            "graph_seed",
        )
    }
    pub fn search(&self, q: &Search) -> Result<EvidenceBundle> {
        check_text(&q.scope)?;
        if q.query.is_empty()
            || q.query.len() > 4096
            || q.query.contains('\0')
            || q.after.zip(q.before).is_some_and(|(a, b)| a > b)
        {
            return Err(Error::Invalid("malformed search query/range".into()));
        }
        let terms: Vec<_> = q.query.split_whitespace().collect();
        if terms.is_empty() || terms.len() > 32 {
            return Err(Error::Invalid("search requires 1..32 literal terms".into()));
        }
        if terms.iter().any(|s| s.chars().count() >= 3) {
            return Err(Error::Unsupported(
                "archive does not implement SQLite FTS5 trigram/BM25".into(),
            ));
        }
        let source = ArchiveEvidence(self, GraphDirection::Both);
        let deadline = Instant::now() + Duration::from_millis(100);
        let mut seeds = Vec::new();
        let mut bundle = EvidenceBundle::default();
        for &id in self.meta.keys().rev() {
            if Instant::now() >= deadline {
                bundle.truncated = true;
                break;
            }
            if source.eligible(id, q)? {
                seeds.push(id);
                if seeds.len() > CANDIDATES {
                    return Err(Error::NarrowScope);
                }
            }
        }
        bundle.candidates_fetched = seeds.len();
        let lower = q.query.to_lowercase();
        let mut matching = Vec::new();
        for id in seeds {
            if store::normalized(&self.get(id)?).contains(&lower) {
                matching.push(id);
            }
        }
        retrieval::traverse(&source, q, matching, bundle, deadline, "lexical")
    }
}
struct ArchiveEvidence<'a>(&'a Archive, GraphDirection);
impl EvidenceRead for ArchiveEvidence<'_> {
    fn get(&self, id: i64) -> Result<Event> {
        self.0.get(id)
    }
    fn eligible(&self, id: i64, q: &Search) -> Result<bool> {
        let Some(m) = self.0.meta.get(&id) else {
            return Ok(false);
        };
        if m.scope != q.scope
            || q.session.as_ref().is_some_and(|s| *s != m.session)
            || q.slot
                .as_ref()
                .is_some_and(|s| m.slot.as_ref() != Some(&s.key(&q.scope)))
            || m.recorded < q.after.unwrap_or(i64::MIN)
            || m.recorded > q.before.unwrap_or(i64::MAX)
            || id > q.snapshot_id.unwrap_or(i64::MAX)
        {
            return Ok(false);
        }
        if q.memory_only && (!matches!(m.kind, 0 | 1 | 2 | 5) || m.question) {
            return Ok(false);
        }
        if !q.history {
            if !matches!(m.kind, 0..=2) || m.question {
                return Ok(false);
            }
            if m.kind == 1
                && (self.0.latest(
                    m.slot.as_ref().expect("fact slot"),
                    q.snapshot_id.unwrap_or(i64::MAX),
                    q.before.unwrap_or(i64::MAX),
                ) != Some(id)
                    || m.from.is_some_and(|t| t > q.valid_at)
                    || m.until.is_some_and(|t| t <= q.valid_at))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn last(&self, e: &Event, q: &Search) -> Result<i64> {
        self.0
            .latest(
                &e.kind
                    .slot()
                    .expect("slot checked by traversal")
                    .key(&e.scope),
                q.snapshot_id.unwrap_or(i64::MAX),
                q.before.unwrap_or(i64::MAX),
            )
            .ok_or_else(|| bad("version index"))
    }
    fn edges(&self, id: i64, q: &Search) -> Result<Vec<RelationStep>> {
        Ok(self
            .0
            .edges
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|e| {
                let m = &self.0.meta[&e.origin];
                self.1.matches(id, e.from, e.to)
                    && e.origin <= q.snapshot_id.unwrap_or(i64::MAX)
                    && m.scope == q.scope
                    && q.session.as_ref().is_none_or(|s| *s == m.session)
                    && m.recorded >= q.after.unwrap_or(i64::MIN)
                    && m.recorded <= q.before.unwrap_or(i64::MAX)
            })
            .take(VISITED + 1)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Existing v3 independently calculated canonical golden, not encode() output.
    const EVENT: &[u8] = &[
        82, 80, 86, 51, 1, 0, 0, 0, 13, 0, 0, 0, 13, 0, 0, 0, 46, 98, 208, 165, 1, 0, 0, 1, 117, 1,
        115, 1, 116, 0, 0, 0, 0,
    ];
    fn literal() -> Vec<u8> {
        let mut prefix = [0u8; 144];
        prefix[..12].copy_from_slice(&[82, 51, 65, 82, 67, 72, 0, 0, 1, 0, 0, 2]);
        prefix[12..16].copy_from_slice(&72u32.to_le_bytes());
        prefix[16..24].copy_from_slice(&249u64.to_le_bytes());
        prefix[24..32].copy_from_slice(&177u64.to_le_bytes());
        prefix[32..40].copy_from_slice(&1i64.to_le_bytes());
        prefix[48..52].copy_from_slice(&1u32.to_le_bytes());
        prefix[52..56].copy_from_slice(&1u32.to_le_bytes());
        prefix[56..64].copy_from_slice(&33u64.to_le_bytes());
        prefix[136..144].copy_from_slice(&13u64.to_le_bytes());
        let canonical = [
            1i64.to_le_bytes().as_slice(),
            33u64.to_le_bytes().as_slice(),
            EVENT,
        ]
        .concat();
        prefix[72..104].copy_from_slice(&Sha256::digest(&canonical));
        let mut directory = Vec::new();
        directory.extend_from_slice(&144u64.to_le_bytes());
        directory.extend_from_slice(&33u32.to_le_bytes());
        directory.extend_from_slice(&33u32.to_le_bytes());
        directory.extend_from_slice(&[0; 4]);
        directory.extend_from_slice(&Sha256::digest(EVENT));
        directory.extend_from_slice(&1i64.to_le_bytes());
        directory.extend_from_slice(&[0; 8]);
        directory.extend_from_slice(&33u32.to_le_bytes());
        let mut hash = Sha256::new();
        hash.update(&prefix[..104]);
        hash.update(&prefix[136..144]);
        hash.update(&directory);
        prefix[104..136].copy_from_slice(&hash.finalize());
        [prefix.as_slice(), EVENT, directory.as_slice()].concat()
    }
    fn rehash(bytes: &mut [u8]) {
        let offset = u64_at(bytes, 24) as usize;
        let mut hash = Sha256::new();
        hash.update(&bytes[..104]);
        hash.update(&bytes[136..144]);
        hash.update(&bytes[offset..]);
        bytes[104..136].copy_from_slice(&hash.finalize());
    }
    #[test]
    fn literal_archive_decode_and_independent_encoder_bytes() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("literal.r3a");
        let bytes = literal();
        std::fs::write(&path, &bytes).unwrap();
        let archive = Archive::open(&path).unwrap();
        assert_eq!(archive.canonical_body(1).unwrap(), EVENT);
        assert_eq!(archive.get(1).unwrap(), codec::decode(EVENT).unwrap());
        assert_eq!(archive.info.file_bytes, 249);
        assert_eq!(archive.info.decoded_event_bytes, 13);
        let db = d.path().join("source.db");
        let mut store = Store::init(&db).unwrap();
        store
            .conn
            .execute("INSERT INTO records(id,body) VALUES(1,?1)", [EVENT])
            .unwrap();
        store.reindex().unwrap();
        store.doctor(true).unwrap();
        let exported = d.path().join("export.r3a");
        store.export_archive(&exported, Compression::Raw).unwrap();
        assert_eq!(std::fs::read(exported).unwrap(), bytes);
    }
    #[test]
    fn archive_rejects_bounds_corruption_unknown_codec_and_index_overlap() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("corrupt.r3a");
        let good = literal();
        for case in [
            "version",
            "canonical_version",
            "header_checksum",
            "oversized_directory",
            "oversized_count",
            "oversized_decoded",
            "truncated",
            "trailing",
            "codec",
            "block_overlap",
            "record_overlap",
            "record_id",
            "source_digest",
            "body_checksum",
            "duplicate_index",
        ] {
            let mut b = good.clone();
            match case {
                "version" => b[8] = 9,
                "canonical_version" => b[11] = 9,
                "header_checksum" => b[104] ^= 1,
                "oversized_directory" => b[12..16].copy_from_slice(&u32::MAX.to_le_bytes()),
                "oversized_count" => b[48..52].copy_from_slice(&u32::MAX.to_le_bytes()),
                "oversized_decoded" => b[136..144].copy_from_slice(&u64::MAX.to_le_bytes()),
                "truncated" => {
                    b.pop();
                }
                "trailing" => b.push(0),
                "codec" => {
                    b[177 + 16] = 9;
                    rehash(&mut b);
                }
                "block_overlap" => {
                    b[177..185].copy_from_slice(&143u64.to_le_bytes());
                    rehash(&mut b);
                }
                "record_overlap" => {
                    b[177 + 52 + 12..177 + 52 + 16].copy_from_slice(&1u32.to_le_bytes());
                    rehash(&mut b);
                }
                "record_id" => {
                    b[177 + 52..177 + 60].copy_from_slice(&2i64.to_le_bytes());
                    rehash(&mut b);
                }
                "source_digest" => {
                    b[72] ^= 1;
                    rehash(&mut b);
                }
                "body_checksum" => b[144 + 20] ^= 1,
                "duplicate_index" => {
                    let second = b[229..249].to_vec();
                    b.extend(second);
                    b[12..16].copy_from_slice(&92u32.to_le_bytes());
                    b[16..24].copy_from_slice(&269u64.to_le_bytes());
                    b[48..52].copy_from_slice(&2u32.to_le_bytes());
                    rehash(&mut b);
                }
                _ => unreachable!(),
            }
            std::fs::write(&path, b).unwrap();
            assert!(Archive::open(&path).is_err(), "{case}");
        }
    }

    #[test]
    fn compressed_archive_rejects_extra_frames_and_expansion_mismatch() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("compressed.r3a");
        let frame = zstd::stream::encode_all(EVENT, 3).unwrap();
        for case in [
            "valid",
            "extra_frame",
            "wrong_decoded_length",
            "damaged_frame",
        ] {
            let mut stored = frame.clone();
            if case == "extra_frame" {
                stored.extend_from_slice(&frame);
            } else if case == "damaged_frame" {
                stored[0] ^= 1;
            }
            let good = literal();
            let mut b = good[..144].to_vec();
            b[10] = 1;
            b.extend_from_slice(&stored);
            let directory = b.len();
            b.extend_from_slice(&good[177..]);
            b[directory + 8..directory + 12].copy_from_slice(&(stored.len() as u32).to_le_bytes());
            b[directory + 16] = 1;
            if case == "wrong_decoded_length" {
                b[directory + 12..directory + 16].copy_from_slice(&32u32.to_le_bytes());
                b[directory + 52 + 16..directory + 72].copy_from_slice(&32u32.to_le_bytes());
                b[56..64].copy_from_slice(&32u64.to_le_bytes());
            }
            let total = b.len() as u64;
            b[16..24].copy_from_slice(&total.to_le_bytes());
            b[24..32].copy_from_slice(&(directory as u64).to_le_bytes());
            rehash(&mut b);
            std::fs::write(&path, b).unwrap();
            if case == "valid" {
                assert_eq!(
                    Archive::open(&path).unwrap().canonical_body(1).unwrap(),
                    EVENT
                );
            } else {
                assert!(
                    matches!(Archive::open(&path), Err(Error::Corrupt(_))),
                    "{case}"
                );
            }
        }
    }
}
