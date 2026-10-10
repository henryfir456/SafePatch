# Windows development and M1 verification

Related issues: #2, #3, #4, #13; parent #1.

## Prerequisites

Windows 10/11, Node.js 24, official Rust stable MSVC toolchain, Microsoft C++ Build Tools with Desktop development with C++ / Windows SDK, and WebView2. Install missing tools only with the host owner's authorization. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) describes the official sources. WinGet is optional: absence, untrusted signature, unavailable source agreements, or timeout yields partial coverage, never invented results.

Clone into a dedicated task directory. This repo has no need to touch other projects or stop their processes.

```powershell
npm ci --ignore-scripts
npm run build
npm test
npm run test:schema
powershell -NoProfile -NonInteractive -File scripts/test-adapter.ps1
powershell -NoProfile -NonInteractive -File scripts/test-vscode.ps1
powershell -NoProfile -NonInteractive -File scripts/test-node.ps1
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
npm run tauri -- dev
npm run tauri -- build --no-bundle -- --locked
```

Dependencies are pinned in npm's lockfile, fetched from registry.npmjs.org, and installed with lifecycle scripts disabled. Vite uses the declared platform binary dependency. Rust crates are fetched from crates.io; Rust build scripts compile only declared application dependencies. They are never executed for scanned projects. CI action revisions are pinned. Cargo.lock was recovered from the digest-verified initial Windows CI artifact and committed; CI uses `--locked`.

## Operate

Launch `src-tauri/target/release/safepatch.exe` as the current user. Click **開始唯讀掃描**. The button locks during a scan; **取消掃描** requests cancellation. The table contains actual adapter output, exact observed versions, publisher when available, scope/view, catalog provenance and candidate duplicates. Search/filter operates on the local snapshot. Browser preview cannot scan and displays BACKEND_UNAVAILABLE instead of sample data.

The database is in the current user's application local-data directory under the Tauri identifier `io.github.henryfir456.safepatch`. It inherits that user profile's ACL; no shared/public storage or admin requirement is introduced. Exports and advisory calls are absent in M1. Restart restores the latest snapshot; abandoned RUNNING scans become INTERRUPTED. A scan commits assets, history snapshots and terminal status in one SQLite transaction. Older snapshots remain immutable.

## Adapter safety and coverage

After verifying the executable signature, local `source export --name winget` must match the official URL, pre-indexed type, identifier and data before inventory export. A custom source reusing the name is rejected with UNKNOWN coverage; source configuration is never changed. Source verification has a 15-second deadline and cancellable owned-process cleanup. [Microsoft source documentation](https://learn.microsoft.com/en-us/windows/package-manager/winget/source) documents the JSON metadata.

Registry uses .NET RegistryKey read-only APIs in HKLM/HKCU, Registry32/Registry64. Missing DisplayName entries are outside displayed-product coverage. It never reads/executes uninstall commands, DisplayIcon binaries or project manifests. InstallLocation and registry key names are hashed before crossing IPC; no raw user path is persisted. Registry enumeration is bounded to 30 seconds / 20,000 displayed entries.

The fixed adapter discovers the registered Microsoft Desktop App Installer and checks a valid Microsoft Authenticode signer before running its WinGet executable. PATH is not used. It runs `export --include-versions --source winget --disable-interactivity` into a short-lived local temporary JSON file, drains/discards command output, and deletes the file in `finally`. No install, upgrade, source agreement acceptance, source modification or elevation is requested. WinGet itself may use its own local logs/cache and fetch official source metadata; SafePatch never submits inventory to a service. Command errors are stable codes without stdout/stderr/path disclosure. [Microsoft WinGet list documentation](https://learn.microsoft.com/en-us/windows/package-manager/winget/list) explains that catalog association includes applications installed by other means.

WinGet export provides catalog ID and version, not product display name, Publisher, install method, all installed instances or packages unrecognized by that catalog. Display name for a catalog-only record is its ID; source `winget` means catalog provenance only. Non-winget catalogs (including Store/custom REST sources) are deliberately not contacted in M1. Coverage remains PARTIAL. Registry inventory supplements those gaps. True install origin remains UNKNOWN. Registry `WinGetPackageIdentifier` is used only if present; arbitrary names do not establish vendor identity.

Reflected Registry views merge only for the same key hash, scope, nonempty install path hash, exact product/publisher/version metadata. Multiple paths, scopes and versions remain separate. Exact identity/version candidates are grouped as POTENTIAL and retained; without strong evidence, Registry and catalog records are not silently collapsed. Same-name different-vendor records are not grouped by name alone. This conservative limitation is visible in the UI. M1 does not provide vulnerability findings or a safe verdict.

Cancellation is checked per Registry entry and every 100 ms during WinGet. WinGet has a 45-second deadline and its own child process is reaped. The Rust adapter has a 100-second outer watchdog and an 8 MiB output limit. An owned Windows Job is assigned before providing the fixed script; kill-on-close contains/reaps only our PowerShell and its descendants, including on app exit or timeout. Job-assignment failure refuses the scan. SafePatch terminates only subprocesses it creates. No foreign process is stopped. The adapter restricts PSModulePath to its trusted Windows system modules for that process only.

## Repeatable actual Windows checks

```powershell
# Outputs only counts, OS build, coverage and stable codes; no software list.
powershell -NoProfile -NonInteractive -File scripts/verify-windows-adapter.ps1 -RequireWinGet
# Through production Rust scanner, normalization, SQLite and restart persistence:
cargo run --locked --manifest-path src-tauri/Cargo.toml --no-default-features --bin inventory_probe
```

Then launch the desktop app, start a scan, confirm actual rows/versions and partial errors are rendered, cancel a second scan, restart and verify persistence. Keep full inventory/screen content local. Public acceptance reports may include aggregate counts and coverage codes only. A unit test, a hosted runner build or the standalone adapter does not establish desktop end-to-end acceptance; record each gate separately in `docs/M1_VERIFICATION.md` and the linked Issues. Installer generation is reserved for M5.

When the local compiler is unavailable, download **this repository's** CI artifact and verify its archive SHA-256 against GitHub's artifact digest. It contains the production executable, real Rust integration probe, and a separate `acceptance`-feature executable. The latter uses a hidden native WebView, real DOM click events, production IPC/adapter/normalization/SQLite, and checks rendered rows/versions, cancellation, partial status and process-restart persistence. It has no device fixtures. Test-only IPC and scripts are excluded from the production binary. Run the following as the current user, without elevation, in the isolated artifact directory:

```powershell
.\inventory_probe.exe
.\inventory_probe.exe --cancel
$check=Start-Process .\safepatch-acceptance.exe -ArgumentList '--acceptance-check' -WindowStyle Hidden -PassThru -Wait
$check.ExitCode # 0 means the automated desktop gate passed
$restart=Start-Process .\safepatch-acceptance.exe -ArgumentList '--acceptance-restart' -WindowStyle Hidden -PassThru -Wait
$restart.ExitCode
```

The acceptance binary writes only local `desktop-acceptance.sqlite`, `desktop-acceptance.json`, and `desktop-restart.json` beside itself, bounded to 170 seconds. JSON reports contain only counts, internal scan ID and booleans, never the inventory. This is automated native-WebView acceptance, distinct from human-visible/manual UI review and installer acceptance. The probe and desktop executable are in different subdirectories of the CI artifact (`target/debug` vs `target/release`); use their actual paths after extraction.

The M2 VS Code collector is included in the same scan and source filters. [VS Code coverage](M2_VSCODE_VERIFICATION.md) documents profiles, bounded metadata traversal and limits. The combined adapter's outer watchdog is now 125 seconds; the original M1-only 100-second bound described above predates this collector.

The Node collector adds resource-only runtime discovery and offline lifecycle dates. [Node coverage](M2_NODE_VERIFICATION.md) records limits and current verification gates. With this collector the owned adapter bound is 145 seconds and native acceptance bound is 200 seconds; earlier bounds in this historical M1 guide predate Node discovery.
