CREATE TABLE IF NOT EXISTS records(id INTEGER PRIMARY KEY CHECK(id>0), body BLOB NOT NULL);
CREATE TRIGGER IF NOT EXISTS immutable_update BEFORE UPDATE ON records BEGIN SELECT RAISE(ABORT,'canonical records are immutable'); END;
CREATE TRIGGER IF NOT EXISTS immutable_delete BEFORE DELETE ON records BEGIN SELECT RAISE(ABORT,'canonical records are immutable'); END;
CREATE TABLE IF NOT EXISTS projection_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1), version INTEGER NOT NULL, last_id INTEGER NOT NULL);
INSERT OR IGNORE INTO projection_state VALUES(1,1,0);
CREATE TABLE IF NOT EXISTS record_meta(
 id INTEGER PRIMARY KEY REFERENCES records(id), kind INTEGER NOT NULL, scope TEXT NOT NULL,
 session TEXT NOT NULL, source TEXT NOT NULL, recorded_at INTEGER NOT NULL, observed_at INTEGER,
 slot BLOB, question INTEGER NOT NULL, valid_from INTEGER, valid_until INTEGER);
CREATE INDEX IF NOT EXISTS meta_scope_time ON record_meta(scope,recorded_at,id);
CREATE INDEX IF NOT EXISTS meta_slot ON record_meta(slot,id);
CREATE TABLE IF NOT EXISTS current_heads(slot BLOB PRIMARY KEY, event_id INTEGER NOT NULL REFERENCES records(id));
CREATE TABLE IF NOT EXISTS relations(origin INTEGER NOT NULL REFERENCES records(id), from_id INTEGER NOT NULL REFERENCES records(id), to_id INTEGER NOT NULL REFERENCES records(id), kind INTEGER NOT NULL, PRIMARY KEY(origin,from_id,to_id,kind));
CREATE INDEX IF NOT EXISTS edges_from ON relations(from_id,origin,to_id);
CREATE INDEX IF NOT EXISTS edges_to ON relations(to_id,origin,from_id);
CREATE TABLE IF NOT EXISTS request_state(scope TEXT NOT NULL, request_key BLOB NOT NULL, event_id INTEGER NOT NULL REFERENCES records(id), PRIMARY KEY(scope,request_key));
CREATE TABLE IF NOT EXISTS results(input_id INTEGER PRIMARY KEY REFERENCES records(id), event_id INTEGER UNIQUE NOT NULL REFERENCES records(id));
CREATE VIRTUAL TABLE IF NOT EXISTS record_fts USING fts5(text, tokenize='trigram');
