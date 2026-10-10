# M1 verification record

Issues: #2, #3, #4, #13; parent #1.

The **M1 inventory vertical slice passed automated real Windows 11 acceptance** at source commit `f71074bad2ad0e57e386255a8cc1ae45a8f5e654`. Native WebView scan/cancel/exact metadata rendering and a separate-process restart passed; this is not fixture-only evidence. The remaining MVP milestones are not complete.

Host: Microsoft Windows 11, build 26200 (10.0.26200), isolated task workspace, Node v24.20.0, npm 11.19.0, WinGet v1.29.380. The launcher token's Administrator role check returned false; no elevation was requested. No Rust/Cargo or MSVC C++ Build Tools/Windows SDK was found. No missing host tool has been installed. Public evidence contains only aggregate counts and stable coverage codes; no personal inventory/path/screen content.

| Gate | Evidence | Status |
|---|---|---|
| Frontend build | TypeScript + Vite production build | PASS |
| UI unit regressions | 6 tests: no fabricated output, real IPC actions, sanitized errors, escaped hostile metadata, terminal-result loading, startup snapshot loading | PASS; fixtures, not device evidence |
| SQLite SQL schema | 4 Node SQLite tests: migration/rollback, FK/check constraints, 30 immutable snapshots, cross-environment edges | PASS |
| PowerShell parser regression | 13 authored assertions including official/private/REST source aliases, missing and non-scalar fields | PASS; synthetic metadata |
| First real adapter attempt | Registry reads completed; WinGet reached 45s cap and returned WINGET_TIMEOUT | FAILED strict WinGet gate; no fabricated fallback |
| Repeat real adapter | Windows build 10.0.26200.0, PowerShell 5.1.26100.9444; Registry 167 records (55/70/21/21 by HKLM64/HKLM32/HKCU64/HKCU32), WinGet 61, observed versions 222, Publishers 164; 9.64s; no raw paths | PASS adapter only; WinGet catalog coverage PARTIAL |
| Exact product stdin/stdout transport | Initial signature cmdlet autoload failed (inherited module path). Process-only system-module restriction fixed it. `node scripts/verify-adapter-pipe.mjs`: 228 real records (Registry 167 / WinGet 61), 0 stderr bytes, bounded JSON; catalog coverage explicitly PARTIAL | PASS |
| Rust unit/migration tests | Windows CI run 38008547081: 12 locked Rust tests, 0 failures; all-target/all-feature Clippy with warnings denied | PASS |
| Windows executable build/launch | Windows CI run 38008547081 builds production Tauri, real probe and separate instrumented acceptance binary; native acceptance processes launch and exit 0 without requesting elevation | PASS Windows 11; Windows 10/installer not verified |
| Real Rust + SQLite probe | Digest-verified artifact 11652247228, commit f71074b: 220 assets, 214 versions, 156 Publishers, 38 duplicate-candidate records; 4s; exact snapshot equality after reopen. Separate real cancellation returned CANCELLED in 1s and also persisted. | PASS |
| Official source guard | Real stdin/stdout adapter after checking configured official WinGet source: Registry 167 + catalog 61, 0 stderr bytes; no source configuration changes | PASS adapter only |
| Desktop scan/cancel/restart | Commit f71074b, CI 38008547081, digest-verified artifact 11652247228: two native WebView processes exit 0; 220 real rows and 214 versions, exact rendered names/versions/Publishers, same persisted scan after restart, prior cancellation. Independent read-only local SQL verifies Registry views 55/70/22/22 COMPLETE and WinGet 61 PARTIAL. | PASS automated native WebView; human-visible/manual UI review not verified |

Verified artifact archive SHA-256: `5df90ad28944ff2d39be322361bd5c5f78a48ebaccfc55d3960364e4e28ef066`. Counts changed between earlier scans as this is actual device data, not a fixed fixture. Full SQLite snapshots and software lists are ignored local acceptance artifacts and were not uploaded.

AC mapping: AC-01 launch is verified on Windows 11; installation and Windows 10 remain NOT VERIFIED. AC-02 has conservative Registry view reconciliation, retained provenance/multiple versions and explicit PARTIAL catalog coverage; Registry/catalog correlation is not claimed as universally complete. AC-11 has 30-snapshot regression evidence plus actual process-restart persistence. The M1 subset of AC-12/15 is covered by read-only APIs, fixed commands, owned-process containment, cancellation/timeouts and poisoned metadata tests; later multi-runtime/version-range/offline regressions remain pending.

The normalized schema includes future advisory states, but M2–M5 behavior is not implemented. #13's findings/priority filters depend on M3/M4 and remain open. True installation origin can be UNKNOWN; no vulnerability results or safe verdict are claimed. See `WINDOWS_DEVELOPMENT.md` for exact commands and coverage limitations. Final PR/Issue records identify the subsequent documentation commit and main integration.
