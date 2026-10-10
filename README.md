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
M1's real inventory vertical slice is implemented and passed automated native WebView acceptance on Windows 11: Tauri/React UI, Rust scan service, SQLite migrations, read-only Windows Registry/WinGet scanning, cancellation and restart persistence. The UI contains no simulated device scan. WinGet catalog coverage is explicitly PARTIAL; vulnerability assessment remains UNKNOWN until later milestones. Windows 10, human-visible/manual UI review and installer acceptance are not yet verified.

See [Windows development and operation](docs/WINDOWS_DEVELOPMENT.md) and the explicit [M1 verification record](docs/M1_VERIFICATION.md). Missing tools or a passing fixture test do not establish Windows end-to-end acceptance. Related issues: #2, #3, #4, #13.

M2 #5 adds read-only VS Code/application/extension/profile metadata to the same scan and passed Windows 11 native WebView scan/cancel/restart acceptance. See [VS Code verification and coverage](docs/M2_VSCODE_VERIFICATION.md) for evidence and explicit limitations. npm, Python and Git collectors remain pending (#7–#9).

M2 #6 adds distinct Node runtime metadata and offline support lifecycle and passed Windows 11 native WebView scan/cancel/restart acceptance with two actual runtime paths and two distinct observed version values. See [Node verification/coverage](docs/M2_NODE_VERIFICATION.md) for evidence, the observed WinGet timeout/recovery and coverage limits. Support lifecycle is independent of vulnerability applicability.


M2 #7 adds opt-in npm lockfile inventory and passed Windows 11 native WebView scan/cancel/restart acceptance with 159 actual declarations and 189 dependency edges. [npm verification and limits](docs/M2_NPM_VERIFICATION.md) distinguishes lockfile declarations from installed packages and records read-only metadata fingerprints, exact SQLite/graph comparisons and synthetic pagination tests separately. Python and Git are pending (#8–#9).
