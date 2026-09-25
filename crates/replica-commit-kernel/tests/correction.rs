use replica_commit_kernel::correction::*;
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

struct Native;
impl SnapshotCodec for Native {
    fn encode(&self, s: &Snapshot) -> io::Result<Vec<u8>> {
        replica_v3::binary::to_vec(s).map_err(io::Error::other)
    }
    fn decode(&self, b: &[u8]) -> io::Result<Snapshot> {
        replica_v3::binary::from_canonical_slice(b).map_err(io::Error::other)
    }
}
fn key(id: &str, version: u64) -> Key {
    (id.into(), version)
}
fn record(id: &str, version: u64, kind: Kind, deps: Vec<Key>, executed: bool) -> Record {
    Record {
        id: id.into(),
        version,
        kind,
        status: "VERIFIED".into(),
        payload: "preserved payload".into(),
        deps,
        correction_of: if version > 1 {
            Some(key(id, version - 1))
        } else {
            None
        },
        executed_external: executed,
    }
}
fn initial() -> Store {
    let mut s = Store::default();
    s.append(record("root", 1, Kind::Evidence, vec![], false))
        .unwrap();
    s.append(record("claim", 1, Kind::Claim, vec![key("root", 1)], false))
        .unwrap();
    s.append(record(
        "action",
        1,
        Kind::Action,
        vec![key("claim", 1)],
        false,
    ))
    .unwrap();
    s
}
fn correction() -> Record {
    let mut r = record("root", 2, Kind::Evidence, vec![], false);
    r.status = "VERIFIED_FAILURE".into();
    r
}
fn recompute() -> Vec<Record> {
    vec![
        record("claim", 2, Kind::Claim, vec![key("root", 2)], false),
        record("action", 2, Kind::Action, vec![key("claim", 2)], false),
    ]
}
#[test]
fn frozen_provenance_and_directed_closure() {
    for (path, expected, version) in [
        (
            "tests/data/commit_kernel_v1_2K_vectors.json",
            "5c540883cfc656c9d0b953b2002ff62fe17ce24543e52146f69e398712ae7f1b",
            "1.2K",
        ),
        (
            "tests/data/commit_kernel_v1_2L_vectors.json",
            "066e571ead5fe48fc87293dee8e592012069009e4a985da5c35d6446842d3618",
            "1.2L",
        ),
    ] {
        let bytes = fs::read(path).unwrap();
        assert_eq!(hex::encode(Sha256::digest(&bytes)), expected);
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["version"], version);
    }
    let mut initial_state = initial().snapshot();
    initial_state
        .records
        .iter_mut()
        .find(|r| r.id == "root")
        .unwrap()
        .status = "UNKNOWN_EFFECT".into();
    let mut s = Store::restore(initial_state).unwrap();
    let original = s.record(&key("root", 1)).unwrap().clone();
    let mut success = correction();
    success.status = "VERIFIED_SUCCESS".into();
    s.append(success).unwrap();
    // Negative control: direct-head-only would still grant the claim authority.
    assert_eq!(s.head("claim"), Some(1));
    assert!(!s.is_current(&key("claim", 1)));
    assert!(!s.is_current(&key("action", 1)));
    let before = s.snapshot();
    let error = s
        .append(record(
            "stale-attempt",
            1,
            Kind::Action,
            vec![key("claim", 1)],
            false,
        ))
        .unwrap_err();
    assert!(error.to_string().contains("REJECT_STALE_CLOSURE"));
    assert_eq!(s.snapshot(), before);
    for r in recompute() {
        s.append(r).unwrap();
    }
    assert!(s.is_current(&key("action", 2)));
    let mut failure = record("root", 3, Kind::Evidence, vec![], false);
    failure.status = "VERIFIED_FAILURE".into();
    s.append(failure).unwrap();
    assert_eq!(s.validity(&key("action", 2)).unwrap(), Validity::Stale);
    s.append(record(
        "external",
        1,
        Kind::Action,
        vec![key("root", 3)],
        true,
    ))
    .unwrap();
    let mut disputed = record("root", 4, Kind::Evidence, vec![], false);
    disputed.status = "DISPUTED".into();
    s.append(disputed).unwrap();
    assert_eq!(
        s.validity(&key("external", 1)).unwrap(),
        Validity::CompensationRequired
    );
    assert_eq!(s.record(&key("root", 1)), Some(&original));
}
#[test]
fn malformed_restore_and_rejected_append_are_atomic() {
    let s = initial();
    let snapshot = s.snapshot();
    let mut bad = snapshot.clone();
    bad.heads.insert("root".into(), 2);
    assert!(Store::restore(bad).is_err()); // split-file dangling-head mutant
    let mut bad = snapshot.clone();
    bad.records.push(bad.records[0].clone());
    assert!(Store::restore(bad).is_err());
    let mut bad = snapshot.clone();
    bad.records[0].deps = vec![key("missing", 1)];
    assert!(Store::restore(bad).is_err());
    let mut bad = snapshot.clone();
    bad.records
        .iter_mut()
        .find(|r| r.id == "root")
        .unwrap()
        .deps = vec![key("action", 1)];
    assert!(Store::restore(bad).is_err());
    let mut bad = snapshot.clone();
    bad.version = 2;
    assert!(Store::restore(bad).is_err());
    let mut s = s;
    for r in [
        record("root", 1, Kind::Evidence, vec![], false),
        record("root", 3, Kind::Evidence, vec![], false),
        record("missing", 1, Kind::Claim, vec![key("absent", 1)], false),
        record("root", 2, Kind::Evidence, vec![key("claim", 1)], false),
    ] {
        assert!(s.append(r).is_err());
        assert_eq!(s.snapshot(), snapshot);
    }
    let bytes = Native.encode(&snapshot).unwrap();
    assert!(Native.decode(&bytes[..bytes.len() / 2]).is_err());
}
#[test]
fn fanout_branches_races_and_native_rebuild() {
    let start = Instant::now();
    let mut s = Store::default();
    s.append(record("root", 1, Kind::Evidence, vec![], false))
        .unwrap();
    for i in 0..100_000 {
        s.append(record(
            &format!("node{i}"),
            1,
            Kind::Action,
            vec![key("root", 1)],
            i < 40,
        ))
        .unwrap();
    }
    s.append(correction()).unwrap();
    let mut stale = 0;
    let mut compensation = 0;
    for i in 0..100_000 {
        match s.validity(&key(&format!("node{i}"), 1)).unwrap() {
            Validity::Stale => stale += 1,
            Validity::CompensationRequired => compensation += 1,
            other => panic!("{other:?}"),
        }
    }
    assert_eq!((stale, compensation), (99_960, 40));
    // The native codec's existing item bound is unchanged. K's native rebuild
    // gate uses36336 records, separately from the100k in-memory fanout gate.
    let mut snapshot = s.snapshot();
    let included = |id: &str| {
        id == "root"
            || id
                .strip_prefix("node")
                .and_then(|n| n.parse::<usize>().ok())
                .is_some_and(|n| n < 36_334)
    };
    snapshot.records.retain(|r| included(&r.id));
    snapshot.heads.retain(|id, _| included(id));
    assert_eq!(snapshot.records.len(), 36_336);
    let bytes = Native.encode(&snapshot).unwrap();
    let restored = Store::restore(Native.decode(&bytes).unwrap()).unwrap();
    assert_eq!(Native.encode(&restored.snapshot()).unwrap(), bytes);
    assert!(!restored.is_current(&key("node36333", 1)));
    let mut branches = Store::default();
    for root in ["A", "B"] {
        branches
            .append(record(root, 1, Kind::Evidence, vec![], false))
            .unwrap();
        for i in 0..25_000 {
            branches
                .append(record(
                    &format!("{root}{i}"),
                    1,
                    Kind::Claim,
                    vec![key(root, 1)],
                    false,
                ))
                .unwrap();
        }
    }
    branches
        .append(record("A", 2, Kind::Evidence, vec![], false))
        .unwrap();
    for i in 0..25_000 {
        assert!(!branches.is_current(&key(&format!("A{i}"), 1)));
        assert!(branches.is_current(&key(&format!("B{i}"), 1)));
    }
    // Constructed interleaving, not a claim to recover the original random tape.
    let mut race = initial();
    for i in 0..5000 {
        let v = i + 2;
        race.append(record("root", v, Kind::Evidence, vec![], false))
            .unwrap();
        assert!(race
            .append(record(
                &format!("bad{i}"),
                1,
                Kind::Action,
                vec![key("claim", 1)],
                false
            ))
            .is_err());
        race.append(record(
            &format!("fresh{i}"),
            1,
            Kind::Action,
            vec![key("root", v)],
            false,
        ))
        .unwrap();
    }
    println!("fanout=100000 stale={stale} compensation={compensation} native_restore_records={} branches=25000+25000 races=5000 stale_reject=5000 fresh=5000 elapsed_ms={}",restored.len(),start.elapsed().as_millis());
}

fn root(name: &str) -> PathBuf {
    let p =
        PathBuf::from(std::env::var_os("R3_KERNEL_D_EVIDENCE").expect("explicit evidence root"));
    fs::create_dir_all(&p).unwrap();
    p.join(name)
}
fn child(path: &Path, mode: &str, point: &str) -> Command {
    let mut c = Command::new(std::env::current_exe().unwrap());
    c.args(["--ignored", "--exact", "process_worker", "--nocapture"])
        .env("R3_D_STORE", path)
        .env("R3_D_MODE", mode)
        .env("R3_D_POINT", point)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}
fn run(path: &Path, mode: &str) {
    let out = child(path, mode, "").output().unwrap();
    fs::write(path.join(format!("{mode}.stdout")), out.stdout).unwrap();
    fs::write(path.join(format!("{mode}.stderr")), out.stderr).unwrap();
    assert!(out.status.success(), "{mode}: {:?}", out.status);
}
fn kill_at(path: &Path, mode: &str, point: &str) {
    let mut c = child(path, mode, point).spawn().unwrap();
    let start = Instant::now();
    while !path.join("reached").exists() {
        assert!(c.try_wait().unwrap().is_none(), "early child exit {point}");
        if start.elapsed() > Duration::from_secs(10) {
            c.kill().unwrap();
            c.wait().unwrap();
            panic!("timeout {point}");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    c.kill().unwrap();
    let out = c.wait_with_output().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(out.status.signal(), Some(9));
    fs::write(path.join("killed.stdout"), out.stdout).unwrap();
    fs::write(path.join("killed.stderr"), out.stderr).unwrap();
}
#[test]
#[ignore = "fresh process worker only"]
fn process_worker() {
    let path = PathBuf::from(std::env::var_os("R3_D_STORE").unwrap());
    let mode = std::env::var("R3_D_MODE").unwrap();
    let point = std::env::var("R3_D_POINT").unwrap();
    if mode == "init" {
        DurableStore::create(&path, initial(), Native).unwrap();
        return;
    }
    let mut store = DurableStore::open(&path, Native).unwrap();
    let mut hook = |at: &str| -> io::Result<()> {
        if at == point {
            fs::write(path.join("reached"), at)?;
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        Ok(())
    };
    match mode.as_str() {
        "correct" => {
            store.append_batch(&[correction()], &mut hook).unwrap();
            // Test-only advisory sidecar: canonical restore never reads it.
            fs::write(path.join("cache.tmp"), b"torn native cache").unwrap();
            hook("cache-half").unwrap();
            fs::rename(path.join("cache.tmp"), path.join("cache.r3b")).unwrap();
            hook("cache-rename").unwrap();
        }
        "recompute" => store.append_batch(&recompute(), &mut hook).unwrap(),
        "split" => {
            let mut heads = store.store().unwrap().snapshot().heads;
            heads.insert("root".into(), 2);
            fs::write(
                path.join("split-heads.r3b"),
                replica_v3::binary::to_vec(&heads).unwrap(),
            )
            .unwrap();
            hook("split-head").unwrap();
        }
        "read" => {
            let s = store.store().unwrap();
            let bytes = Native.encode(&s.snapshot()).unwrap();
            println!(
                "records={} root={:?} claim={:?} action={:?} old_current={} new_current={}",
                s.len(),
                s.head("root"),
                s.head("claim"),
                s.head("action"),
                s.is_current(&key("action", 1)),
                s.is_current(&key("action", 2))
            );
            fs::write(path.join("readback.r3b"), bytes).unwrap();
        }
        _ => panic!("unknown mode"),
    }
}
#[test]
#[ignore = "real SIGKILL, native restore and advisory-cache boundary"]
fn correction_and_recompute_process_boundaries() {
    let points = [
        "temp-open",
        "write",
        "file-sync",
        "close",
        "rename",
        "directory-sync",
        "cache-half",
        "cache-rename",
    ];
    let mut summary = String::new();
    for (i, point) in points.iter().enumerate() {
        let path = root(&format!("correct-{point}"));
        run(&path, "init");
        kill_at(&path, "correct", point);
        run(&path, "read");
        let s = Store::restore(
            Native
                .decode(&fs::read(path.join("readback.r3b")).unwrap())
                .unwrap(),
        )
        .unwrap();
        let new = i >= 4;
        assert_eq!(s.head("root"), Some(if new { 2 } else { 1 }));
        assert_eq!(s.len(), if new { 4 } else { 3 });
        assert_eq!(s.is_current(&key("action", 1)), !new);
        summary.push_str(&format!(
            "correction {point}: signal9 world={} coherent=true\n",
            if new { "NEW" } else { "OLD" }
        ));
    }
    for (i, point) in ["write", "file-sync", "close", "rename", "directory-sync"]
        .iter()
        .enumerate()
    {
        let path = root(&format!("recompute-{point}"));
        run(&path, "init");
        run(&path, "correct");
        kill_at(&path, "recompute", point);
        run(&path, "read");
        let s = Store::restore(
            Native
                .decode(&fs::read(path.join("readback.r3b")).unwrap())
                .unwrap(),
        )
        .unwrap();
        assert_eq!(s.head("root"), Some(2));
        assert!(!s.is_current(&key("action", 1)));
        assert_eq!(s.is_current(&key("action", 2)), i >= 3);
        assert_eq!(s.len(), if i >= 3 { 6 } else { 4 });
        summary.push_str(&format!(
            "recompute {point}: signal9 fresh={} coherent=true\n",
            i >= 3
        ));
    }
    let path = root("split-mutant");
    run(&path, "init");
    kill_at(&path, "split", "split-head");
    let mut bad = DurableStore::open(&path, Native)
        .unwrap()
        .store()
        .unwrap()
        .snapshot();
    bad.heads =
        replica_v3::binary::from_canonical_slice(&fs::read(path.join("split-heads.r3b")).unwrap())
            .unwrap();
    assert!(Store::restore(bad)
        .unwrap_err()
        .to_string()
        .contains("head"));
    summary.push_str("split-head mutant: actual process signal9 dangling-head rejected\n");
    let path = root("cache-poison");
    run(&path, "init");
    run(&path, "correct");
    for (i, bytes) in [
        b"malformed".to_vec(),
        replica_v3::binary::to_vec(&vec![(key("action", 1), "ACTIVE")]).unwrap(),
    ]
    .iter()
    .enumerate()
    {
        fs::write(path.join("cache.r3b"), bytes).unwrap();
        run(&path, "read");
        let s = Store::restore(
            Native
                .decode(&fs::read(path.join("readback.r3b")).unwrap())
                .unwrap(),
        )
        .unwrap();
        assert!(!s.is_current(&key("action", 1)));
        fs::write(path.join(format!("cache-variant-{i}.r3b")), bytes).unwrap();
    }
    summary.push_str("cache malformed/lying ACTIVE: both stale closures rejected\n");
    fs::write(root("process-summary.txt"), summary).unwrap();
}
#[test]
#[ignore = "200 bounded real process kills; not the original random tape"]
fn repeated_sigkill_keeps_whole_world() {
    let points = [
        "temp-open",
        "write",
        "file-sync",
        "close",
        "rename",
        "directory-sync",
        "cache-half",
        "cache-rename",
    ];
    let (mut old, mut new) = (0, 0);
    for i in 0..200 {
        let path = root(&format!("stress-{i}"));
        run(&path, "init");
        kill_at(&path, "correct", points[i % 8]);
        run(&path, "read");
        let s = Store::restore(
            Native
                .decode(&fs::read(path.join("readback.r3b")).unwrap())
                .unwrap(),
        )
        .unwrap();
        match (s.head("root"), s.len(), s.is_current(&key("action", 1))) {
            (Some(1), 3, true) => old += 1,
            (Some(2), 4, false) => new += 1,
            other => panic!("partial world: {other:?}"),
        }
    }
    assert_eq!((old, new), (100, 100));
    fs::write(
        root("sigkill-summary.txt"),
        format!("runs=200 signal9=200 OLD={old} NEW={new} OTHER=0 schedule=round_robin\n"),
    )
    .unwrap();
}
#[test]
#[ignore = "native atomic batch, exclusive writer and save failure"]
fn native_batch_failure_and_restart() {
    let path = root("batch");
    let mut s = DurableStore::create(&path, initial(), Native).unwrap();
    assert!(DurableStore::open(&path, Native).is_err());
    let before = s.store().unwrap().snapshot();
    let bad = record("bad", 1, Kind::Action, vec![key("root", 1)], false);
    assert!(s
        .append_batch(&[correction(), bad], &mut |_| Ok(()))
        .is_err());
    assert_eq!(s.store().unwrap().snapshot(), before);
    s.append_batch(&[correction()], &mut |_| Ok(())).unwrap();
    s.append_batch(&recompute(), &mut |_| Ok(())).unwrap();
    let continuous = s.store().unwrap().snapshot();
    drop(s);
    run(&path, "read");
    assert_eq!(
        Native
            .decode(&fs::read(path.join("readback.r3b")).unwrap())
            .unwrap(),
        continuous
    );
    let mut s = DurableStore::open(&path, Native).unwrap();
    let r = record("root", 3, Kind::Evidence, vec![], false);
    assert!(s
        .append_batch(&[r], &mut |p| if p == "write" {
            Err(io::Error::other("injected short/disk write failure"))
        } else {
            Ok(())
        })
        .is_err());
    assert!(s.store().is_err());
    drop(s);
    let s = DurableStore::open(&path, Native).unwrap();
    assert_eq!(s.store().unwrap().snapshot(), continuous);
    drop(s);
    fs::write(path.join("correction.state.r3b"), b"torn committed state").unwrap();
    assert!(DurableStore::open(&path, Native).is_err()); // no promotion of leftover valid temp
}
