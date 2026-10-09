# Architecture v0.1

## System boundaries
React/TypeScript renderer -> narrow Tauri IPC commands -> Rust application layer -> inventory adapters / dev scanners / advisory adapters / risk engine -> SQLite.

## Inventory adapters
- Windows: WinGet JSON/list where reliably available, uninstall registry keys in HKLM/HKCU (including 32-bit views). Normalize publisher, product, version and source without conflating separately installed versions.
- VS Code: installed binaries/profiles, `code --list-extensions --show-versions` (with explicit CLI paths); marketplace identity = publisher.extension; report unknown provenance where applicable.
- Node: enumerate PATH and known version-manager installations; do not assume a single `node --version` covers all installs.
- npm: parse exact versions and full dependency graph from recognized lockfile versions; first MVP supports package-lock/npm-shrinkwrap and reports unsupported pnpm/Yarn formats as coverage gaps. Do not execute npm install/scripts as part of inspection.
- Python: explicit interpreter/venv discovery; inspect installed distribution metadata or safely query interpreter with controlled commands; avoid installing dependencies during scan.
- Git: resolve binary paths and versions, distinguish Git for Windows from bundled Git.

## Advisory adapters
OSV ecosystem/version batch queries for npm and PyPI; vendor security advisories for runtimes/desktop products; NVD/CVE and CISA KEV are corroboration and exploitation-priority signals. Apply rate limits, caching, timestamps, source URLs, and retry/backoff. An unavailable service yields STALE or UNKNOWN rather than 'no vulnerabilities'.

## Correlation and assessment
Normalize ecosystem/package identity, product vendor, installed version, ranges and fixes. Matching states: MATCHED, POTENTIAL, NOT_AFFECTED, UNKNOWN; independently record evidence confidence and risk priority P0/P1/P2/UPDATE_ONLY/REVIEW. Do not infer affected from name match or CVSS alone. KEV affects priority only after applicable-version evidence.

## Data model (initial)
- assets(id,kind,display_name,publisher,normalized_id,source,installed_version,install_path_hash,observed_at)
- environments(id,kind,label,path_hash,observed_at)
- packages(id,environment_id,ecosystem,name,version,direct,source,observed_at)
- dependency_edges(id,from_package_id,to_package_id,environment_id)
- advisories(id,source,source_url,published_at,modified_at,severity,raw_cache_key)
- affected_ranges(id,advisory_id,ecosystem,package_id_or_product,range_expression,fixed_version)
- findings(id,asset_or_package_id,advisory_id,applicability,confidence,priority,evidence_json,observed_at)
- scan_runs(id,started_at,finished_at,status,scanner_versions,coverage_json,error_summary)
- scan_observations(id,scan_run_id,finding_id,status)

Introduce migrations, referential integrity, indexes, and tested upgrades; never store secrets or raw environment variables. Hash paths in reports by default; local storage paths are user-controlled.

## IPC design
Commands: start_scan(scope), cancel_scan(scan_id), scan_status(scan_id), list_assets(filters), list_findings(filters), finding_detail(id), export_report(format,redaction). Long scans use background Rust tasks and progress events. Validate IPC args and avoid arbitrary shell command input.

## Release strategy
M1 Windows inventory + working UI; M2 developer scanners; M3 advisory/risk; M4 history/report; M5 scheduler/release. Windows-host tests are needed for real-world acceptance; non-Windows CI does not count as that evidence.
