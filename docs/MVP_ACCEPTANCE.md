# MVP Acceptance v0.1

A green gate needs repeatable test evidence, not a screenshot or a successful build alone.

| ID | Testable acceptance criterion |
|---|---|
| AC-01 | Tauri desktop application installs and launches on Windows 10/11 without mandatory elevation. |
| AC-02 | Registry + WinGet inventory reconciles duplicates, preserves provenance and multiple versions. |
| AC-03 | VS Code and extensions report exact ID, version, profile/source when discoverable. |
| AC-04 | Multiple Node.js runtimes are discoverable or explicitly marked out of coverage. |
| AC-05 | npm lockfile direct/transitive dependencies and exact versions are parsed without executing install scripts. |
| AC-06 | Selected Python interpreters, installed packages and virtual environments can be scanned safely. |
| AC-07 | Separate Git installations/versions are inventoried. |
| AC-08 | Findings show advisory ID, source URL, affected-version rule, matched installed version, and remediation evidence. |
| AC-09 | Same-name different-vendor false matches are rejected; UNKNOWN/POTENTIAL cases never become confirmed vulnerability. |
| AC-10 | Dashboard filters by product, scanner, priority, confidence and applicability. |
| AC-11 | SQLite stores at least 30 scan runs and history survives restart. |
| AC-12 | Scans do not change installed programs, projects or package dependencies; no auto-fix or elevation. |
| AC-13 | Offline scans preserve timestamped cache and explicitly mark unverified/stale coverage. |
| AC-14 | JSON and CSV exports work with redaction enabled by default. |
| AC-15 | Regression tests cover version ranges, duplicates, partial failures, multi-runtime, offline, poisoned names and untrusted metadata. |

## Evidence gate
Each issue/PR documents tests run, platform, relevant logs, limitations, and manual steps. Do not claim Windows validation if only Linux CI ran. Support cancellation, bounded timeouts, and clear partial-coverage outcomes.
