use replica_commit_kernel::concurrency::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    },
    thread,
    time::{Duration, Instant},
};
struct Done(Arc<AtomicUsize>);
impl Drop for Done {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
fn wait_ticket(ticket: &Ticket, started: Instant) {
    while ticket.poll().unwrap().is_none() {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "bounded ticket wait expired"
        );
        thread::yield_now();
    }
}

fn response(ticket: &Ticket) -> Response {
    ticket.poll().unwrap().expect("serviced").unwrap()
}
fn drain(a: &Authority) -> Vec<Execution> {
    let mut log = vec![];
    while let Some(e) = a.service_one().unwrap() {
        log.push(e);
    }
    log
}
fn evidence(name: &str, text: &str) {
    if let Some(root) = std::env::var_os("R3_KERNEL_C_EVIDENCE") {
        let root = std::path::PathBuf::from(root);
        fs::create_dir_all(&root).unwrap();
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }
}
fn orders(lengths: &[usize]) -> Vec<Vec<usize>> {
    fn visit(
        lengths: &[usize],
        used: &mut [usize],
        tape: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if tape.len() == lengths.iter().sum::<usize>() {
            out.push(tape.clone());
            return;
        }
        for actor in 0..lengths.len() {
            if used[actor] < lengths[actor] {
                tape.push(actor * 10 + used[actor]);
                used[actor] += 1;
                visit(lengths, used, tape, out);
                used[actor] -= 1;
                tape.pop();
            }
        }
    }
    let mut out = vec![];
    visit(lengths, &mut vec![0; lengths.len()], &mut vec![], &mut out);
    out
}

// Direct transcription of the supplied E weak toy semantics, not the F engine.
// It deliberately uses stale read state. Production hardened outcomes below use
// the actual F State::execute path. E and F share guard goals, not every state law.
fn weak_e(case: usize, tape: &[usize]) -> bool {
    let mut bits = [true; 2];
    let mut versions = [0; 2];
    let mut read_bits = [[true; 2]; 2];
    let mut read_versions = [0; 2];
    let mut read_free = [false; 2];
    let mut outcomes = [false; 2];
    let mut value = 0;
    let mut ledger = false;
    let mut nonce = false;
    let mut version = 0;
    let mut matching = 0;
    let mut read_matching = 0;
    let mut read_at = 0;
    let mut commit_at = 0;
    let mut insert_at = None;
    let mut read_positions = [0; 2];
    let mut commit_positions = [0; 2];
    let mut parent = true;
    let mut cap = true;
    let mut observed_authorized = false;
    let mut invalidated = false;
    for (at, event) in tape.iter().enumerate() {
        let (actor, phase) = (event / 10, event % 10);
        match case {
            0 => match phase {
                0 => {
                    read_bits[actor] = bits;
                    read_versions[actor] = versions[actor];
                }
                2 => {
                    if read_versions[actor] == versions[actor] && read_bits[actor][1 - actor] {
                        bits[actor] = false;
                        versions[actor] += 1;
                    }
                }
                _ => (),
            },
            1 => match (actor, phase) {
                (0, 0) => {
                    read_matching = matching;
                    read_at = at;
                }
                (0, 2) => {
                    outcomes[0] = read_matching == 0;
                    commit_at = at;
                }
                (1, 0) => {
                    matching += 1;
                    insert_at = Some(at);
                }
                _ => (),
            },
            2 => {
                if phase == 0 {
                    read_free[actor] = !ledger;
                } else if read_free[actor] {
                    value += 5;
                    ledger = true;
                }
            }
            3 => {
                if phase == 0 {
                    read_free[actor] = !nonce;
                } else if read_free[actor] {
                    nonce = true;
                    outcomes[actor] = true;
                }
            }
            4 => {
                if phase == 0 {
                    read_versions[actor] = version;
                    read_positions[actor] = at;
                } else {
                    version += 1;
                    outcomes[actor] = true;
                    commit_positions[actor] = at;
                }
            }
            5 => match (actor, phase) {
                (0, 0) => {
                    observed_authorized = parent && cap;
                    read_at = at;
                }
                (0, 1) => {
                    outcomes[0] = observed_authorized;
                    commit_at = at;
                }
                (1, 0) => {
                    parent = false;
                    if at > read_at && commit_at == 0 {
                        invalidated = true;
                    }
                }
                (2, 0) => {
                    cap = false;
                    if at > read_at && commit_at == 0 {
                        invalidated = true;
                    }
                }
                _ => (),
            },
            _ => unreachable!(),
        }
    }
    match case {
        0 => !bits[0] && !bits[1],
        1 => insert_at.is_some_and(|p| read_at < p && p < commit_at) && outcomes[0],
        2 => value != 5,
        3 => outcomes[0] && outcomes[1],
        4 => {
            outcomes[0]
                && outcomes[1]
                && (read_positions[0] < read_positions[1]
                    && read_positions[1] < commit_positions[0]
                    || read_positions[1] < read_positions[0]
                        && read_positions[0] < commit_positions[1])
        }
        5 => invalidated && outcomes[0],
        _ => unreachable!(),
    }
}
fn hardened_f(case: usize, tape: &[usize]) -> bool {
    let mut s = State::default();
    let mut snapshots = [s.snapshot(), s.snapshot()];
    let mut committed = [false; 2];
    let mut stale_accepted = false;
    for event in tape {
        let (actor, phase) = (event / 10, event % 10);
        if case == 1 && actor == 1 {
            s.execute(&Request::Control(Control::InsertMatch)).unwrap();
            continue;
        }
        if case == 5 && actor > 0 {
            s.execute(&Request::Control(if actor == 1 {
                Control::Cancel
            } else {
                Control::Revoke
            }))
            .unwrap();
            continue;
        }
        if phase == 0 {
            snapshots[actor] = s.snapshot();
            continue;
        }
        if (case == 0 || case == 1) && phase == 1 {
            continue;
        }
        let mutation = match case {
            0 => {
                if actor == 0 {
                    Mutation::AOff
                } else {
                    Mutation::BOff
                }
            }
            1 => Mutation::NoMatch,
            _ => Mutation::Delta(5),
        };
        let op = if case == 2 { 1 } else { actor as u64 + 1 };
        let nonce = if case == 2 || case == 3 {
            11
        } else {
            actor as u64 + 11
        };
        let before = s.snapshot();
        let r = s
            .execute(&Request::Propose(Proposal::new(
                op,
                nonce,
                mutation,
                snapshots[actor].clone(),
                1,
            )))
            .unwrap();
        committed[actor] = r.outcome == "COMMIT";
        if committed[actor] {
            stale_accepted |= match case {
                0 | 1 => snapshots[actor].predicate_version != before.predicate_version,
                4 => snapshots[actor].object_version != before.object_version,
                5 => {
                    !before.parent_active
                        || snapshots[actor].capability_epoch != before.capability_epoch
                }
                _ => false,
            };
        }
    }
    match case {
        0 => stale_accepted || !(s.snapshot().a || s.snapshot().b),
        1 | 4 | 5 => stale_accepted,
        2 => s.snapshot().value != 5 || s.ledger().len() != 1,
        3 => s.ledger().len() > 1,
        _ => unreachable!(),
    }
}
#[test]
fn frozen_e_interleavings_and_actual_f_guards() {
    #[derive(Deserialize)]
    struct Count {
        bad: usize,
        total: usize,
    }
    #[derive(Deserialize)]
    struct Scenario {
        name: String,
        weak: Count,
        hardened: Count,
    }
    #[derive(Deserialize)]
    struct Fixture {
        scenarios: Vec<Scenario>,
    }
    let raw = fs::read("tests/data/commit_kernel_v1_2E_concurrency_vectors.json").unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&raw)),
        "b5ef799f90fcebd8ab4f917f1a77e6e7e318360cb2b38eb23752139f1f81dce9"
    );
    let fixture: Fixture = serde_json::from_slice(&raw).unwrap();
    assert_eq!(fixture.scenarios.len(), 6);
    let lengths: &[&[usize]] = &[&[3, 3], &[3, 1], &[2, 2], &[2, 2], &[2, 2], &[2, 1, 1]];
    let mut summary = String::new();
    for (case, expected) in fixture.scenarios.iter().enumerate() {
        let all = orders(lengths[case]);
        let weak = all.iter().filter(|o| weak_e(case, o)).count();
        let hard = all.iter().filter(|o| hardened_f(case, o)).count();
        assert_eq!(
            (weak, all.len()),
            (expected.weak.bad, expected.weak.total),
            "E {}",
            expected.name
        );
        assert_eq!(
            (hard, all.len()),
            (expected.hardened.bad, expected.hardened.total),
            "F {}",
            expected.name
        );
        summary.push_str(&format!(
            "{} E_weak={weak}/{} F_bad={hard}/{}\n",
            expected.name,
            all.len(),
            all.len()
        ));
    }
    evidence("exhaustive.txt", &summary);
}

#[test]
fn preservation_guards_and_real_queue_boundaries() {
    let a = Authority::default();
    let snap = a.snapshot().unwrap();
    let p = Proposal::new(1, 11, Mutation::Delta(5), snap.clone(), 1);
    let first = a.submit(p.clone(), Tier::Low).unwrap();
    assert_eq!(first.tier, Tier::High);
    let retry = a.submit(p.clone(), Tier::High).unwrap();
    assert!(retry.coalesced);
    for _ in 0..1000 {
        assert!(a.submit(p.clone(), Tier::High).unwrap().coalesced);
    }
    assert_eq!(a.queued().unwrap(), [0, 1, 0]);
    assert_eq!(drain(&a).len(), 1);
    assert_eq!(response(&first).outcome, "COMMIT");
    assert_eq!(response(&retry).outcome, "REPLAY");
    let mut changed = p.clone();
    changed.mutation = Mutation::Delta(6);
    changed.reviewed_effect_digest = changed.mutation.digest();
    let wrong = a.submit(changed, Tier::High).unwrap();
    drain(&a);
    assert_eq!(response(&wrong).outcome, "REJECT_OP_MISMATCH");
    let mut tamper = Proposal::new(2, 12, Mutation::Delta(1), a.snapshot().unwrap(), 1);
    tamper.mutation = Mutation::Delta(2);
    let ticket = a.submit(tamper, Tier::High).unwrap();
    drain(&a);
    assert_eq!(response(&ticket).outcome, "REJECT_EFFECT_DIGEST");
    let no_cap = a
        .submit(
            Proposal::new(3, 13, Mutation::Delta(1), a.snapshot().unwrap(), 99),
            Tier::High,
        )
        .unwrap();
    drain(&a);
    assert_eq!(response(&no_cap).outcome, "REJECT_CAP");
    let s = a.state().unwrap();
    assert_eq!(s.ledger().len(), 1);
    assert_eq!(s.nonces().len(), 1);
    let mut counts = [0; 3];
    for _ in 0..32 {
        a.submit_control(Control::Policy).unwrap();
    }
    for i in 0..64 {
        a.submit(
            Proposal::new(100 + i, 100 + i, Mutation::Delta(0), snap.clone(), 1),
            Tier::High,
        )
        .unwrap();
        a.submit_snapshot().unwrap();
    }
    assert!(matches!(
        a.submit_control(Control::Policy),
        Err(Error::Backpressure(Tier::Critical))
    ));
    assert!(matches!(
        a.submit(
            Proposal::new(900, 900, Mutation::Delta(0), snap.clone(), 1),
            Tier::Low
        ),
        Err(Error::Backpressure(Tier::High))
    ));
    assert!(matches!(
        a.submit_snapshot(),
        Err(Error::Backpressure(Tier::Low))
    ));
    // Start a fresh scheduler to measure exactly one full weighted cycle.
    let q = Authority::default();
    for _ in 0..32 {
        q.submit_control(Control::Policy).unwrap();
    }
    for i in 0..64 {
        q.submit(
            Proposal::new(i, i, Mutation::Delta(0), snap.clone(), 1),
            Tier::High,
        )
        .unwrap();
        q.submit_snapshot().unwrap();
    }
    for i in 0..100 {
        let e = q.service_one().unwrap().unwrap();
        match e.request {
            Request::Control(_) => {
                counts[0] += 1;
                q.submit_control(Control::Policy).unwrap();
            }
            Request::Propose(_) => {
                counts[1] += 1;
                q.submit(
                    Proposal::new(
                        1000 + i,
                        1000 + i,
                        Mutation::Delta(0),
                        q.snapshot().unwrap(),
                        1,
                    ),
                    Tier::High,
                )
                .unwrap();
            }
            Request::Snapshot => {
                counts[2] += 1;
                q.submit_snapshot().unwrap();
            }
        }
        assert_eq!(q.queued().unwrap(), [32, 64, 64]);
    }
    assert_eq!(counts, [65, 25, 10]);
    let empty_shares = Authority::default();
    for _ in 0..64 {
        empty_shares.submit_snapshot().unwrap();
    }
    assert_eq!(drain(&empty_shares).len(), 64);
    evidence(
        "queue.txt",
        "caps=32/64/64 service=65/25/10 borrow=64/64 retry_coalesced=1001 risk_floor=High PASS\n",
    );
}

#[test]
fn commit_time_invalidation_and_replay_preserve_original_guards() {
    for (control, mutation, expected) in [
        (Control::Cancel, Mutation::Delta(1), "REJECT_PARENT"),
        (Control::ReuseParent, Mutation::Delta(1), "REJECT_PARENT"),
        (Control::Policy, Mutation::Delta(1), "REJECT_POLICY"),
        (Control::Revoke, Mutation::Delta(1), "REJECT_CAP"),
        (Control::ObjectChange, Mutation::Delta(1), "REJECT_OBJ"),
        (Control::InsertMatch, Mutation::NoMatch, "REJECT_PRED"),
    ] {
        let a = Authority::default();
        let p = Proposal::new(1, 11, mutation, a.snapshot().unwrap(), 1);
        let old = a.submit(p, Tier::High).unwrap();
        a.submit_control(control).unwrap();
        drain(&a);
        assert_eq!(response(&old).outcome, expected);
        assert!(a.state().unwrap().ledger().is_empty());
    }
    let a = Authority::default();
    let p = Proposal::new(1, 11, Mutation::Delta(3), a.snapshot().unwrap(), 1);
    let first = a.submit(p.clone(), Tier::High).unwrap();
    drain(&a);
    a.submit_control(Control::Cancel).unwrap();
    a.submit_control(Control::Revoke).unwrap();
    drain(&a);
    let retry = a.submit(p, Tier::High).unwrap();
    drain(&a);
    assert_eq!(response(&retry).outcome, "REPLAY");
    assert_eq!(response(&retry).receipt, response(&first).receipt);
}

#[test]
fn seven_barrier_races_use_the_actual_authority() {
    let mut summary = String::new();
    for case in 0..7 {
        let mut commits = 0;
        let mut replays = 0;
        let mut rejects = 0;
        for _ in 0..128 {
            let a = Arc::new(Authority::default());
            let snap = a.snapshot().unwrap();
            let p = Proposal::new(
                1,
                11,
                match case {
                    2 => Mutation::AOff,
                    4 => Mutation::NoMatch,
                    _ => Mutation::Delta(5),
                },
                snap.clone(),
                1,
            );
            let b = Arc::new(Barrier::new(3));
            let tickets = thread::scope(|scope| {
                let a1 = a.clone();
                let b1 = b.clone();
                let t1 = scope.spawn(move || {
                    b1.wait();
                    a1.submit(p, Tier::High).unwrap()
                });
                let a2 = a.clone();
                let b2 = b.clone();
                let t2 = scope.spawn(move || {
                    b2.wait();
                    match case {
                        0 => a2
                            .submit(
                                Proposal::new(1, 11, Mutation::Delta(5), snap, 1),
                                Tier::High,
                            )
                            .unwrap(),
                        1 => a2
                            .submit(
                                Proposal::new(2, 11, Mutation::Delta(5), snap, 1),
                                Tier::High,
                            )
                            .unwrap(),
                        2 => a2
                            .submit(Proposal::new(2, 12, Mutation::BOff, snap, 1), Tier::High)
                            .unwrap(),
                        3 => a2
                            .submit(
                                Proposal::new(2, 12, Mutation::Delta(7), snap, 1),
                                Tier::High,
                            )
                            .unwrap(),
                        4 => a2.submit_control(Control::InsertMatch).unwrap(),
                        5 => a2.submit_control(Control::Cancel).unwrap(),
                        _ => a2.submit_control(Control::Revoke).unwrap(),
                    }
                });
                b.wait();
                // Drain while submitters race, not only after both have enqueued.
                while !t1.is_finished() || !t2.is_finished() {
                    a.service_one().unwrap();
                    thread::yield_now();
                }
                (t1.join().unwrap(), t2.join().unwrap())
            });
            drain(&a);
            let r1 = response(&tickets.0);
            let r2 = response(&tickets.1);
            for r in [&r1, &r2] {
                if r.outcome == "COMMIT" {
                    commits += 1;
                } else if r.outcome == "REPLAY" {
                    replays += 1;
                } else if r.outcome.starts_with("REJECT_") {
                    rejects += 1;
                }
            }
            match case {
                0 => {
                    let set = BTreeSet::from([r1.outcome.as_str(), r2.outcome.as_str()]);
                    assert_eq!(set, BTreeSet::from(["COMMIT", "REPLAY"]));
                }
                1..=3 => {
                    let reject = match case {
                        1 => "REJECT_NONCE",
                        2 => "REJECT_PRED",
                        _ => "REJECT_OBJ",
                    };
                    assert_eq!(
                        BTreeSet::from([r1.outcome.as_str(), r2.outcome.as_str()]),
                        BTreeSet::from(["COMMIT", reject])
                    );
                }
                _ => {
                    if r2.sequence < r1.sequence {
                        assert_ne!(r1.outcome, "COMMIT");
                    }
                }
            }
            let state = a.state().unwrap();
            assert!(state.snapshot().a || state.snapshot().b);
            assert_eq!(state.ledger().len(), state.nonces().len());
            assert!(state.ledger().len() <= 1);
        }
        summary.push_str(&format!("case={case} rounds=128 failures=0 COMMIT={commits} REPLAY={replays} REJECT={rejects}\n"));
    }
    evidence("barrier.txt", &summary);
}

#[test]
fn thirty_two_workers_replay_every_actual_transition() {
    let a = Arc::new(Authority::default());
    let active = Arc::new(AtomicUsize::new(33));
    let barrier = Arc::new(Barrier::new(34));
    let executions = thread::scope(|scope| {
        for worker in 0..32_u64 {
            let a = a.clone();
            let active = active.clone();
            let barrier = barrier.clone();
            scope.spawn(move || {
                let _done = Done(active);
                barrier.wait();
                let started = Instant::now();
                for i in 0..256_u64 {
                    let expected = a.snapshot().unwrap();
                    let mutation = match i % 4 {
                        0 => Mutation::Delta(1),
                        1 => Mutation::AOff,
                        2 => Mutation::BOff,
                        _ => Mutation::NoMatch,
                    };
                    let (op, nonce) = if i % 16 == 0 {
                        (1_000_000 + i, 2_000_000 + i)
                    } else {
                        (worker * 1000 + i + 1, worker * 1000 + i + 100_000)
                    };
                    let proposal = Proposal::new(op, nonce, mutation, expected, 1);
                    let ticket = loop {
                        assert!(started.elapsed() < Duration::from_secs(30));
                        match a.submit(proposal.clone(), Tier::High) {
                            Ok(t) => break t,
                            Err(Error::Backpressure(_)) => thread::yield_now(),
                            Err(e) => panic!("{e:?}"),
                        }
                    };
                    wait_ticket(&ticket, started);
                }
            });
        }
        let ca = a.clone();
        let ca_active = active.clone();
        let cb = barrier.clone();
        scope.spawn(move || {
            let _done = Done(ca_active);
            cb.wait();
            let started = Instant::now();
            for i in 0..512 {
                let c = match i % 7 {
                    0 => Control::Cancel,
                    1 => Control::ReuseParent,
                    2 => Control::Revoke,
                    3 => Control::Policy,
                    4 => Control::ObjectChange,
                    5 => Control::InsertMatch,
                    _ => Control::ResetAB,
                };
                let ticket = loop {
                    assert!(started.elapsed() < Duration::from_secs(30));
                    match ca.submit_control(c.clone()) {
                        Ok(t) => break t,
                        Err(Error::Backpressure(_)) => thread::yield_now(),
                        Err(e) => panic!("{e:?}"),
                    }
                };
                wait_ticket(&ticket, started);
            }
        });
        barrier.wait();
        let mut log = vec![];
        while active.load(Ordering::SeqCst) > 0 || a.queued().unwrap() != [0, 0, 0] {
            if let Some(e) = a.service_one().unwrap() {
                log.push(e);
            } else {
                thread::yield_now();
            }
        }
        log
    });
    let mut replay = State::default();
    let mut ops = BTreeSet::new();
    let mut nonces = BTreeSet::new();
    for e in &executions {
        assert_eq!(replay.execute(&e.request).unwrap(), e.response);
        if e.response.outcome == "COMMIT" {
            if let Request::Propose(p) = &e.request {
                assert!(ops.insert(p.operation_id));
                assert!(nonces.insert(p.nonce));
            }
        }
        assert!(replay.snapshot().a || replay.snapshot().b);
    }
    let actual = a.state().unwrap();
    assert_eq!(actual, replay);
    assert_eq!(actual.digest(), replay.digest());
    assert_eq!(actual.ledger().len(), ops.len());
    assert_eq!(actual.nonces().len(), nonces.len());
    assert!(executions.len() >= 512 + 32 * 240);
    assert!(executions.len() <= 512 + 8192);
    let bytes = replica_v3::binary::to_vec(&executions).unwrap();
    assert_eq!(
        replica_v3::binary::from_canonical_slice::<Vec<Execution>>(&bytes).unwrap(),
        executions
    );
    if let Some(root) = std::env::var_os("R3_KERNEL_C_EVIDENCE") {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::PathBuf::from(root).join("actual-executions.r3b"))
            .unwrap();
        f.write_all(&bytes).unwrap();
        f.sync_all().unwrap();
    }
    evidence("stress.txt",&format!("workers=32 attempts=8192 controls=512 executions={} replay_mismatch=0 digest={} commits={} duplicate_effect=0 same_nonce_multi_commit=0\n",executions.len(),actual.digest(),ops.len()));
}
