CREATE TABLE scan_runs (
 id TEXT PRIMARY KEY, started_at TEXT NOT NULL, finished_at TEXT,
 status TEXT NOT NULL CHECK(status IN ('RUNNING','COMPLETE','PARTIAL','FAILED','CANCELLED','INTERRUPTED')),
 scanner_versions TEXT NOT NULL, coverage_json TEXT NOT NULL CHECK(json_valid(coverage_json)), error_summary TEXT
);
CREATE TABLE assets (
 id TEXT PRIMARY KEY, kind TEXT NOT NULL, display_name TEXT NOT NULL, publisher TEXT,
 normalized_id TEXT, source TEXT NOT NULL, installed_version TEXT, install_path_hash TEXT,
 identity_confidence TEXT NOT NULL CHECK(identity_confidence IN ('MATCHED','POTENTIAL','UNKNOWN')),
 provenance_json TEXT NOT NULL CHECK(json_valid(provenance_json)), observed_at TEXT NOT NULL
);
CREATE INDEX assets_identity ON assets(normalized_id, installed_version);
CREATE TABLE scan_assets (
 scan_run_id TEXT NOT NULL REFERENCES scan_runs(id), asset_id TEXT NOT NULL REFERENCES assets(id),
 snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)), PRIMARY KEY(scan_run_id,asset_id)
);
CREATE TABLE environments (id TEXT PRIMARY KEY,kind TEXT NOT NULL,label TEXT NOT NULL,path_hash TEXT,observed_at TEXT NOT NULL);
CREATE TABLE packages (
 id TEXT PRIMARY KEY,environment_id TEXT NOT NULL REFERENCES environments(id),ecosystem TEXT NOT NULL,
 name TEXT NOT NULL,version TEXT,direct INTEGER NOT NULL CHECK(direct IN (0,1)),source TEXT NOT NULL,observed_at TEXT NOT NULL,
 UNIQUE(id,environment_id)
);
CREATE TABLE dependency_edges (
 id TEXT PRIMARY KEY,from_package_id TEXT NOT NULL,to_package_id TEXT NOT NULL,environment_id TEXT NOT NULL REFERENCES environments(id),
 FOREIGN KEY(from_package_id,environment_id) REFERENCES packages(id,environment_id),
 FOREIGN KEY(to_package_id,environment_id) REFERENCES packages(id,environment_id),
 UNIQUE(from_package_id,to_package_id,environment_id)
);
CREATE TABLE advisories (id TEXT PRIMARY KEY,source TEXT NOT NULL,source_url TEXT NOT NULL,published_at TEXT,modified_at TEXT,severity TEXT,raw_cache_key TEXT);
CREATE TABLE affected_ranges (id TEXT PRIMARY KEY,advisory_id TEXT NOT NULL REFERENCES advisories(id),ecosystem TEXT,package_id_or_product TEXT NOT NULL,range_expression TEXT NOT NULL,fixed_version TEXT);
CREATE TABLE findings (
 id TEXT PRIMARY KEY,asset_id TEXT REFERENCES assets(id),package_id TEXT REFERENCES packages(id),advisory_id TEXT REFERENCES advisories(id),
 applicability TEXT NOT NULL CHECK(applicability IN ('CONFIRMED_APPLICABLE_VULNERABILITY','POTENTIAL','UNKNOWN','NOT_AFFECTED','UPDATE_AVAILABLE')),
 confidence TEXT NOT NULL CHECK(confidence IN ('HIGH','MEDIUM','LOW','UNKNOWN')),
 priority TEXT NOT NULL CHECK(priority IN ('P0','P1','P2','UPDATE_ONLY','REVIEW')),
 evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json)),observed_at TEXT NOT NULL,
 CHECK((asset_id IS NOT NULL AND package_id IS NULL) OR (asset_id IS NULL AND package_id IS NOT NULL))
);
CREATE TABLE scan_observations (id TEXT PRIMARY KEY,scan_run_id TEXT NOT NULL REFERENCES scan_runs(id),finding_id TEXT NOT NULL REFERENCES findings(id),status TEXT NOT NULL);
CREATE INDEX findings_applicability ON findings(applicability,priority);
CREATE INDEX scan_runs_time ON scan_runs(started_at);
