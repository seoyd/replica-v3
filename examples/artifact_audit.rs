//! Repository-local, read-only artifact inventory. Never follows links or deletes files.
use replica_v3::binary;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, ffi::OsString, fs::{self, File, OpenOptions}, io::{Read, Write}, os::unix::{ffi::{OsStrExt, OsStringExt}, fs::MetadataExt}, path::{Path, PathBuf}, time::{Instant, SystemTime, UNIX_EPOCH}};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const SHARD_ENTRIES: usize = 512;
const ENTRY_LIMIT: usize = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    path: Vec<u8>, kind: String, bytes: u64, allocated: u64, dev: u64, inode: u64,
    nlink: u64, mode: u32, mtime: i64, mtime_nsec: i64,
    role: String, owner: Vec<u8>, references: Vec<Vec<u8>>, preserve: String, action: String,
    hash: Option<String>, hash_source: String, hash_time: Option<u64>,
}
#[derive(Serialize, Deserialize)]
struct Shard { path: String, sha256: String, entries: usize, bytes: u64 }
#[derive(Serialize, Deserialize)]
struct Inventory {
    version: u32, root: Vec<u8>, excluded_output: Vec<u8>, device: u64, started: u64,
    entries: usize, kinds: BTreeMap<String,u64>, logical_bytes: u64, inode_unique_bytes: u64,
    allocated_estimate: u64, regular_allocated_estimate: u64, skipped_mounts: usize,
    elapsed_seconds: f64, metadata_only: bool, content_hashed_bytes: u64,
    shards: Vec<Shard>, direct_children: BTreeMap<String,u64>, largest_directories: Vec<(String,u64)>,
    largest_files: Vec<(String,u64)>, available_before: String, available_after: String,
    management_output_bytes: u64, deletion_authorized: bool,
}
fn escaped(bytes: &[u8]) -> String { bytes.iter().flat_map(|b|std::ascii::escape_default(*b)).map(char::from).collect() }
fn stamp() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() }
fn hash(path:&Path) -> Result<String> {
    let mut f=File::open(path)?;let mut h=Sha256::new();let mut buf=[0u8;65536];
    loop {let n=f.read(&mut buf)?;if n==0{break}h.update(&buf[..n]);}Ok(format!("{:x}",h.finalize()))
}
fn publish<T:Serialize>(path:&Path, value:&T)->Result<u64> {
    let bytes=binary::to_storage_vec(value)?;let mut f=OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(&bytes)?;f.sync_all()?;File::open(path.parent().unwrap())?.sync_all()?;Ok(bytes.len()as u64)
}
fn available(root:&Path)->Result<String> {
    let out=std::process::Command::new("df").args(["-kP"]).arg(root).output()?;
    if !out.status.success(){return Err("df failed".into())}Ok(String::from_utf8(out.stdout)?)
}
fn scan(root:&Path, output:&Path)->Result<()> {
    if fs::symlink_metadata(root)?.file_type().is_symlink(){return Err("root symlink prohibited".into())}
    let root=root.canonicalize()?;fs::create_dir(output)?;let output=output.canonicalize()?;
    let start=Instant::now();let available_before=available(&root)?;let dev=fs::symlink_metadata(&root)?.dev();
    let mut all=Inventory{version:1,root:root.as_os_str().as_bytes().to_vec(),excluded_output:output.as_os_str().as_bytes().to_vec(),device:dev,started:stamp(),entries:0,kinds:BTreeMap::new(),logical_bytes:0,inode_unique_bytes:0,allocated_estimate:0,regular_allocated_estimate:0,skipped_mounts:0,elapsed_seconds:0.,metadata_only:true,content_hashed_bytes:0,shards:vec![],direct_children:BTreeMap::new(),largest_directories:vec![],largest_files:vec![],available_before,available_after:String::new(),management_output_bytes:0,deletion_authorized:false};
    let mut pending=vec![root.clone()];let mut batch=vec![];let mut unique=BTreeSet::new();let mut dirs=BTreeMap::<Vec<u8>,u64>::new();let mut top=vec![];
    while let Some(dir)=pending.pop() {
        for item in fs::read_dir(&dir)? {
            let item=item?;let path=item.path();if path==output{continue}
            let m=fs::symlink_metadata(&path)?;let relative=path.strip_prefix(&root)?.as_os_str().as_bytes().to_vec();
            all.entries+=1;if all.entries>ENTRY_LIMIT{return Err("inventory entry limit; preserve partial shards".into())}
            let kind=if m.file_type().is_symlink(){"symlink"}else if m.dev()!=dev{all.skipped_mounts+=1;"other-mount"}else if m.is_dir(){pending.push(path.clone());"directory"}else if m.is_file(){"regular"}else{"special"};
            *all.kinds.entry(kind.into()).or_default()+=1;
            if unique.insert((m.dev(),m.ino())) && m.dev()==dev {
                all.allocated_estimate+=m.blocks()*512;
                if kind=="regular" {all.inode_unique_bytes+=m.len();all.regular_allocated_estimate+=m.blocks()*512;}
            }
            let owner=relative.split(|b|*b==b'/').next().unwrap().to_vec();
            if kind=="regular" {
                all.logical_bytes+=m.len();*all.direct_children.entry(escaped(&owner)).or_default()+=m.len();
                let mut parent=path.parent();while let Some(p)=parent{if p==root{break}*dirs.entry(p.strip_prefix(&root)?.as_os_str().as_bytes().to_vec()).or_default()+=m.len();parent=p.parent();}
                top.push((escaped(&relative),m.len()));top.sort_by(|a,b|b.1.cmp(&a.1).then(a.0.cmp(&b.0)));top.truncate(30);
            }
            batch.push(Entry{path:relative,kind:kind.into(),bytes:m.len(),allocated:m.blocks()*512,dev:m.dev(),inode:m.ino(),nlink:m.nlink(),mode:m.mode(),mtime:m.mtime(),mtime_nsec:m.mtime_nsec(),role:"UNCLASSIFIED_METADATA".into(),owner,references:vec![],preserve:"References not yet verified; no deletion authority".into(),action:"KEEP_UNKNOWN".into(),hash:None,hash_source:"NOT_HASHED_METADATA_ONLY".into(),hash_time:None});
            if batch.len()==SHARD_ENTRIES {flush(&output,&mut all,&mut batch)?;}
        }
    }
    if !batch.is_empty(){flush(&output,&mut all,&mut batch)?;}
    all.largest_directories=dirs.into_iter().map(|(p,n)|(escaped(&p),n)).collect();
    all.largest_directories.sort_by(|a,b|b.1.cmp(&a.1).then(a.0.cmp(&b.0)));all.largest_directories.truncate(20);all.largest_files=top;
    all.available_after=available(&root)?;all.elapsed_seconds=start.elapsed().as_secs_f64();
    publish(&output.join("artifact-inventory.r3b"),&all)?;
    let mut report=OpenOptions::new().write(true).create_new(true).open(output.join("inventory.md"))?;
    writeln!(report,"# Artifact inventory (metadata only, dry run)\n\nentries={} kinds={:?}\nlogical_bytes={}\ninode_unique_bytes={}\nallocated_estimate={}\nregular_allocated_estimate={}\nelapsed_seconds={}\ncontent_hashed_bytes=0\nshard_output_bytes={}\n\nAllocation uses unique (dev,inode) and stat blocks; APFS physical reclaim is UNKNOWN. Excludes this live audit output. No link or other mount followed; no content read or deletion.\n\n## Direct children",all.entries,all.kinds,all.logical_bytes,all.inode_unique_bytes,all.allocated_estimate,all.regular_allocated_estimate,all.elapsed_seconds,all.management_output_bytes)?;
    for(p,n)in &all.direct_children{writeln!(report,"- `{p}`: {n} bytes")?;}
    for(title,items)in [("Largest directories",&all.largest_directories),("Largest files",&all.largest_files)]{writeln!(report,"\n## {title}")?;for(p,n)in items{writeln!(report,"- `{p}`: {n} bytes")?;}}
    writeln!(report,"\n## Available space (df -kP)\n\nBefore:\n```text\n{}\n```\nAfter:\n```text\n{}\n```",all.available_before,all.available_after)?;report.sync_all()?;
    println!("INVENTORY entries={} regular={} logical={} unique={} allocated={} elapsed={:.3} content_hashed=0 deleted=0 manifest={}",all.entries,all.kinds.get("regular").unwrap_or(&0),all.logical_bytes,all.inode_unique_bytes,all.allocated_estimate,all.elapsed_seconds,output.join("artifact-inventory.r3b").display());Ok(())
}
fn flush(output:&Path,all:&mut Inventory,batch:&mut Vec<Entry>)->Result<()> {
    let name=format!("inventory-{:04}.r3b",all.shards.len());let bytes=publish(&output.join(&name),batch)?;
    all.shards.push(Shard{path:name.clone(),sha256:hash(&output.join(name))?,entries:batch.len(),bytes});all.management_output_bytes+=bytes;batch.clear();Ok(())
}
#[derive(Serialize,Deserialize)]
struct Unit {root:String,source:String,report:String,report_hash:String,source_lock_hash:String,source_manifest_hash:String,files:usize,bytes:u64,allocated:u64,recipe:String}
#[derive(Serialize,Deserialize)]
struct CleanupPlan {
    schema:u32,inventory:String,inventory_hash:String,root:Vec<u8>,started:u64,elapsed_seconds:f64,
    roles:BTreeMap<String,(u64,u64)>,candidate_shards:Vec<Shard>,classification_shards:Vec<Shard>,
    units:Vec<Unit>,references:Vec<(Vec<u8>,String,Vec<String>)>,protected_hashes:Vec<(Vec<u8>,String,u64)>,
    candidate_unique_bytes:u64,candidate_allocated_estimate:u64,duplicate_upper_bound:u64,
    duplicate_scope:String,hashed_input_bytes:u64,metadata_read_bytes:u64,output_bytes:u64,
    process_observation:String,available_before:String,available_after:String,
    purge_authority:String,applied:bool,reclaimed_bytes:u64,physical_reclaim:String,archive_probe:String,
    #[serde(default)] reused_hash_plan:Option<(String,String)>,
}
fn decode<T:serde::de::DeserializeOwned>(p:&Path)->Result<T>{
    if fs::metadata(p)?.len()>128*1024*1024{return Err("management record size limit".into())}
    Ok(binary::from_slice(&fs::read(p)?)?)
}
fn unchanged(root:&Path,e:&Entry)->Result<PathBuf>{
    let relative=PathBuf::from(OsString::from_vec(e.path.clone()));
    if relative.is_absolute()||relative.components().any(|c|!matches!(c,std::path::Component::Normal(_))){return Err("invalid relative path".into())}
    let mut path=root.to_path_buf();
    for part in relative.components(){path.push(part);let m=fs::symlink_metadata(&path)?;
        if m.file_type().is_symlink()||m.dev()!=e.dev{return Err("link/mount changed since inventory".into())}}
    let m=fs::symlink_metadata(&path)?;
    if !m.is_file()||m.dev()!=e.dev||m.ino()!=e.inode||m.len()!=e.bytes||m.mtime()!=e.mtime||m.mtime_nsec()!=e.mtime_nsec||m.nlink()!=e.nlink{return Err("file changed since inventory".into())}Ok(path)
}
// Only these closed review tasks have their build ownership and reproduction
// evidence inspected. All other trees remain protected, including other targets.
const CLOSED_BUILDS:[(&str,&str,&str);3]=[
    ("citation-precision-20260922-review/A-target/debug/incremental","citation-precision-20260922-review/A-source","docs/CITATION_PRECISION_REVIEW_A_2026-09-22.md"),
    ("answer-mean-citation-20260922-review/A-target/debug/incremental","answer-mean-citation-20260922-review/A-source","docs/ANSWER_MEAN_INDEPENDENT_REVIEW_2026-09-22.md"),
    ("query-signal-independent-20260921-EpXxfa/target/debug/incremental","query-signal-independent-20260921-EpXxfa/source","docs/QUERY_SIGNAL_INDEPENDENT_REVIEW_2026-09-21.md"),
];
// Explicitly supported metadata owners, not a recursive binary crawler. Native,
// sealed corpus and user DB bodies are never decoded by this audit.
const OWNERS:[&str;4]=["instruction-bridge-completion-20260924-study-final","qa-integrity-bridge-20260923-study-final","citation-precision-20260922-study-final","rebind-consolidation-20260921-study"];
fn reference_strings(v:&binary::Value,out:&mut Vec<String>){
    if let Some(s)=v.as_str(){if s.starts_with('/')||s.starts_with("artifacts/"){out.push(s.into())}}
    else if let Some(a)=v.as_array(){for v in a{reference_strings(v,out)}}
    else if let Some(o)=v.as_object(){for v in o.values(){reference_strings(v,out)}}
}
fn analyze(input:&Path,output:&Path,reuse:Option<&Path>)->Result<()> {
    let inv:Inventory=decode(&input.join("artifact-inventory.r3b"))?;
    if inv.version!=1||!inv.metadata_only{return Err("unsupported inventory".into())}
    let root=PathBuf::from(OsString::from_vec(inv.root.clone()));let repo=root.parent().ok_or("repository root")?;
    fs::create_dir(output)?;let output=output.canonicalize()?;let start=Instant::now();
    let processes=std::process::Command::new("ps").args(["-axo","pid,ppid,etime,command"]).output()?;
    if !processes.status.success(){return Err("process observation failed".into())}
    let process_text=String::from_utf8(processes.stdout)?;
    // Preserve the process snapshot locally. A candidate still needs a fresh
    // writer check immediately before any separately authorized application.
    let owned=process_text.lines().filter(|l|l.contains("replica")||l.contains("rustc")||l.contains("cargo")).collect::<Vec<_>>().join("\n");
    let mut proc=OpenOptions::new().write(true).create_new(true).open(output.join("owned-processes.txt"))?;proc.write_all(owned.as_bytes())?;proc.sync_all()?;
    if process_text.lines().any(|l|CLOSED_BUILDS.iter().any(|(p,_,_)|l.contains(p.split("/debug/").next().unwrap()))){return Err("candidate target has active process; no plan".into())}
    let mut plan=CleanupPlan{schema:1,inventory:input.display().to_string(),inventory_hash:hash(&input.join("artifact-inventory.r3b"))?,root:inv.root.clone(),started:stamp(),elapsed_seconds:0.,roles:BTreeMap::new(),candidate_shards:vec![],classification_shards:vec![],units:vec![],references:vec![],protected_hashes:vec![],candidate_unique_bytes:0,candidate_allocated_estimate:0,duplicate_upper_bound:0,duplicate_scope:"Only the two reported final checkpoints, whole-file SHA256; other content not exhaustively hashed".into(),hashed_input_bytes:0,metadata_read_bytes:0,output_bytes:0,process_observation:hash(&output.join("owned-processes.txt"))?,available_before:available(&root)?,available_after:String::new(),purge_authority:"NOT_AUTHORIZED".into(),applied:false,reclaimed_bytes:0,physical_reclaim:"UNKNOWN: stat allocation is not APFS physical reclaim; clones/snapshots/open files unmeasured".into(),archive_probe:"NOT_RUN: existing native-storage probe exports models and repeats loads; no bounded generic archive CLI; no new archive engine".into(),reused_hash_plan:None};
    let mut verified_hashes=BTreeMap::new();
    if let Some(dir)=reuse {
        let old:CleanupPlan=decode(&dir.join("cleanup-plan.r3b"))?;
        if old.inventory_hash!=plan.inventory_hash||old.root!=plan.root||old.applied{return Err("hash reuse inventory/authority mismatch".into())}
        let proof=hash(&dir.join("cleanup-plan.r3b"))?;plan.reused_hash_plan=Some((dir.display().to_string(),proof));
        for shard in &old.candidate_shards {
            if hash(&dir.join(&shard.path))?!=shard.sha256{return Err("reused candidate shard changed".into())}
            let entries:Vec<Entry>=decode(&dir.join(&shard.path))?;plan.metadata_read_bytes+=shard.bytes*2;
            for e in entries{unchanged(&root,&e)?;verified_hashes.insert(e.path.clone(),(e.hash.ok_or("missing previous full hash")?,e.bytes,e.hash_time));}
        }
        for (path,h,n) in old.protected_hashes{verified_hashes.insert(path,(h,n,Some(old.started)));}
    }
    for &(cache,source,report) in &CLOSED_BUILDS {
        let lock=root.join(source).join("Cargo.lock");let manifest=root.join(source).join("Cargo.toml");
        for p in [&lock,&manifest,&repo.join(report)]{plan.hashed_input_bytes+=fs::metadata(p)?.len();}
        plan.units.push(Unit{root:cache.into(),source:source.into(),report:report.into(),report_hash:hash(&repo.join(report))?,source_lock_hash:hash(&lock)?,source_manifest_hash:hash(&manifest)?,files:0,bytes:0,allocated:0,
            recipe:format!("Preserve entire {source}, its overlays and Cargo.lock, all existing executable/deps/rlib/dylib/logs. Rebuild on installed Rust1.98.1 using cargo build/test --locked --offline --features accelerate,test-support, with the report's profile/features/command. Incremental data is optional compiler state; exact executable reproduction is NOT promised; retained executables are the keeper.")});
    }
    // Gather supported root metadata and reference strings from the recorded
    // list. Unknown schemas and subtrees are kept by default.
    for shard in &inv.shards {
        let path=input.join(&shard.path);if hash(&path)?!=shard.sha256{return Err("inventory shard hash".into())}
        let entries:Vec<Entry>=decode(&path)?;plan.metadata_read_bytes+=shard.bytes*2;
        for e in entries {
            let s=std::str::from_utf8(&e.path).unwrap_or("");
            let recognized=OWNERS.iter().any(|r|s.starts_with(&format!("{r}/")))&&["plan.r3b","selection.r3b","preparation.r3b","review-a.r3b","review-b.r3b"].contains(&s.rsplit('/').next().unwrap_or(""));
            if recognized && !s.contains("/sealed/")&&e.kind=="regular" {
                let p=unchanged(&root,&e)?;let v:binary::Value=decode(&p)?;let mut refs=vec![];reference_strings(&v,&mut refs);refs.sort();refs.dedup();
                let h=hash(&p)?;plan.hashed_input_bytes+=e.bytes;plan.metadata_read_bytes+=e.bytes;
                plan.references.push((e.path,h,refs));
            }
        }
    }
    let mut candidates=vec![];let mut classes=vec![];let mut candidate_inodes=BTreeSet::new();let mut duplicate=vec![];
    for shard in &inv.shards {
        let entries:Vec<Entry>=decode(&input.join(&shard.path))?;plan.metadata_read_bytes+=shard.bytes;
        for mut e in entries {
            let s=std::str::from_utf8(&e.path).unwrap_or("");
            e.role="UNKNOWN_OR_ACTIVE".into();e.action="KEEP".into();e.preserve="Unknown ownership or unsupported schema; entire subtree protected, not a deletion candidate".into();
            let hot=OWNERS.iter().any(|r|s==*r||s.starts_with(&format!("{r}/")));
            if hot {e.role="HOT_PROTECTED".into();e.preserve="Protected accepted/current lineage and all transitive files, including unknown children; no original edits".into();}
            else if s.contains("-study")||s.contains("-executable")||s.contains("/segment-")||s.ends_with(".r3rows")||s.ends_with(".r3m") {
                e.role="COLD_EVIDENCE".into();e.preserve="Conservative evidence retention; name is not proof of expendability; retain originals and dependencies".into();
            }
            for (path,_,refs) in &plan.references {if refs.iter().any(|r|r==&root.join(OsString::from_vec(e.path.clone())).display().to_string()){e.references.push(path.clone());}}
            if let Some(unit)=plan.units.iter_mut().find(|u|s.starts_with(&format!("{}/",u.root))) {
                let referred=plan.references.iter().any(|(_,_,refs)|refs.iter().any(|r|r.contains(&unit.root)));
                // Retain executable bits, multiply linked files, references and
                // unfamiliar compiler entries. Never nominate whole directories.
                let compiler_file=s.ends_with(".o")||s.ends_with("/dep-graph.bin")||s.ends_with("/query-cache.bin")||s.ends_with("/work-products.bin");
                if e.kind=="regular"&&compiler_file&&e.mode&0o111==0&&e.nlink==1&&!referred&&e.references.is_empty() {
                    let p=unchanged(&root,&e)?;
                    let (h,time,reused)=if let Some((h,n,t))=verified_hashes.get(&e.path){if *n!=e.bytes{return Err("reused size mismatch".into())}(h.clone(),*t,true)}else{plan.hashed_input_bytes+=e.bytes;(hash(&p)?,Some(stamp()),false)};
                    unchanged(&root,&e)?;
                    e.role="REBUILDABLE".into();e.action="PROPOSE_DELETE_EXACT_FILE_AFTER_SEPARATE_APPROVAL".into();e.hash=Some(h);e.hash_source=if reused{"REUSED_VERIFIED_PLAN: original whole-file SHA256; current metadata unchanged; not a fresh content read"}else{"EXECUTED_THIS_RUN whole-file SHA256, unchanged metadata before/after"}.into();e.hash_time=time;
                    e.preserve=format!("Closed review provenance {}; optional rustc incremental only; keeper {} plus all existing runtime/deps/reader/patch/log files. Known metadata refs checked; unknown sibling trees retained",unit.report,unit.source);
                    e.references.push(unit.report.as_bytes().to_vec());unit.files+=1;unit.bytes+=e.bytes;unit.allocated+=e.allocated;
                    if candidate_inodes.insert((e.dev,e.inode)){plan.candidate_unique_bytes+=e.bytes;plan.candidate_allocated_estimate+=e.allocated;}
                    candidates.push(e.clone());if candidates.len()==SHARD_ENTRIES{plan.output_bytes+=write_entries(&output,"candidates",&mut plan.candidate_shards,&mut candidates)?;}
                }
            }
            if ["instruction-bridge-completion-20260924-study-final/ANSWER-MEAN/segment-0004/final","instruction-bridge-completion-20260924-study-final/ANSWER-MEAN/segment-0005/final"].contains(&s) {
                let p=unchanged(&root,&e)?;
                let(h,time,reused)=if let Some((h,n,t))=verified_hashes.get(&e.path){if *n!=e.bytes{return Err("reused checkpoint size mismatch".into())}(h.clone(),*t,true)}else{plan.hashed_input_bytes+=e.bytes;(hash(&p)?,Some(stamp()),false)};
                unchanged(&root,&e)?;e.hash=Some(h.clone());e.hash_source=if reused{"REUSED_VERIFIED_PLAN whole-file SHA256; inventory/current identity unchanged"}else{"EXECUTED_THIS_RUN whole-file SHA256; not model-weight hash"}.into();e.hash_time=time;
                plan.protected_hashes.push((e.path.clone(),h.clone(),e.bytes));duplicate.push((h,e.bytes,e.dev,e.inode));
                e.preserve="Final checkpoint/Adam remain path-bound evidence even if whole bytes match; no unlink or hardlink replacement".into();
            }
            let total=plan.roles.entry(e.role.clone()).or_default();total.0+=1;if e.kind=="regular"{total.1+=e.bytes;}
            classes.push(e);if classes.len()==SHARD_ENTRIES{plan.output_bytes+=write_entries(&output,"classification",&mut plan.classification_shards,&mut classes)?;}
        }
    }
    if duplicate.len()!=2{return Err("two protected final checkpoints not in inventory".into())}
    if duplicate[0].0==duplicate[1].0&&duplicate[0].1==duplicate[1].1&&(duplicate[0].2,duplicate[0].3)!=(duplicate[1].2,duplicate[1].3){plan.duplicate_upper_bound=duplicate[0].1;}
    if !candidates.is_empty(){plan.output_bytes+=write_entries(&output,"candidates",&mut plan.candidate_shards,&mut candidates)?;}
    if !classes.is_empty(){plan.output_bytes+=write_entries(&output,"classification",&mut plan.classification_shards,&mut classes)?;}
    plan.elapsed_seconds=start.elapsed().as_secs_f64();plan.available_after=available(&root)?;
    publish(&output.join("cleanup-plan.r3b"),&plan)?;
    let mut report=OpenOptions::new().write(true).create_new(true).open(output.join("cleanup-plan.md"))?;
    writeln!(report,"# Artifact dry-run plan\n\nPURGE_AUTHORITY=NOT_AUTHORIZED; APPLIED=false; reclaimed=0.\nPlan SHA256: {}\nInventory logical={} allocated_estimate={} (unique inode).\nRoles: {:?}\nCandidate unique bytes={} allocated_estimate={}; actual physical reclaim UNKNOWN.\nSelected checkpoint duplicate upper bound={} (not added to purge candidates).\nHashed original input bytes={}; metadata read bytes={}; output shards bytes={}; elapsed_seconds={}.\nArchive probe: {}\n\nEvery candidate file is pinned in candidates shards (path bytes, inode, size, mtime, SHA256, provenance); directories are not purge entries. All originals, executables, runtime dependencies, readers, patches, failed logs and unknown subtrees remain. An apply needs the exact plan digest, a fresh no-follow metadata/hash/reference/keeper/writer check; this tool has no apply command.\n\n## Candidate units",hash(&output.join("cleanup-plan.r3b"))?,inv.logical_bytes,inv.allocated_estimate,plan.roles,plan.candidate_unique_bytes,plan.candidate_allocated_estimate,plan.duplicate_upper_bound,plan.hashed_input_bytes,plan.metadata_read_bytes,plan.output_bytes,plan.elapsed_seconds,plan.archive_probe)?;
    for u in &plan.units{writeln!(report,"- `{}`: {} files, {} logical bytes, {} allocated estimate. Proof `{}` SHA256 `{}`. Keeper `{}`. {}",u.root,u.files,u.bytes,u.allocated,u.report,u.report_hash,u.source,u.recipe)?;}
    writeln!(report,"\n## Protected duplicate comparison")?;
    for(p,h,n)in &plan.protected_hashes{writeln!(report,"- `{}`: {n} bytes, SHA256 `{h}`; KEEP both original paths",escaped(p))?;}
    writeln!(report,"\nReused hash proof: {:?}; unchanged metadata is not a new content hash.\n\n## Space before / after (df -kP)\n```text\n{}\n{}\n```",plan.reused_hash_plan,plan.available_before,plan.available_after)?;report.sync_all()?;
    println!("AUDIT candidates={} logical={} allocated={} hashed={} duplicate_upper={} deleted=0 elapsed={:.3}",plan.units.iter().map(|u|u.files).sum::<usize>(),plan.candidate_unique_bytes,plan.candidate_allocated_estimate,plan.hashed_input_bytes,plan.duplicate_upper_bound,plan.elapsed_seconds);Ok(())
}
fn write_entries(output:&Path,prefix:&str,shards:&mut Vec<Shard>,entries:&mut Vec<Entry>)->Result<u64>{
    let name=format!("{prefix}-{:04}.r3b",shards.len());let bytes=publish(&output.join(&name),entries)?;
    shards.push(Shard{path:name.clone(),sha256:hash(&output.join(name))?,entries:entries.len(),bytes});entries.clear();Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_bytes_links_shards_and_identity()->Result<()> {
        let tmp=tempfile::tempdir_in("artifacts")?;let root=tmp.path().join("input");fs::create_dir(&root)?;
        let first=root.join("original");fs::write(&first,b"abc")?;
        fs::hard_link(&first,root.join("same-inode"))?;
        let outside=tmp.path().join("outside");fs::create_dir(&outside)?;fs::write(outside.join("never-follow"),b"private fixture")?;
        std::os::unix::fs::symlink(&outside,root.join("link"))?;
        for i in 0..SHARD_ENTRIES{fs::write(root.join(format!("empty-{i:04}")),b"")?;}
        let output=tmp.path().join("inventory");scan(&root,&output)?;
        let inv:Inventory=decode(&output.join("artifact-inventory.r3b"))?;
        assert_eq!(inv.shards.len(),2);assert_eq!(inv.logical_bytes,6);assert_eq!(inv.inode_unique_bytes,3);
        assert_eq!(inv.kinds["symlink"],1);assert!(!inv.deletion_authorized);assert_eq!(inv.content_hashed_bytes,0);
        let mut entries=vec![];for shard in inv.shards{assert_eq!(hash(&output.join(&shard.path))?,shard.sha256);entries.extend(decode::<Vec<Entry>>(&output.join(shard.path))?);}
        assert!(!entries.iter().any(|e|e.path.ends_with(b"never-follow")));
        let e=entries.iter().find(|e|e.path==b"original").unwrap();assert_eq!(e.nlink,2);assert_eq!(fs::read(unchanged(&root,e)?)?,b"abc");
        // APFS rejects invalid UTF-8 names; test the manifest's byte contract
        // independently rather than pretending such a pathname was created.
        let mut invalid=e.clone();invalid.path=vec![b'x',255];
        let roundtrip:Entry=binary::from_slice(&binary::to_storage_vec(&invalid)?)?;
        assert_eq!(roundtrip.path,invalid.path);assert_eq!(escaped(&roundtrip.path),"x\\xff");
        fs::write(&first,b"changed")?;assert!(unchanged(&root,e).is_err());
        assert!(scan(&root,&output).is_err());assert_eq!(fs::read(outside.join("never-follow"))?,b"private fixture");
        Ok(())
    }
}
fn main()->Result<()> {
    let args=std::env::args_os().skip(1).collect::<Vec<OsString>>();
    match args.as_slice(){[mode,root,output] if mode=="inventory"=>scan(Path::new(root),Path::new(output)),[mode,input,output] if mode=="analyze"=>analyze(Path::new(input),Path::new(output),None),[mode,input,output,prior] if mode=="analyze"=>analyze(Path::new(input),Path::new(output),Some(Path::new(prior))),_=>Err("usage: artifact_audit inventory ARTIFACTS NEW_OUTPUT | analyze INVENTORY_DIR NEW_OUTPUT [PRIOR_PLAN_FOR_HASH_REUSE]".into())}
}
