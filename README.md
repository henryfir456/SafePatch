# SafePatch

v0.3.0 Preview (#28/#29; publication is confirmed only by the [GitHub prerelease](https://github.com/henryfir456/SafePatch/releases/tag/v0.3.0-preview)): opt-in OSV/NVD/KEV security assessment with independent applicability/priority, evidence details, UNKNOWN dimensions and embedded application version/build identity. [Actual verification and limits](docs/SECURITY_HEALTH_VERIFICATION.md). Lockfile declarations never prove installation; controlled Windows product mappings do not establish installation origin. PRs #30/#31 track implementation, CI, independent review and real Windows evidence. Python PR #21 remains paused.

Local-first Windows software update health and developer-environment inventory.

The v0.2.0 Preview milestone (Issue #26) adds read-only WinGet update evidence and an update-first dashboard. See [update semantics and actual verification](docs/UPDATE_HEALTH_VERIFICATION.md) and [繁體中文 CHANGELOG](CHANGELOG.md). That milestone left vulnerabilities NOT_CHECKED; v0.3 adds the separate opt-in assessment. NO_UPDATE_REPORTED does not guarantee latest.

Previously released Windows Preview 0.1.0 packages only the verified Windows, VS Code, Node.js and npm inventory on main. [Preview installation, hashes and verification limits](docs/WINDOWS_PREVIEW.md) explain the current-user EXE/MSI packages and WebView2 prerequisite. No Rust/Node/VS Code development environment is needed to run the installed app. Python/Git, CVE analysis and automatic security updates are not included; the broader scope below is the paused roadmap.

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
M1's real inventory vertical slice is implemented and passed automated native WebView acceptance on Windows 11: Tauri/React UI, Rust scan service, SQLite migrations, read-only Windows Registry/WinGet scanning, cancellation and restart persistence. The UI contains no simulated device scan. WinGet and security source coverage remain explicitly PARTIAL; v0.3 adds opt-in OSV/NVD/KEV assessment with conservative UNKNOWN/POTENTIAL results. Windows 10, human-visible/manual UI review and installer acceptance are not yet verified.

See [Windows development and operation](docs/WINDOWS_DEVELOPMENT.md) and the explicit [M1 verification record](docs/M1_VERIFICATION.md). Missing tools or a passing fixture test do not establish Windows end-to-end acceptance. Related issues: #2, #3, #4, #13.

M2 #5 adds read-only VS Code/application/extension/profile metadata to the same scan and passed Windows 11 native WebView scan/cancel/restart acceptance. See [VS Code verification and coverage](docs/M2_VSCODE_VERIFICATION.md) for evidence and explicit limitations. Python and Git collectors remain pending (#8–#9).

M2 #6 adds distinct Node runtime metadata and offline support lifecycle and passed Windows 11 native WebView scan/cancel/restart acceptance with two actual runtime paths and two distinct observed version values. See [Node verification/coverage](docs/M2_NODE_VERIFICATION.md) for evidence, the observed WinGet timeout/recovery and coverage limits. Support lifecycle is independent of vulnerability applicability.


M2 #7 adds opt-in npm lockfile inventory and passed Windows 11 native WebView scan/cancel/restart acceptance with 159 actual declarations and 189 dependency edges. [npm verification and limits](docs/M2_NPM_VERIFICATION.md) distinguishes lockfile declarations from installed packages and records read-only metadata fingerprints, exact SQLite/graph comparisons and synthetic pagination tests separately. Python and Git are pending (#8–#9).
