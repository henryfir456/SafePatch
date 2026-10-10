# M1 verification record

Issues: #2, #3, #4, #13; parent #1.

M1 is **NOT ACCEPTED** until real Rust/Tauri Windows end-to-end inventory and UI checks complete. No milestone/issue is closed solely by fixture tests.

Host: Windows task workspace, Node v24.20.0, npm 11.19.0, WinGet v1.29.380. No Rust/Cargo or MSVC C++ Build Tools/Windows SDK was found. No missing host tool has been installed. Public evidence contains only aggregate counts and stable coverage codes; no personal inventory/path/screen content.

| Gate | Evidence | Status |
|---|---|---|
| Frontend build | TypeScript + Vite production build | PASS |
| UI unit regressions | 6 tests: no fabricated output, real IPC actions, sanitized errors, escaped hostile metadata, terminal-result loading, startup snapshot loading | PASS; fixtures, not device evidence |
| SQLite SQL schema | 4 Node SQLite tests: migration/rollback, FK/check constraints, 30 immutable snapshots, cross-environment edges | PASS |
| PowerShell parser regression | 13 authored assertions including official/private/REST source aliases, missing and non-scalar fields | PASS; synthetic metadata |
| First real adapter attempt | Registry reads completed; WinGet reached 45s cap and returned WINGET_TIMEOUT | FAILED strict WinGet gate; no fabricated fallback |
| Repeat real adapter | Windows build 10.0.26200.0, PowerShell 5.1.26100.9444; Registry 167 records (55/70/21/21 by HKLM64/HKLM32/HKCU64/HKCU32), WinGet 61, observed versions 222, Publishers 164; 9.64s; no raw paths | PASS adapter only; WinGet catalog coverage PARTIAL |
| Exact product stdin/stdout transport | Initial signature cmdlet autoload failed (inherited module path). Process-only system-module restriction fixed it. `node scripts/verify-adapter-pipe.mjs`: 228 real records (Registry 167 / WinGet 61), 0 stderr bytes, bounded JSON; catalog coverage explicitly PARTIAL | PASS adapter transport only; Rust/Desktop pending |
| Rust unit/migration tests | Windows CI run 38007024938; locked tests and all-feature Clippy | PASS |
| Windows executable build/launch | Windows CI run 38007024938 builds production Tauri and instrumented acceptance binaries; no host compiler installed | BUILD PASS; latest desktop launch pending |
| Real Rust + SQLite probe | Digest-verified artifact 11651194476, commit 965dd41: 219 assets, 213 versions, 155 Publishers, 38 duplicate-candidate records; 9s; exact snapshot equality after reopen. Separate real cancellation returned CANCELLED in 1s and also persisted. | PASS core binary; latest guarded-source desktop build pending |
| Official source guard | Real stdin/stdout adapter after checking configured official WinGet source: Registry 167 + catalog 61, 0 stderr bytes; no source configuration changes | PASS adapter only |
| Desktop scan/cancel/restart | Pending built binary and UI operation | NOT VERIFIED |

AC-01, AC-02 and full AC-12/15 remain pending. The normalized schema includes future advisory states, but M2–M5 behavior is not implemented. #13's findings/priority filters depend on M3/M4 and remain open. See `WINDOWS_DEVELOPMENT.md` for exact commands and coverage limitations.
