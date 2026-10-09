# SafePatch Agent Instructions

Read README.md and docs/{ARCHITECTURE,MVP_ACCEPTANCE,SECURITY_MODEL,DELIVERY_PLAN}.md before changes.

1. Always link implementation work to an existing GitHub issue or create one first. Report issue number and changed files.
2. Prioritize the M1 real Windows inventory vertical slice before breadth or polish. Never fake sample scans as actual device output.
3. Default to read-only/no admin/no arbitrary script execution/no auto-update. Treat all discovered and remote data as untrusted; no secrets in reports/logs.
4. Explicitly separate UPDATE_AVAILABLE, CONFIRMED_APPLICABLE_VULNERABILITY, POTENTIAL, UNKNOWN and NOT_AFFECTED. Keep provenance and evidence. Unknown is not safe.
5. Include tests for each behavior, test negative/false-positive cases and document actual verification host. Do not claim Windows validation without Windows execution evidence.
6. Keep main continuously usable. For small verified changes, commit to main if permitted; use PRs for nontrivial/high-risk changes. Never auto-merge unverified changes; do not strand finished work on an unmerged branch.
7. Completion report: issues addressed, commit/PR, tests run, Windows verification evidence, remaining blockers and next task.
8. No unnecessary clarification for already specified stack/scope. Make progress using documented defaults.
