# SafePatch

Local-first Windows software and developer-environment security health checker.

## Scope
- Windows 10/11 installed software inventory via WinGet and Registry
- VS Code and extension inventory
- Node.js runtimes, npm lockfile dependencies, Python interpreters/venvs, and Git installations
- Evidence-backed vulnerability correlation (OSV, NVD, CISA KEV, vendor advisories)
- SQLite history, risk dashboard, and opt-in exports

## Principles
Read-only scans by default; no automatic upgrades, arbitrary project execution, cloud uploads, or admin elevation. Vulnerability applicability is distinct from availability of an update. Unknown is **not** safe.

## Stack
Tauri 2 / React / TypeScript / Rust / SQLite. See [Architecture](docs/ARCHITECTURE.md), [MVP Acceptance](docs/MVP_ACCEPTANCE.md), [Security Model](docs/SECURITY_MODEL.md), [Delivery Plan](docs/DELIVERY_PLAN.md), and [Agent Rules](AGENTS.md).

## Status
Design/bootstrap stage; no operational scanner has been implemented yet.
