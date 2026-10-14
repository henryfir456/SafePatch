-- Independent advisory snapshots preserve legacy inventory schema and history.
CREATE TABLE security_runs (
 id TEXT PRIMARY KEY, started_at TEXT NOT NULL, finished_at TEXT,
 status TEXT NOT NULL CHECK(status IN ('RUNNING','COMPLETE','PARTIAL','FAILED','CANCELLED','INTERRUPTED')),
 report_json TEXT CHECK(report_json IS NULL OR json_valid(report_json)), error_code TEXT
);
CREATE INDEX security_runs_time ON security_runs(started_at);
CREATE TABLE security_cache (
 key TEXT PRIMARY KEY, fetched_at TEXT NOT NULL, body TEXT NOT NULL CHECK(json_valid(body))
);
