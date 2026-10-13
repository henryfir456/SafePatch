# v0.2.0 update health design and implementation plan — Issue #26

Goal: one read-only Windows scan reports confirmed available versions with exact WinGet product identity, source and observation time. Vulnerability assessment remains NOT_CHECKED.

Existing architecture: product-owned PowerShell adapter -> bounded Rust collector -> normalized assets -> immutable SQLite scan_assets JSON snapshots -> typed IPC -> React. Registry identities and separately installed versions remain distinct. No schema version change is needed: optional update evidence extends the existing immutable JSON snapshot; legacy snapshots deserialize to UNKNOWN / NOT_CHECKED.

Approach: use signed registered Microsoft App Installer, confirm the exact official winget source, then read WinGet list --source winget --disable-interactivity. Microsoft ReportListResult emits installed ID/version and available upgrade version in the list, using the same version selection/pinning engine as upgrade listings. This is safer than any upgrade command. Parse rows by verified exported product IDs and invariant trailing values, never translated headers or display-name matching. UTF-8 and BOM UTF-16 are decoded strictly. Unsupported/truncated/ambiguous rows fail closed. Explicit row with no available version means NO_UPDATE_REPORTED; an absent row is UNKNOWN. Numeric dotted versions must increase for confirmation; unsupported version grammars are UNKNOWN. Query faults preserve Registry and exported inventory.

Source evidence: https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLICore/Workflows/WorkflowBase.cpp (ReportListResult). Only official HTTPS catalog and derived winget-pkgs product manifests URLs enter update evidence. Local metadata may be cached; no freshness guarantee or latest/safe claim.

- [ ] Add fixed bounded/cancellable WinGet list runner and independent parser, UTF decoding, failure codes and synthetic PowerShell tests for locale/format/multiple installations/duplicate rows/unavailable/failure/timeout.
- [ ] Add typed update evidence and NOT_CHECKED vulnerability field, default legacy uncertainty, numeric conservative comparison, reject spoofed sources and exact-version mismatch; Rust negative and snapshot restart tests.
- [ ] Add update-first homepage, four summary cards, inventory secondary tab, status/source/search filters and evidence/recommendation columns. React tests compare counts and displayed backend evidence and reject unsafe URL rendering.
- [ ] Add regression steps to Windows CI and adapt native acceptance to both tabs. Update version 0.2.0, Traditional Chinese changelog and verification documentation.
- [ ] Run available Windows-host tests; push issue-linked PR. Independent review and Windows Actions must pass before merge; package exact verified main EXE/MSI with hashes. Never merge paused Python PR #21. Record any missing device/install verification as NOT_VERIFIED.

Review focus: same name different product; mismatched versions between sequential queries; missing or truncated table rows; pinned/multiple-version limitations of WinGet; legacy history must not become safe/current.
