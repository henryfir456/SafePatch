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
-- Package catalog rows can be retained for findings/history references.
-- Only this membership defines packages in the latest successful environment graph.
CREATE TABLE environment_current_packages (
 environment_id TEXT NOT NULL REFERENCES environments(id),package_id TEXT NOT NULL,
 PRIMARY KEY(environment_id,package_id),
 FOREIGN KEY(package_id,environment_id) REFERENCES packages(id,environment_id)
);
CREATE INDEX scan_runs_scope_time ON scan_runs(scope,started_at);
