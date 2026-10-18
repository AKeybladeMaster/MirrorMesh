# skills.sh + Codex skill routing (curated and verified 2026-10-18)

`skills.sh` is a **marketplace**, not an authority. The full catalog is open-ended. Below are discovered skills that map to this architecture; installation is optional per task. **Review the upstream `SKILL.md`, scripts, network/secret access, license and audit findings before enabling. Pin reviewed versions/commits where possible**. A green badge is not a security guarantee.

| Phase | Skill | Verified listing | Purpose |
|---|---|---|---|
| Intake | `find-skills` | https://skills.sh/vercel-labs/skills/find-skills | Find later specialist skills |
| Scoping | `brainstorming` | https://skills.sh/obra/superpowers/brainstorming | Form testable requirements and constraints |
| RE investigation | `reverse-engineer-anything` | https://skills.sh/morluto/rea/reverse-engineer-anything | Evidence-based binary/runtime examination |
| Domain modeling | `domain-modeling` | https://skills.sh/mattpocock/skills/domain-modeling | Device/session/source vocabulary |
| Interfaces | `codebase-design` | https://skills.sh/mattpocock/skills/codebase-design | Deep modules and clean seams |
| Architecture revision | `improve-codebase-architecture` | https://skills.sh/mattpocock/skills/improve-codebase-architecture | Post-MVP hotspots/ADRs; not day-0 abstraction inflation |
| Plan decomposition | `writing-plans` | https://skills.sh/obra/superpowers/writing-plans | Narrow tests and PR-sized Codex tasks |
| Execution | `executing-plans` | https://skills.sh/obra/superpowers/executing-plans | Sequential task execution and checkpoints |
| Rust changes | `rust-best-practices` | https://skills.sh/pedronauck/skills/rust-best-practices | Ownership, async, perf, FFI, API standards |
| GUI design | `frontend-design` | https://skills.sh/anthropics/skills/frontend-design | UI vocabulary and visual identity; adapt to Flutter, don't copy web-only code |
| Tests | `test-driven-development` | https://skills.sh/openai/plugins/test-driven-development | Domain/protocol test-first process |
| Bugs | `systematic-debugging` | https://skills.sh/obra/superpowers/systematic-debugging | Trace root cause rather than patch symptoms |
| Review | `requesting-code-review` | https://skills.sh/obra/superpowers/requesting-code-review | Independent task/PR review |
| Security | `security-review` | https://skills.sh/getsentry/skills/security-review | High-confidence security findings; listing has audit warnings, review carefully |
| Completion | `verification-before-completion` | https://skills.sh/obra/superpowers/verification-before-completion | Commands + evidence before DONE |

**Avoid auto-installing entire packs** or every search result. Some trending mobile-design skills require SaaS keys and have failed security audits; this project can accomplish mobile UX in Flutter without such dependency.

## Minimal install commands (inspect before execution)

```bash
npx skills add https://github.com/vercel-labs/skills --skill find-skills
npx skills add https://github.com/obra/superpowers --skill brainstorming
npx skills add https://github.com/morluto/rea --skill reverse-engineer-anything
npx skills add https://github.com/mattpocock/skills --skill domain-modeling
npx skills add https://github.com/mattpocock/skills --skill codebase-design
npx skills add https://github.com/obra/superpowers --skill writing-plans
npx skills add https://github.com/pedronauck/skills --skill rust-best-practices
npx skills add https://github.com/anthropics/skills --skill frontend-design
npx skills add https://github.com/openai/plugins --skill test-driven-development
npx skills add https://github.com/obra/superpowers --skill systematic-debugging
npx skills add https://github.com/obra/superpowers --skill requesting-code-review
npx skills add https://github.com/obra/superpowers --skill verification-before-completion
```

The REA **MCP** integration is **separate**:

```bash
npx rea-agents@latest setup
# Approve registration in Codex and restart your Codex session.
```

For additional discovery use `npx skills find rust`, `npx skills find webrtc`, `npx skills find flutter`, etc. Verify any resulting listing; these are search examples, not claims the skill exists. Consult https://skills.sh/docs/cli and https://github.com/morluto/rea.

## Workflow routing to tasks

- **Task 00:** `reverse-engineer-anything`, `brainstorming` → evidence table; authorization checkpoint.
- **Task 01:** `domain-modeling`, `codebase-design`, `rust-best-practices`, TDD → stable Rust types.
- **Task 02:** `rust-best-practices`, `systematic-debugging` → measured interoperability spike.
- **Task 03:** TDD, `security-review`, code review → pairing, auth, rogue peer tests.
- **Task 04:** `frontend-design`, TDD → Flutter UX and compositing.
- **Task 05:** platform-specific official documentation FIRST, Rust practices second; do not assume marketplace skill is accurate.
- **Task 06:** verification, code review, dependency/licensing checks and signed release.
