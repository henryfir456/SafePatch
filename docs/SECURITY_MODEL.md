# SafePatch Security Model v0.1

## Trust boundaries
Treat Registry, filesystem names, manifests, VS Code extensions, subprocess stdout/stderr and network advisory payloads as untrusted. All results are data, never executable instructions. No remote code execution or installation as a scan side effect.

## Defaults
- Local-first and read-only; avoid admin rights. No automatic updates, extension installs, pip/npm installs, or scripts.
- Project scanning is opt-in, directory scoped, path canonicalized and cancellable. Respect configured exclusions and avoid scanning credential/private key files.
- External lookup is opt-in for project dependency metadata; show destination/domain and minimize sent data. Never transmit full project paths, proprietary source or package-lock files wholesale.
- Allowlist advisory endpoints and official update sources, HTTPS, bounded responses, rate limits/timeouts/backoff, defensive JSON parsing.
- Keep evidence and timestamp. Missing, stale or ambiguous data is UNKNOWN, never 'safe'. CISA KEV is not version applicability evidence.
- Separate package vulnerability, malicious extension/provenance warning, unsupported runtime and update availability.
- Local SQLite file permission restrictions; redact user/project paths in exports; do not log environment variables, tokens or secrets.
- Use command argument arrays, never shell string interpolation with filenames. Prefer parsing lockfiles/metadata instead of invoking project tools.
- Show explicit review + confirmation before any future update actions. Updates are out of v0.1 read-only scope.

## Threat test cases
Malicious package name escaping UI; CLI spoofing/path hijack; symbolic link loops; huge/corrupt lockfile; symlink crossing selected directory; network outage; OSV/NVD mismatch; vendor/product name collision; stale data; duplicate advisory IDs; regex/version edge cases; path disclosure in exports; privilege escalation requests.

## Product promises we must not make
'No CVE found' does not mean safe. Do not claim that unindexed marketplace extensions have a comprehensive CVE database; represent coverage. Never say that desktop binaries are fully analyzed by an NVD name search.
