# Authorized AirMirroring REA Investigation SOP

Reference: [Microsoft Store app `9P2K9G20SWZ6`](https://apps.microsoft.com/detail/9P2K9G20SWZ6?hl=en&gl=IT&ocid=pdpshare). Store description indicates **iPhone/iPad/Mac → Windows PC**. The internal implementation **has not been analyzed in this blueprint**. Goal: enumerate visible, permissioned features and determine protocol *from evidence*, not assumptions.

## Gate 0 — legal, provenance, and environment

- Use a lawfully obtained installed copy on a device you own and your own sending devices/network. Review applicable product EULA, local law and security research policy. Stop if extraction or reverse engineering is not permitted in your circumstances; obtain advice where necessary.
- Do **not** bypass DRM, license checks, authentication, code signing, store encryption or access controls. Do not redistribute proprietary executables, screenshots showing personal data, credentials or packet captures. If protected bundles cannot be inspected lawfully, do black-box UI/network behavior only.
- Isolated test LAN / Windows VM or sacrificial machine, local packet capture with consent and defined endpoints; record OS, version, package full name, SHA-256, date and device specs.
- Never modify the installed signed package. Native binaries are read-only evidence.
- REA requirements depend on target/provider/host. Check `https://github.com/morluto/rea` and `rea --help` before calling commands; follow tool-discovered schemas, not imaginary commands.

## Gate 1 — prepare REA in Codex

```bash
# Compatible Node.js: see REA official README for currently supported ranges.
npx skills add https://github.com/morluto/rea --skill reverse-engineer-anything
npx rea-agents@latest setup
# Select Codex in interactive agent setup; inspect config changes and approve them.
# Restart Codex to load the MCP server.
npx rea-agents@latest --help
```

Skill installation **does not** register the MCP server. Native decompilation may require supported Ghidra/Hopper/IDA tooling; REA's Windows-native coverage can depend on provider/build. If a tool is unavailable, use standard OS metadata and supported static analyzers as independent manual steps, label which produced evidence.

## Gate 2 — metadata & initial behavior inventory

On Windows PowerShell, discover installed package without assuming its package name:

```powershell
Get-AppxPackage | Where-Object { $_.Name -match 'AirMirroring|Mirror|Cast' } |
    Select-Object Name, PackageFullName, PackageFamilyName, InstallLocation, Version
```

Record the *actual* package info. If approved and accessible, inventory **read-only** manifest, DLL/EXE names, file sizes, hashes, PE/CLI metadata, strings indicating frameworks and protocol libraries. WindowsApps default package locations are OS-protected; do not take ownership, relax ACLs, or alter signed files solely to extract them. Alternative: use a legitimately provided unpackaged build or stick to black box.

Create `docs/research/airmirroring-findings.md` as a table with `feature_id | proposed feature | observed UI | protocol hypothesis | evidence_id | confidence | unresolved experiment`.

Behavioral test cases:
1. Launch app disconnected, screenshot UI labels/tabs/dialogs (redact device names).
2. Note any incoming network firewall prompts, screen capture approvals, permissions, login and paywall.
3. On iOS/iPadOS test Control Center Screen Mirroring detection, connection, orientation change, lock/unlock, audio route, stop/cancel. On Mac test AirPlay screen sharing. All endpoints owned/authorized.
4. Disconnect Wi-Fi, disable BLE, swap APs; determine if same subnet required or Wi-Fi Direct is involved, and whether Bluetooth contributes at all.
5. Attempt source switching, multiple sessions, screenshot/record/annotate features **only if UI provides them**; note what is absent.
6. Restart receiver, pause media, turn VPN on/off, verify reconnect behavior; record failure messages.

## Gate 3 — passive protocol evidence

Use Wireshark/tshark, OS Process Monitor and netstat/Get-NetTCPConnection where authorized. Begin capture immediately **before** launching the app and end after explicit disconnect. Restrict to lab hosts and redact packet payloads, device IDs and IPs before sharing.

**Test hypotheses rather than assert**:
- AirPlay family: DNS-SD records `_airplay._tcp` / `_raop._tcp`; possible RTSP/HTTP-like control and TCP/UDP media. Detect if actually advertised.
- `_googlecast._tcp` (Google Cast), `_miracast._tcp` or Wi-Fi Direct indicators, app-specific mDNS names.
- UDP discovery, SSDP, local TCP listeners, TLS handshakes, ICE/STUN, DTLS, SRTP, audio codecs.
- Check port/process correlation via `Get-NetTCPConnection -State Listen` joined to PID and Windows process metadata. No brute-force port scanning or unsolicited third-party endpoints.
- Note packet timestamps for discovery→connect→first frame; do not decrypt third-party encrypted streams.

Evidence minimum:
`E-001` screenshot UI; `E-002` package/manifest hash; `E-003` DNS-SD capture summary; `E-004` owned-host socket list; `E-005` connect/disconnect timed trace; `E-006` permissions/dialogs; `E-007` optional static binary references. Every finding uses `OBSERVED`, `INFERRED`, or `UNKNOWN` and a confidence/reproducibility field.

## Gate 4 — focused REA static analysis

Only after Gate 3 has a target question, ask Codex/REA to search package layout, imported modules, managed/native boundaries, interesting protocol strings and call graph. Ask for **function name + offset/hash + evidence** rather than copied decompiled source. Do not paste proprietary code into the product repo. Report limitations if stripped, packed, signed, encrypted or outside REA support.

**Sample prompt**:
> On the authorized AirMirroring test installation, determine whether the app advertises an AirPlay receiver using DNS-SD and where the listener is started. Give evidence IDs, file hashes, symbols/call chains where legally reviewable, confidence and competing explanations. Do not copy executable logic or remove protection.

## Gate 5 — clean-room handoff

A researcher delivers **behavioral contracts only**, e.g. “receiver announces capability over mDNS; pairing is initiated by sender; screen rotates, audio optional.” Separate product engineers implement their **own** protocol from public specifications and native APIs, without consuming proprietary snippets. Evaluate license/trademark/patent obligations before adding compatibility adapters.

## Stop criteria / report

- Produce a feature inventory and protocol hypothesis matrix with evidence + unresolved items.
- Track permissions, failure flows, requirements and test reproducibility.
- **No claim** that connection is Bluetooth or ordinary LAN sockets until observed.
- If AirPlay ecosystem cannot be implemented compliantly, mark it an optional interoperability research project, not MVP.
