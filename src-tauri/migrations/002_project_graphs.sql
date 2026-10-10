ALTER TABLE scan_runs ADD COLUMN scope TEXT NOT NULL DEFAULT 'windows' CHECK(scope IN ('windows','npm'));
ALTER TABLE packages ADD COLUMN direct_status TEXT NOT NULL DEFAULT 'UNKNOWN' CHECK(direct_status IN ('DIRECT','TRANSITIVE','UNKNOWN'));
UPDATE packages SET direct_status='DIRECT' WHERE direct=1;
ALTER TABLE packages ADD COLUMN slot_hash TEXT;
ALTER TABLE dependency_edges ADD COLUMN kinds_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(kinds_json));
CREATE TABLE scan_project_graphs (
 scan_run_id TEXT PRIMARY KEY REFERENCES scan_runs(id),
 environment_id TEXT NOT NULL REFERENCES environments(id),
 snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json))
);
CREATE INDEX scan_runs_scope_time ON scan_runs(scope,started_at);
