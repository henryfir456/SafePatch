# SafePatch Windows Preview 0.2.0

Issue [#26](https://github.com/henryfir456/SafePatch/issues/26), update health; original installer Issue [#22](https://github.com/henryfir456/SafePatch/issues/22), under installer/CI [#16](https://github.com/henryfir456/SafePatch/issues/16). This read-only update-health Preview does not complete M5.

## Download and identity

Use the **SafePatch-Windows-Preview-<commit>** artifact from the successful final **main** Windows Preview workflow. GitHub Actions artifact download requires a GitHub login and the artifact has a limited retention period. PR artifacts are review builds, not the final delivery. The final Issue/PR report links the exact main run and artifact.

Extract the ZIP. It contains:
- `SafePatch-Preview-0.2.0-x64-setup.exe`: recommended current-user wizard installer.
- `SafePatch-Preview-0.2.0-x64.msi`: current-user Windows Installer package with basic progress UI.
- `safepatch.exe`: the same production application payload for identity comparison.
- `SHA256SUMS.txt`, `manifest.json`, and aggregate CI package/lifecycle reports.

Check the ZIP SHA-256 against the digest published for that exact GitHub artifact, then compare the installer hash to its entry in SHA256SUMS.txt:

```powershell
Get-FileHash .\SafePatch-Preview-0.2.0-x64-setup.exe -Algorithm SHA256
```

`manifest.json` records version 0.2.0, Preview channel, full source commit, CI run/attempt, architecture, production payload hash and installer hashes. Confirm that its source_commit is the final main commit reported in Issue #26. The build is **UNSIGNED**: no Authenticode certificate or SmartScreen reputation is claimed. Do not disable Windows security policies; if your policy blocks it, report that limitation.

## Requirements and install

Windows 10/11 x64 with Microsoft Edge WebView2 Evergreen Runtime already installed. Other architectures are outside this Preview. Rust, Cargo, Node.js, npm, VS Code and C++ Build Tools are **not required to install or run SafePatch**; absent scanned tools simply limit inventory coverage. Built application code and SQLite are included, with static MSVC CRT linkage.

The installer deliberately does not download, install or update WebView2 or any scanned program. If WebView2 is missing, stop and arrange its installation separately through the computer owner's normal process. SafePatch does not elevate or alter that prerequisite.

Run the EXE installer normally as your current user, choose its current-user directory, and finish. Launch **SafePatch Preview** from the **SafePatch Preview EXE** Start-menu folder. It does not auto-launch at setup completion.

Alternatively, open the MSI or use Windows Installer:

```powershell
msiexec /i .\SafePatch-Preview-0.2.0-x64.msi /passive /norestart
```

MSI is strictly per-user and rejects ALLUSERS overrides; its Start-menu folder is **SafePatch Preview MSI**. EXE defaults to `%LOCALAPPDATA%\Programs\SafePatchPreview-EXE`; MSI defaults to `%LOCALAPPDATA%\SafePatchPreview-MSI`. Their install directories and shortcuts are distinct. Choose one format for normal use. Both use the same per-user history location, and only one running application can own that database. Close SafePatch before reinstalling. Installation success is distinct from scanner coverage.

## Operate

The primary **更新健檢** tab shows available updates, installed/target versions, exact product IDs, official catalog/manifest evidence and query timestamps. Filter UNKNOWN / ERROR / NO_UPDATE_REPORTED separately; none means safe/latest. Vulnerabilities are **未檢查**. **完整軟體清單** is the secondary tab.

Click **開始唯讀掃描** to collect actual Registry/WinGet, VS Code/extensions/profiles and Node resource metadata. Progress, partial errors, exact observed versions and provenance remain visible. Use **取消掃描** while a scan is active. Missing WinGet or a source timeout produces PARTIAL/UNKNOWN; a catalog association does not establish the actual installation origin.

In the npm section, explicitly select one local project directory with package-lock.json or npm-shrinkwrap.json, then click **開始 npm 唯讀掃描**. No npm/Node/project script or lifecycle hook executes and no dependencies are installed. Lockfile entries are declarations, not proof of installed packages; exact versions, DIRECT/TRANSITIVE/UNKNOWN relations and outgoing edges have local search/paging.

Close and reopen SafePatch to load persisted observations. The current-user database remains at the application local-data location for identifier `io.github.henryfir456.safepatch`, separate from installer directories. No personal inventory or selected private path is submitted to an advisory service.

This Preview does not implement Python/Git inventory, CVE/advisory matching, automatic security updates, scheduled scans, reports or a safe verdict. **UNKNOWN does not mean safe.** Node support lifecycle metadata is separate from vulnerability applicability.

## Removal and data retention

Windows Settings → Apps lists **SafePatch Preview (EXE)** or **SafePatch Preview (MSI)**. Close the app before removing that chosen installation. Removing one format does not own the other format's executable or shortcuts. Both uninstallers retain local scan history and do not offer automatic app-data deletion. This guide does not authorize removal of a user's installation/data as an acceptance test.

## Verification limits

Prior main scanner evidence on the connected Windows 11 Home build 26200, non-admin, is recorded in [M1](M1_VERIFICATION.md), [VS Code](M2_VSCODE_VERIFICATION.md), [Node](M2_NODE_VERIFICATION.md) and [npm](M2_NPM_VERIFICATION.md). It is background scanner evidence, not installation acceptance for this final payload. v0.2.0 adapter evidence and current pending gates are recorded in [update health verification](UPDATE_HEALTH_VERIFICATION.md).

The new pipeline runs on a clean GitHub Windows Server runner. Package checks inspect compiled MSI privilege/registry/action tables and extract real EXE/MSI payloads to compare their SHA-256 with the production binary. Tauri bundle-type binary patching is disabled (Preview has no updater), so both formats preserve that exact production payload. Acceptance-feature binaries build in a separate Cargo target directory and are excluded from the downloadable Preview artifact.

The CI lifecycle attempts installation, payload comparison, launch/UI scan/cancel/npm/restart and own test uninstall/retention (including an unchanged SQLite-file hash after both apps close) in a restricted same-user child token, and records the actual result in ci-lifecycle.json. PASS means that reported gate executed on the hosted Server runner; it is not an independent standard-account login or Windows 10/11 user-device claim. Missing runtime/token/UIA execution remains NOT_VERIFIED and an observed functional failure fails CI.

**Connected user Windows 10/11 final-installer non-admin install, launch, scan, cancellation, reopen and manual visible UI: NOT VERIFIED while that environment is unavailable.** No user data or other projects are modified. A green regression/build or an earlier functional scanner artifact cannot replace these gates.
