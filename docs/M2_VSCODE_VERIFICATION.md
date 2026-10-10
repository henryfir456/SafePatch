# M2 VS Code inventory (#5)

The production scan combines Registry, signed WinGet and read-only VS Code metadata. Stable and Insiders default user/system install roots, both extension stores, default extension indexes and up to 100 profile indexes are inspected. Both legacy application layouts and one-level versioned application directories are supported. UI source filters include VS Code and Extensions; profile references persist with each immutable snapshot.

Extension identity is publisher.name with the exact manifest version. A profile claim requires matching ID, version **and local location hash** from the profile index. Separate versions/locations/profiles remain separate. Unknown or conflicting metadata remains UNKNOWN. Manifest publisher is claimed metadata, not verified vendor provenance; installation origin remains UNKNOWN. Profile folder references use opaque IDs/hashes, not friendly names or private paths.

No VS Code executable, extension entry point, package script or project dependency is executed. Only bounded JSON metadata is read; UNC/network roots, alternate streams and reparse points are rejected before traversal. Enumeration/read failures produce stable partial coverage and preserve other collectors. Reads are limited to 1 MiB per file, 5,000 extension records, 100 profiles, 20 installation children and 15 seconds. The whole owned adapter has a 125-second watchdog. The native acceptance executable has a 170-second watchdog.

## Verification gates

- Local Windows 11 Home / build 26200, current user without elevation: UI 7 tests, SQLite 4 tests, Windows adapter 13 assertions, VS Code metadata 29 assertions, TypeScript production build and inherited module trap passed.
- Failure-first regressions covered poisoned/non-scalar metadata, profile directory failure, extension directory failure, same ID/version at another location, malformed obsolete values, corrupt/oversized JSON, URI profile location and versioned application layout.
- Actual fixed PowerShell stdin/stdout transport found 1 VS Code application metadata record and 10 extension records, all 10 with indexed profile association; Registry views and WinGet remained available. Mixed separators are canonicalized identically before location hashing; profile version strings use ordinal comparison. This is adapter evidence; Rust/native-WebView acceptance for this change is pending the digest-verified CI artifact.
- Local Rust/Cargo/MSVC/SDK are absent and were not installed. Hosted Windows build, Rust tests/Clippy and native acceptance remain pending. Fixtures do not establish device acceptance.

## Coverage limits

Custom/portable installations and custom extension-store roots are not enumerated. Profile inheritance, enabled/disabled state, selected active application version and extension execution state are not established. Missing indexes or unavailable referenced locations yield UNKNOWN profile associations while manifests remain inventoried. Metadata is not an authenticity proof. Reparse rejection is conservative; concurrent filesystem replacement remains a limitation of path-based metadata access. Windows 10 and human-visible/manual UI validation are NOT VERIFIED. No vulnerability match or safe verdict is implemented here.

Profile schema reference: [Microsoft VS Code profile scanner](https://github.com/microsoft/vscode/blob/main/src/vs/platform/extensionManagement/common/extensionsProfileScannerService.ts). No device inventory, paths or manifest contents are published.
