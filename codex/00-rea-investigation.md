# TASK 00 — REA evidence + constraints inventory

**Skills:** reverse-engineer-anything, brainstorming, domain-modeling
**Inputs:** `docs/research/airmirroring-rea.md`, `docs/research/airmirroring-evidence-template.md`, Microsoft Store listing, user-owned authorized installed app. **No product implementation in this task.**

## Codex steps

1. Ask owner to confirm authority to analyze installed package, approve passive traffic capture and specify test Windows host with paired iPhone/iPad/Mac. If not approved, do published-material analysis only.
2. Verify Node.js prerequisites and REA current setup on local host, use official `npx rea-agents@latest setup`, select Codex and restart. Confirm available MCP tools; no native decompilation tool invented.
3. Inventory screenshot/UI and exact package identity; record manifest/file hashes if access lawfully available. Never bypass Store ACL/security.
4. Capture before app launch with authorized passive sniffer and compare mDNS/DNS-SD, open sockets, PID, firewall prompts, initial/successful/disconnected state. Explicitly check `_airplay._tcp`, `_raop._tcp`, `_googlecast._tcp`, ICE/STUN and Wi-Fi Direct indicators.
5. Inspect protocol only with approved REA analyzer and static read-only methods, matching passive observations to code evidence where possible. No decompiled source copied to product repo.
6. Enumerate all visible user actions and settings. Per action test failure/offline modes, rotate, audio, screenshot, user permissions and multi-device behavior. Distinguish absent feature from untested.
7. Produce `docs/research/airmirroring-findings.md`: one row per capability `ID, expected test, observation, evidence, confidence, counterexample, clean-room requirement`. Redact payloads.
8. Update protocol assumptions in SPEC as hypothesis/validated; add `docs/adr/0003-reference-interoperability.md` if evidence justifies optional AirPlay feature.

## Acceptance

- 100% of claimed features/protocols supported by evidence ID and reproducible test.
- Unknown explicitly stays unknown. Conflicting observations logged.
- No copyrighted assets, binaries, decrypted streams, private captures or credentials in GitHub.
- Provide exact test environment and commands used; report `BLOCKED` if legal/OS analysis prevents research.

**Stop and request user acceptance of background/protocol findings before task 01.**
