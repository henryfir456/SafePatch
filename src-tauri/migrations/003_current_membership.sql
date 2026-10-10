-- Preserve preview v2 catalogs and immutable snapshots; rebuild current membership.
CREATE TABLE IF NOT EXISTS environment_current_packages (
 environment_id TEXT NOT NULL REFERENCES environments(id),package_id TEXT NOT NULL,
 PRIMARY KEY(environment_id,package_id),
 FOREIGN KEY(package_id,environment_id) REFERENCES packages(id,environment_id)
);
CREATE TEMP VIEW safepatch_v3_latest AS
 SELECT g.* FROM scan_project_graphs g JOIN scan_runs s ON s.id=g.scan_run_id
 WHERE s.status IN ('COMPLETE','PARTIAL') AND NOT EXISTS (
  SELECT 1 FROM scan_project_graphs newer JOIN scan_runs ns ON ns.id=newer.scan_run_id
  WHERE newer.environment_id=g.environment_id AND ns.status IN ('COMPLETE','PARTIAL')
   AND (ns.started_at>s.started_at OR (ns.started_at=s.started_at AND ns.id>s.id))
 );
INSERT INTO packages(id,environment_id,ecosystem,name,version,direct,source,observed_at,direct_status,slot_hash)
 SELECT json_extract(p.value,'$.id'),g.environment_id,'npm',json_extract(p.value,'$.name'),
  json_extract(p.value,'$.version'),json_extract(p.value,'$.direct_status')='DIRECT',
  json_extract(p.value,'$.source'),json_extract(g.snapshot_json,'$.observed_at'),
  json_extract(p.value,'$.direct_status'),json_extract(p.value,'$.slot_hash')
 FROM safepatch_v3_latest g JOIN json_each(g.snapshot_json,'$.packages') p WHERE 1
 ON CONFLICT(id) DO UPDATE SET name=excluded.name,version=excluded.version,direct=excluded.direct,
  source=excluded.source,observed_at=excluded.observed_at,direct_status=excluded.direct_status,slot_hash=excluded.slot_hash;
DELETE FROM environment_current_packages;
INSERT INTO environment_current_packages
 SELECT g.environment_id,json_extract(p.value,'$.id')
 FROM safepatch_v3_latest g JOIN json_each(g.snapshot_json,'$.packages') p;
DROP VIEW safepatch_v3_latest;
