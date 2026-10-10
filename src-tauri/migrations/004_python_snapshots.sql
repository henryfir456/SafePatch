-- SQLite's documented create/copy/drop/rename migration. Caller temporarily
-- disables foreign keys outside the transaction, checks all FKs before commit,
-- then restores the original setting even if this transaction fails.
CREATE TABLE scan_runs_v4 (
 id TEXT PRIMARY KEY, started_at TEXT NOT NULL, finished_at TEXT,
 status TEXT NOT NULL CHECK(status IN ('RUNNING','COMPLETE','PARTIAL','FAILED','CANCELLED','INTERRUPTED')),
 scanner_versions TEXT NOT NULL, coverage_json TEXT NOT NULL CHECK(json_valid(coverage_json)), error_summary TEXT,
 scope TEXT NOT NULL DEFAULT 'windows' CHECK(scope IN ('windows','npm','python'))
);
INSERT INTO scan_runs_v4 SELECT id,started_at,finished_at,status,scanner_versions,coverage_json,error_summary,scope FROM scan_runs;
DROP TABLE scan_runs;
ALTER TABLE scan_runs_v4 RENAME TO scan_runs;
CREATE INDEX scan_runs_time ON scan_runs(started_at);
CREATE INDEX scan_runs_scope_time ON scan_runs(scope,started_at);
CREATE TABLE scan_python_snapshots (
 scan_run_id TEXT PRIMARY KEY REFERENCES scan_runs(id),
 environment_id TEXT NOT NULL REFERENCES environments(id),
 snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json))
);
