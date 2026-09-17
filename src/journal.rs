//! Explicit experimental single-writer journal over an immutable R3ARCH snapshot.
//! Canonical event bytes and Archive's validators/version/graph view are shared.
use crate::{
    Error, Result,
    archive::Archive,
    codec::{self, Reader},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

const HEADER: usize = 224;
const FRAME: usize = 128;
const TRAILER: usize = 56;
const MAX_RAW: usize = 4 * 1024 * 1024;
const MAX_FILE: u64 = 512 * 1024 * 1024;
type Hash = [u8; 32];
fn bad(s: &str) -> Error {
    Error::Corrupt(format!("journal: {s}"))
}
fn hash(b: &[u8]) -> Hash {
    Sha256::digest(b).into()
}
fn u64_at(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}
fn u32_at(b: &[u8], i: usize) -> usize {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) as usize
}
fn file_hash(file: &mut File) -> Result<Hash> {
    file.seek(SeekFrom::Start(0))?;
    let mut h = Sha256::new();
    let mut b = [0; 65536];
    loop {
        let n = file.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(h.finalize().into())
}
fn source_hash(view: &Archive) -> Result<Hash> {
    let mut out = [0; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&view.info.source_identity[i * 2..i * 2 + 2], 16)
            .map_err(|_| bad("snapshot source hash"))?;
    }
    Ok(out)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub sequence: u64,
    pub digest: Hash,
    pub end: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tail {
    pub offset: u64,
    pub incomplete: bool,
    pub reason: String,
}
pub struct Journal {
    file: File,
    view: Archive,
    header: [u8; HEADER],
    commits: BTreeMap<[u8; 16], (Hash, Commit)>,
    last: Commit,
    tail: Option<Tail>,
    writer: bool,
    poisoned: bool,
}
impl Journal {
    pub fn create(snapshot: &Path, path: &Path, store_id: [u8; 16]) -> Result<()> {
        let view = Archive::open(snapshot)?;
        let mut h = [0; HEADER];
        h[..8].copy_from_slice(b"R3JRN\0\0\0");
        h[8..10].copy_from_slice(&1u16.to_le_bytes());
        h[10] = 2;
        h[11] = 1;
        h[12..16].copy_from_slice(&(HEADER as u32).to_le_bytes());
        h[16..32].copy_from_slice(&store_id);
        h[32..64].copy_from_slice(&file_hash(&mut File::open(snapshot)?)?);
        h[64..96].copy_from_slice(&source_hash(&view)?);
        h[96..104].copy_from_slice(&(MAX_RAW as u64).to_le_bytes());
        h[104..112].copy_from_slice(&MAX_FILE.to_le_bytes());
        let digest = hash(&h[..192]);
        h[192..].copy_from_slice(&digest);
        codec::publish_new(path, |f, _| {
            f.write_all(&h)?;
            Ok(())
        })
    }
    pub fn open(snapshot: &Path, path: &Path, writer: bool) -> Result<Self> {
        let mut file = OpenOptions::new().read(true).write(writer).open(path)?;
        let lock = if writer {
            file.try_lock()
        } else {
            file.try_lock_shared()
        };
        lock.map_err(|e| Error::Conflict(format!("journal lock: {e}")))?;
        let total = file.metadata()?.len();
        if !(HEADER as u64..=MAX_FILE).contains(&total) {
            return Err(bad("header/file bound"));
        }
        let mut header = [0; HEADER];
        file.read_exact(&mut header)?;
        if &header[..8] != b"R3JRN\0\0\0"
            || header[8..12] != [1, 0, 2, 1]
            || u32_at(&header, 12) != HEADER
            || u64_at(&header, 96) != MAX_RAW as u64
            || u64_at(&header, 104) != MAX_FILE
            || header[152..192].iter().any(|b| *b != 0)
            || hash(&header[..192]) != header[192..]
        {
            return Err(bad("header identity/version/hash/bounds"));
        }
        if (header[112..144] == [0; 32]) != (u64_at(&header, 144) == 0) {
            return Err(bad("recovery provenance"));
        }
        let view = Archive::open(snapshot)?;
        if file_hash(&mut File::open(snapshot)?)? != header[32..64]
            || source_hash(&view)? != header[64..96]
        {
            return Err(bad("snapshot root mismatch"));
        }
        let mut out = Self {
            file,
            view,
            header,
            commits: BTreeMap::new(),
            last: Commit {
                sequence: 0,
                digest: [0; 32],
                end: HEADER as u64,
            },
            tail: None,
            writer,
            poisoned: false,
        };
        while out.last.end < total {
            let offset = out.last.end;
            if total - offset < FRAME as u64 {
                out.tail = Some(Tail {
                    offset,
                    incomplete: true,
                    reason: "partial frame header".into(),
                });
                break;
            }
            let mut h = [0; FRAME];
            out.file.seek(SeekFrom::Start(offset))?;
            out.file.read_exact(&mut h)?;
            let stored = u32_at(&h, 80);
            let raw = u32_at(&h, 76);
            let count = u32_at(&h, 72);
            if &h[..8] != b"R3TXN\0\0\0"
                || stored > MAX_RAW
                || raw == 0
                || raw > MAX_RAW
                || count == 0
                || count > 256
                || h[84] > 1
                || h[85..96].iter().any(|b| *b != 0)
                || u64_at(&h, 8) != (FRAME + stored + TRAILER) as u64
            {
                out.tail = Some(Tail {
                    offset,
                    incomplete: false,
                    reason: "frame header/bounds".into(),
                });
                break;
            }
            if total - offset < u64_at(&h, 8) {
                out.tail = Some(Tail {
                    offset,
                    incomplete: true,
                    reason: "partial frame body/trailer".into(),
                });
                break;
            }
            let mut body = vec![0; stored];
            out.file.read_exact(&mut body)?;
            let mut trailer = [0; TRAILER];
            out.file.read_exact(&mut trailer)?;
            if let Err(e) = out.replay(&h, &body, &trailer) {
                out.tail = Some(Tail {
                    offset,
                    incomplete: false,
                    reason: e.to_string(),
                });
                break;
            }
        }
        if writer && out.tail.is_some() {
            return Err(bad("damaged tail: use explicit recovery to new path"));
        }
        Ok(out)
    }
    fn replay(&mut self, h: &[u8; FRAME], stored: &[u8], t: &[u8; TRAILER]) -> Result<()> {
        let mut digest = Sha256::new();
        digest.update(h);
        digest.update(stored);
        let digest: Hash = digest.finalize().into();
        let sequence = u64_at(h, 16);
        let length = u64_at(h, 8);
        if &t[..8] != b"R3COMMIT"
            || u64_at(t, 8) != length
            || u64_at(t, 16) != sequence
            || t[24..] != digest
            || sequence != self.last.sequence + 1
            || h[40..72] != self.last.digest
        {
            return Err(bad("commit/sequence/previous hash"));
        }
        let raw = if h[84] == 0 {
            stored.to_vec()
        } else {
            if zstd::zstd_safe::find_frame_compressed_size(stored).map_err(|_| bad("zstd frame"))?
                != stored.len()
            {
                return Err(bad("trailing zstd"));
            }
            let mut decoder = zstd::stream::read::Decoder::new(stored)?;
            decoder.window_log_max(22)?;
            let mut raw = Vec::new();
            decoder.take(MAX_RAW as u64 + 1).read_to_end(&mut raw)?;
            raw
        };
        if raw.len() != u32_at(h, 76) || hash(&raw) != h[96..128] {
            return Err(bad("raw hash/length"));
        }
        let request: [u8; 16] = h[24..40].try_into().unwrap();
        if self.commits.contains_key(&request) {
            return Err(bad("duplicate request frame"));
        }
        let bodies = decode_body(&raw, u32_at(h, 72))?;
        let events = self.view.prepare_overlay(&bodies)?;
        let commit = Commit {
            sequence,
            digest,
            end: self.last.end + length,
        };
        self.view.apply_overlay(events, bodies);
        self.commits.insert(request, (hash(&raw), commit.clone()));
        self.last = commit;
        Ok(())
    }
    pub fn view(&self) -> &Archive {
        &self.view
    }
    pub fn tail(&self) -> Option<&Tail> {
        self.tail.as_ref()
    }
    pub fn last(&self) -> &Commit {
        &self.last
    }
    pub fn file_bytes(&self) -> Result<u64> {
        Ok(self.file.metadata()?.len())
    }
    pub fn append(
        &mut self,
        request: [u8; 16],
        bodies: Vec<Vec<u8>>,
        level: Option<i32>,
    ) -> Result<Commit> {
        if !self.writer || self.poisoned || self.tail.is_some() {
            return Err(bad("writer unavailable"));
        }
        if bodies.is_empty() || bodies.len() > 256 || level.is_some_and(|v| ![1, 3].contains(&v)) {
            return Err(bad("transaction count/codec"));
        }
        let mut raw = Vec::new();
        for b in &bodies {
            if b.len() > codec::MAX_BODY + codec::HEADER || raw.len() + 4 + b.len() > MAX_RAW {
                return Err(bad("transaction byte bound"));
            }
            raw.extend((b.len() as u32).to_le_bytes());
            raw.extend(b);
        }
        let content = hash(&raw);
        if let Some((old, commit)) = self.commits.get(&request) {
            return if *old == content {
                Ok(commit.clone())
            } else {
                Err(Error::Conflict("journal request content changed".into()))
            };
        }
        let events = self.view.prepare_overlay(&bodies)?;
        let compressed = level.map(|l| zstd::bulk::compress(&raw, l)).transpose()?;
        let (codec, stored) = compressed
            .as_ref()
            .filter(|c| c.len() < raw.len())
            .map_or((0, raw.as_slice()), |c| (1, c.as_slice()));
        let length = FRAME + stored.len() + TRAILER;
        if self.last.end + length as u64 > MAX_FILE || self.file.metadata()?.len() != self.last.end
        {
            return Err(bad("file budget or external append"));
        }
        let sequence = self.last.sequence + 1;
        let mut h = [0; FRAME];
        h[..8].copy_from_slice(b"R3TXN\0\0\0");
        h[8..16].copy_from_slice(&(length as u64).to_le_bytes());
        h[16..24].copy_from_slice(&sequence.to_le_bytes());
        h[24..40].copy_from_slice(&request);
        h[40..72].copy_from_slice(&self.last.digest);
        h[72..76].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        h[76..80].copy_from_slice(&(raw.len() as u32).to_le_bytes());
        h[80..84].copy_from_slice(&(stored.len() as u32).to_le_bytes());
        h[84] = codec;
        h[96..128].copy_from_slice(&content);
        let mut digest = Sha256::new();
        digest.update(h);
        digest.update(stored);
        let digest: Hash = digest.finalize().into();
        let mut t = [0; TRAILER];
        t[..8].copy_from_slice(b"R3COMMIT");
        t[8..16].copy_from_slice(&(length as u64).to_le_bytes());
        t[16..24].copy_from_slice(&sequence.to_le_bytes());
        t[24..].copy_from_slice(&digest);
        // Once any write is attempted, an error requires reopen/replay, never blind retry.
        self.poisoned = true;
        self.file.seek(SeekFrom::Start(self.last.end))?;
        self.file.write_all(&h)?;
        self.file.write_all(stored)?;
        self.file.write_all(&t)?;
        self.file.sync_all()?;
        #[cfg(feature = "test-support")]
        if std::env::var("R3JRN_TEST_CRASH").as_deref() == Ok("after-sync") {
            std::process::exit(91);
        }
        let commit = Commit {
            sequence,
            digest,
            end: self.last.end + length as u64,
        };
        self.view.apply_overlay(events, bodies);
        self.commits.insert(request, (content, commit.clone()));
        self.last = commit.clone();
        self.poisoned = false;
        Ok(commit)
    }
    pub fn recover_to(&mut self, output: &Path) -> Result<()> {
        let source = file_hash(&mut self.file)?;
        let mut h = self.header;
        h[112..144].copy_from_slice(&source);
        h[144..152].copy_from_slice(&self.last.end.to_le_bytes());
        let digest = hash(&h[..192]);
        h[192..].copy_from_slice(&digest);
        self.file.seek(SeekFrom::Start(HEADER as u64))?;
        codec::publish_new(output, |f, _| {
            f.write_all(&h)?;
            let copied =
                std::io::copy(&mut (&mut self.file).take(self.last.end - HEADER as u64), f)?;
            if copied != self.last.end - HEADER as u64 {
                return Err(bad("recovery prefix changed"));
            }
            Ok(())
        })
    }
}
fn decode_body(raw: &[u8], count: usize) -> Result<Vec<Vec<u8>>> {
    let mut r = Reader::new(raw);
    let mut bodies = Vec::with_capacity(count);
    for _ in 0..count {
        let n = u32::from_le_bytes(r.take(4)?.try_into().unwrap()) as usize;
        if !(codec::HEADER..=codec::MAX_BODY + codec::HEADER).contains(&n) {
            return Err(bad("event envelope bound"));
        }
        bodies.push(r.take(n)?.to_vec());
    }
    if !r.finished() {
        return Err(bad("trailing transaction body"));
    }
    Ok(bodies)
}
