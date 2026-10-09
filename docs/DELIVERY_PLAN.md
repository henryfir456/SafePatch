# Delivery Plan v0.1

## Milestones
- M1: scaffold, SQLite migrations, Windows Registry/WinGet inventory, live dashboard vertical slice.
- M2: VS Code/Extensions, multi-Node, npm lockfile, Python, Git inventory.
- M3: OSV + vendor advisories + NVD/KEV correlation, evidence-backed risk engine.
- M4: scan history, filters, JSON/CSV export, cancellation and offline states.
- M5: scheduled scans, notifications, Windows installer, CI/security regression and release gate.

## Execution discipline
Implement the smallest real end-to-end M1 flow first. One issue per traceable slice. Before coding, review current repo/issue state; ensure AGENTS.md rules and issue linkage. Prefer tested direct-main changes only when safe and consistent with branch protections; otherwise use a PR. No auto-merge. Every completed milestone requires observable Windows evidence and regression record. Unvalidated tests remain explicitly pending.

## Constraints
No mandatory telemetry/account/cloud server. Windows 10/11 support targets; no unsupported claim of automatic secure updates. Future Android version is out of v0.1 scope.
