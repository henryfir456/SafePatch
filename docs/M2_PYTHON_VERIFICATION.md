# M2 Python verification — Issue #8

Issue: [#8](https://github.com/henryfir456/SafePatch/issues/8). Draft: [PR #21](https://github.com/henryfir456/SafePatch/pull/21). Functional revision: c4f7484fe57b945c2f595ba1bc45a0e85589a9a7. **Issue #8 is not yet delivered or merged.**

## Implemented scope

Opt-in selected local interpreter/venv root; guarded fixed metadata reads; bounded PE32/PE32+ resource observation; pyvenv.cfg hints separated from interpreter resource versions; direct .dist-info METADATA/INSTALLER observations; exact valid PyPI versions, duplicate slots retained and UNKNOWN dependency relationships. No Python/pip/activation/.pth/package execution, install, elevation or advisory/network submission.

Paths are hashed before IPC/database storage. PE retained fields reject recognizable private paths/URLs/credentials; malformed resource strings cannot establish a known version. Metadata identity, publisher and public origin remain unverified. Base/user/extra import paths, editable metadata, legacy egg-info and linked/unsafe paths have explicit exclusions. A selected embedded interpreter without the supported site-packages layout does not invent package results.

SQLite v4 preserves prior Windows/npm parent references and immutable snapshots. Current Python package membership changes only for the observed environment after a successful PARTIAL result. Cancellation/failure saves no Python snapshot or new membership. The three collectors share one active scan slot with separate latest scopes.

## Verification gates

- Local Windows production renderer build and 16 synthetic IPC/UI tests: PASS at completed UI revision 0a5342b14d14365fdefb1138c2c8a98c76d7e8fe. The earlier dd05ec5 test-only TS2307 was expected before PythonInventory existed, not a final delivery claim.
- Local Node SQLite schema regressions: 7 PASS at 2e430bc45357b2cdd8ac61c4462eb609a6dbdd07.
- Windows hosted reader/PE authored-fixture regression: 57 Rust tests passed at 3cca811d8db0c2f48bfeadbafc28b6f5799046e9; that workflow was not release-ready because Clippy failed. Later aa9c70067c60f6f801569dc95822322cae141084 passed complete Windows workflow [38026699097](https://github.com/henryfir456/SafePatch/actions/runs/38026699097), including PE privacy/NUL fixes and owned helper code, before the final renderer/probe was added.
- Current contracts cover malformed/oversized headers, exact PEP440 package declarations, current Core Metadata 2.6, unsupported formats, retained-field poisoning, exact-block unterminated UTF-16 strings, cancellation, selected-root hardlinks, real owned Scripts/Lib junction fixtures, cache validation, migration FK restoration/rollback, current membership and immutable history. Authored data is not installed-device evidence.
- Review-driven runtime-shape regression was observed failing at 5d82c495b4fd07cc0840c408f8d741a47e612b65 (venv version 1); the shared three-part/no epoch/local runtime predicate was corrected in 895c10d46eb0b874b0aae70223ac52db45d79f15.
- A deterministic acknowledged-cancel race test was observed failing at 3e055b329f48f761ccaab56af243229ca041303a and again at 895c10d (PARTIAL instead of CANCELLED). c4f7484 rechecks active.cancel under the completion lock; [Windows CI 38027784619](https://github.com/henryfir456/SafePatch/actions/runs/38027784619) has passed the Rust stage, with the complete workflow still pending at this checkpoint.

## Actual-host status and blocker

The connected acceptance host used for prior delivered inventory is Windows 11 Home build 26200, current user/non-admin. Read-only Python root discovery found existing interpreters/site-packages without executing a discovered binary. Candidate paths and full metadata remain in ignored local files; this is preflight only.

**Actual installed Python probe/helper/SQLite persistence and native WebView scan/cancel/paging/restart: NOT VERIFIED.** No current #8 CI executable has yet been digest-verified and executed on that device. The local exec-server transport disconnected before the cancellation-fix command could run; the bounded read-only status retry did not complete and was canceled. The small fix and documentation were committed via the already-authorized GitHub draft branch, preserving main and avoiding host changes. Local checkout must be fast-forward synchronized after connectivity returns; do not reset user files.

Real existing venv coverage, Windows 10, human-visible/manual UI and installer acceptance also remain NOT VERIFIED. Do not create/install a new venv or run a discovered interpreter merely to manufacture device evidence. No additional host tools were installed.

## Remaining acceptance work

1. Require complete current-head Windows CI and independent review.
2. Restore the Windows command channel; fast-forward the isolated Python checkout without overwriting any local edits.
3. Verify this repository's artifact SHA-256, then run python_probe and feature-only native WebView acceptance against explicitly selected existing environment roots; independently inspect exact SQLite observations/current membership and compare read-only input fingerprints.
4. Record aggregate real-device results and limitations only. Keep full inventory/private paths local.
5. Update PR/Issue with the final commit and AC mapping; mark ready and merge main only after required gates actually pass. Git #9 and later milestones remain pending.
