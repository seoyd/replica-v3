use crate::{
    Error, Result,
    event::*,
    store::{self, Store},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    time::{Duration, Instant},
};
pub const CANDIDATES: usize = 64;
pub const VISITED: usize = 256;
pub const HOPS: usize = 4;
#[derive(Clone, Debug)]
pub struct Search {
    pub query: String,
    pub scope: String,
    pub session: Option<String>,
    pub slot: Option<Slot>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub snapshot_id: Option<i64>,
    pub history: bool,
    pub graph: bool,
    pub valid_at: i64,
}
impl Search {
    pub fn new(scope: &str, query: &str) -> Self {
        Self {
            query: query.into(),
            scope: scope.into(),
            session: None,
            slot: None,
            after: None,
            before: None,
            snapshot_id: None,
            history: false,
            graph: true,
            valid_at: now_ms(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationStep {
    pub from: i64,
    pub to: i64,
    pub relation: RelationKind,
    pub origin: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub event_id: i64,
    pub original_excerpt: String,
    pub excerpt_truncated: bool,
    pub source: String,
    pub recorded_at: i64,
    pub observed_at: Option<i64>,
    pub version_status: String,
    pub retrieval_reason: String,
    pub relation_path: Vec<RelationStep>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub items: Vec<Evidence>,
    pub truncated: bool,
    pub visited: usize,
    #[serde(default)]
    pub candidates_fetched: usize,
    #[serde(default)]
    pub edges_fetched: usize,
    #[serde(default)]
    pub eligible: usize,
}
const FILTER: &str = "m.scope=?1 AND (?2 IS NULL OR m.session=?2) AND (?3 IS NULL OR m.slot=?3) AND m.recorded_at>=?4 AND m.recorded_at<=?5 AND m.id<=?6 AND (?7 OR (m.kind IN (0,1,2) AND m.question=0 AND (m.kind!=1 OR (m.id=(SELECT max(v.id) FROM record_meta v WHERE v.slot=m.slot AND v.id<=?6 AND v.recorded_at<=?5) AND (m.valid_from IS NULL OR m.valid_from<=?8) AND (m.valid_until IS NULL OR m.valid_until>?8)))))";
impl Store {
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
        let long: Vec<_> = terms
            .iter()
            .filter(|s| s.chars().count() >= 3)
            .map(|s| format!("\"{}\"", s.to_lowercase().replace('"', "\"\"")))
            .collect();
        let deadline = Instant::now() + Duration::from_millis(100);
        self.conn
            .progress_handler(1000, Some(move || Instant::now() >= deadline));
        let result = (|| {
            let tx = self.conn.unchecked_transaction()?;
            let slot = q.slot.as_ref().map(|s| s.key(&q.scope));
            let snapshot = q.snapshot_id.unwrap_or(i64::MAX);
            let after = q.after.unwrap_or(i64::MIN);
            let before = q.before.unwrap_or(i64::MAX);
            let mut bundle = EvidenceBundle::default();
            let mut seeds = Vec::new();
            let sql = if long.is_empty() {
                format!("SELECT m.id FROM record_meta m WHERE {FILTER} ORDER BY m.id DESC LIMIT 65")
            } else {
                // Keep FTS as the outer loop: metadata-first plans repeat MATCH
                // for every row and sort the entire result before LIMIT applies.
                format!(
                    "SELECT m.id FROM record_fts CROSS JOIN record_meta m ON m.id=record_fts.rowid WHERE {FILTER} AND record_fts MATCH ?9 ORDER BY rank LIMIT 65"
                )
            };
            let mut stmt = tx.prepare(&sql)?;
            let mut rows = if long.is_empty() {
                stmt.query(params![
                    q.scope, q.session, slot, after, before, snapshot, q.history, q.valid_at
                ])?
            } else {
                stmt.query(params![
                    q.scope,
                    q.session,
                    slot,
                    after,
                    before,
                    snapshot,
                    q.history,
                    q.valid_at,
                    long.join(" OR ")
                ])?
            };
            loop {
                match rows.next() {
                    Ok(Some(r)) => seeds.push(r.get::<_, i64>(0)?),
                    Ok(None) => break,
                    Err(e) if interrupted(&e) => {
                        bundle.truncated = true;
                        break;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            bundle.candidates_fetched = seeds.len();
            if seeds.len() > CANDIDATES {
                if long.is_empty() {
                    return Err(Error::NarrowScope);
                }
                seeds.truncate(CANDIDATES);
                bundle.truncated = true;
            }
            if long.is_empty() {
                let lower = q.query.to_lowercase();
                seeds.retain(|id| match store::get(&tx, *id, true) {
                    Ok(e) => store::normalized(&e).contains(&lower),
                    Err(_) => true,
                });
            }
            let mut queue: VecDeque<(i64, Vec<RelationStep>)> =
                seeds.into_iter().map(|id| (id, Vec::new())).collect();
            let mut visited = HashSet::new();
            let mut scheduled: HashSet<_> = queue.iter().map(|(id, _)| *id).collect();
            while let Some((id, path)) = queue.pop_front() {
                if Instant::now() >= deadline || visited.len() >= VISITED {
                    bundle.truncated = true;
                    break;
                }
                if !visited.insert(id) {
                    continue;
                }
                let eligible_sql =
                    format!("SELECT m.id FROM record_meta m WHERE {FILTER} AND m.id=?9");
                let eligible = tx
                    .query_row(
                        &eligible_sql,
                        params![
                            q.scope, q.session, slot, after, before, snapshot, q.history,
                            q.valid_at, id
                        ],
                        |r| r.get::<_, i64>(0),
                    )
                    .optional()?;
                if eligible.is_none() {
                    continue;
                }
                let e = store::get(&tx, id, true)?;
                bundle.eligible += 1;
                if bundle.items.len() < MAX_EVIDENCE {
                    let (excerpt, truncated) =
                        store::prefix(std::str::from_utf8(&e.payload).expect("validated"), 4096);
                    let status = if let Some(s) = e.kind.slot() {
                        let last:i64=tx.query_row("SELECT max(id) FROM record_meta WHERE slot=?1 AND id<=?2 AND recorded_at<=?3",params![s.key(&e.scope),snapshot,before],|r|r.get(0))?;
                        if last != id {
                            "superseded"
                        } else if !store::valid_at_time(&e, q.valid_at) {
                            "outside_valid_time"
                        } else {
                            "current"
                        }
                    } else {
                        "observation_or_interpretation"
                    };
                    bundle.items.push(Evidence {
                        event_id: id,
                        original_excerpt: excerpt.into(),
                        excerpt_truncated: truncated,
                        source: e.source,
                        recorded_at: e.recorded_at,
                        observed_at: e.observed_at,
                        version_status: status.into(),
                        retrieval_reason: if path.is_empty() {
                            "lexical"
                        } else {
                            "stored_relation"
                        }
                        .into(),
                        relation_path: path.clone(),
                    });
                } else {
                    bundle.truncated = true;
                }
                if !q.graph {
                    continue;
                }
                let mut edges=tx.prepare("SELECT r.origin,r.from_id,r.to_id,r.kind FROM relations r JOIN record_meta m ON m.id=r.origin WHERE (r.from_id=?1 OR r.to_id=?1) AND r.origin<=?2 AND m.scope=?3 AND (?4 IS NULL OR m.session=?4) AND m.recorded_at>=?5 AND m.recorded_at<=?6 ORDER BY r.origin,r.from_id,r.to_id,r.kind LIMIT 257")?;
                let mapped = edges.query_map(
                    params![id, snapshot, q.scope, q.session, after, before],
                    |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, i64>(1)?,
                            r.get::<_, i64>(2)?,
                            r.get::<_, u8>(3)?,
                        ))
                    },
                )?;
                for (fetched, edge) in mapped.enumerate() {
                    let (origin, from, to, kind) = edge?;
                    bundle.edges_fetched += 1;
                    if fetched == VISITED {
                        bundle.truncated = true;
                        break;
                    }
                    let origin_event = store::get(&tx, origin, true)?;
                    if origin_event.scope != q.scope
                        || q.session
                            .as_ref()
                            .is_some_and(|s| *s != origin_event.session)
                        || origin_event.recorded_at > before
                        || origin_event.recorded_at < after
                    {
                        continue;
                    }
                    let next = if from == id { to } else { from };
                    if scheduled.contains(&next) {
                        continue;
                    }
                    // At the hop boundary an unexamined neighbor can still be
                    // ineligible; report that uncertainty conservatively.
                    if path.len() >= HOPS || scheduled.len() >= VISITED {
                        bundle.truncated = true;
                        continue;
                    }
                    let mut next_path = path.clone();
                    next_path.push(RelationStep {
                        from,
                        to,
                        relation: RelationKind::from_tag(kind)?,
                        origin,
                    });
                    scheduled.insert(next);
                    queue.push_back((next, next_path));
                }
            }
            bundle.visited = visited.len();
            Ok(bundle)
        })();
        self.conn.progress_handler(0, None::<fn() -> bool>);
        match result {
            Err(Error::Sql(e)) if interrupted(&e) => Ok(EvidenceBundle {
                truncated: true,
                ..Default::default()
            }),
            other => other,
        }
    }
}
fn interrupted(e: &rusqlite::Error) -> bool {
    matches!(e,rusqlite::Error::SqliteFailure(code,_)if code.code==rusqlite::ErrorCode::OperationInterrupted)
}
