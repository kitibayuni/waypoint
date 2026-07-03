# Build Plan — Step-by-Step

Companion to `DESIGN.md`. Each phase is meant to leave the app in a runnable
state (`docker compose up` works) before moving to the next. Phases map to
`DESIGN.md` §11 but are broken into concrete, checkable steps.

---

## Phase 0 — Repo & tooling bootstrap
- [ ] `git init`, `.gitignore` (target/, node_modules/, .env, certs/).
- [ ] `.env.example` with `POSTGRES_*`, `SESSION_SECRET`, `CRED_ENC_KEY`.
- [ ] Top-level `docker-compose.yml` with 3 services stubbed: `db`, `backend`, `nginx` (no logic yet, just images that boot).
- [ ] `cargo new backend`, add `axum`, `tokio`, `sqlx` (postgres, runtime-tokio-rustls), `serde`, `tower`, `tower-http` to `Cargo.toml`.
- [ ] `npm create svelte@latest frontend` → SvelteKit, TypeScript, `adapter-static`.
- **Exit check:** `docker compose up` boots Postgres; `cargo run` in backend serves a `/healthz` 200; `npm run dev` in frontend shows the default page.

## Phase 1 — Compose wiring + TLS + reverse proxy
- [ ] `nginx/nginx.conf`: HTTPS listener, self-signed dev cert in `nginx/certs/`, `/api/*` → `backend:PORT`, `/` → static SPA build output.
- [ ] Wire `backend` Dockerfile (multi-stage: `cargo build --release` → slim runtime image).
- [ ] Wire `frontend` Dockerfile (multi-stage: `npm run build` → copy static output into an nginx-servable volume or bake into the nginx image).
- [ ] `docker-compose.yml`: env vars from `.env`, `depends_on` + healthchecks (`pg_isready`, backend `/healthz`), named volume for Postgres data.
- **Exit check:** `docker compose up` end-to-end serves the SvelteKit placeholder over `https://localhost` through Nginx, proxying a test `/api/healthz` to Axum.

## Phase 2 — Database schema & migrations
- [ ] `backend/migrations/0001_init.sql` (via `sqlx migrate add`): `users`, `clients`, `engagements`, `engagement_members`, `scope_items`.
- [ ] `0002_assets.sql`: `hosts`, `host_addresses`, `services`, `tags`, `host_tags`.
- [ ] `0003_credentials.sql`: `credentials`, `credential_usage`.
- [ ] `0004_observations.sql`: `observation_types`, `observations`, `attack_path_rules`, `trust_relationships`.
- [ ] `0005_findings_notes.sql`: `findings`, `finding_hosts`, `notes`, `attachments`, `checklists`, `checklist_items`.
- [ ] `0006_templates.sql`: `templates`, `template_payloads`.
- [ ] `0007_cross_cutting.sql`: `audit_log`, generated `tsvector` columns + GIN indexes on searchable tables.
- [ ] Run `sqlx migrate run` against the compose `db`; commit `sqlx-data.json` / enable offline mode for CI builds without a live DB.
- **Exit check:** fresh `docker compose up` auto-runs migrations (entrypoint script or backend startup hook) and produces the full schema.

## Phase 3 — Auth & RBAC skeleton
- [ ] `backend/src/auth/`: Argon2 password hashing, session table or signed cookie, `POST /auth/login`, `POST /auth/logout`, `GET /auth/me`.
- [ ] Tower middleware: require session for all `/api/*` except `/auth/login` and `/healthz`.
- [ ] Seed one admin user via a startup migration or CLI (`cargo run --bin create-admin`).
- [ ] Frontend: `routes/login/+page.svelte`, `lib/stores/auth.ts`, `+layout.svelte` redirect-to-login guard.
- **Exit check:** can log in via the SPA, land on an empty authenticated shell, log out.

## Phase 4 — Engagements, clients, scope, members
- [ ] Backend: `routes/clients.rs`, `routes/engagements.rs` (CRUD), `routes/scope.rs`, `routes/members.rs` (add/remove/role-change, enforce RBAC per engagement).
- [ ] Frontend: `routes/engagements/+page.svelte` (list/create), `routes/engagements/[id]/+page.svelte` (overview: client info, timeline, global notes, members, scope table).
- [ ] `lib/api/engagements.ts` typed client.
- **Exit check:** create a client, create an engagement under it, add scope items, add a second team member with a role, edit global notes.

## Phase 5 — Hosts, addresses, services, tags
- [ ] Backend: `routes/hosts.rs` CRUD, `routes/services.rs` nested under host, tag endpoints.
- [ ] Frontend: host list view per engagement, `HostCard` component, `routes/engagements/[id]/hosts/[hostId]/+page.svelte` with tabs (General / Services / — other tabs stubbed until later phases).
- **Exit check:** add a host with multiple IPs, hostname, OS; add services (port/proto/product/version); tag it; see it in the host list.

## Phase 6 — Structured observations + catalog seed
- [ ] Backend: `routes/observation_types.rs` (catalog CRUD), `routes/observations.rs` (attach to host/service, status transitions).
- [ ] `backend/seeds/observation_types.sql`: seed rows from the notes table in `DESIGN.md` §12 (`smb_signing_disabled`, `llmnr_enabled`, `ldap_anonymous_bind`, `default_credentials`, `weak_tls_config`, `kerberoastable_spn`, etc.).
- [ ] Frontend: `ObservationChip` component, observation picker (search catalog, attach with evidence note), host detail "Observations" tab.
- **Exit check:** attach `smb_signing_disabled` to a host from the seeded catalog, confirm it, see status change.

## Phase 7 — Credentials + usage tracking
- [ ] Backend: `routes/credentials.rs` (engagement-scoped), `routes/credential_usage.rs`; AEAD encryption for `secret` column (key from env), redact in list responses.
- [ ] Frontend: credentials table (engagement-wide), "test against host/service" action that writes `credential_usage`, visual flag for untested reuse candidates.
- **Exit check:** add a captured credential, mark it used successfully against a second host, see the reuse relationship reflected.

## Phase 8 — Attack graph engine + visualization
- [ ] `backend/seeds/attack_path_rules.sql`: seed the example rules (`smb_signing_disabled → NTLM Relay → Lateral Movement`, `llmnr_enabled → Responder → Hash Capture`, kerberoasting rule, etc.).
- [ ] `backend/src/graph/`: builder function assembling nodes (hosts, credentials, key observations) + edges (trust_relationships, credential_usage, rule matches) into a JSON graph; `GET /engagements/:id/graph`.
- [ ] `routes/trust_relationships.rs` CRUD.
- [ ] Frontend: install `cytoscape` (+ `cytoscape-svelte` wrapper or thin custom binding), `routes/engagements/[id]/graph/+page.svelte`, color-coded node/edge types, a "suggested next steps" side panel driven by unmatched rule outcomes.
- **Exit check:** with the host from Phase 6 (SMB signing disabled) and no matching outcome yet, the graph shows the host node plus a suggested "NTLM Relay → Lateral Movement" path.

## Phase 9 — Templates (all 5 kinds) + instantiation
- [ ] Backend: `routes/templates.rs` CRUD + `POST /templates/:id/instantiate` (transactional: copy payload → create host/checklist/finding/note/engagement + children).
- [ ] Seed a starter template set from `DESIGN.md` §12 (e.g. an "AD recon" host template with a checklist synthesized from `active-directory/initial-recon.txt`, a "login procedure" note template from `myweaknesses.txt`).
- [ ] Frontend: `routes/engagements/[id]/templates/+page.svelte` (browse/create templates), "New host from template" flow, checklist rendering (`ChecklistPanel` with todo/doing/done/na states).
- **Exit check:** create a host from the "AD recon" template and see its checklist and note skeleton auto-populated.

## Phase 10 — Findings, notes, attachments, search
- [ ] Backend: `routes/findings.rs` (fields per `on-reporting.txt`: name/CVE/CVSS/description/references/remediation/PoC/affected systems), `routes/notes.rs` (polymorphic subject), `routes/attachments.rs` (multipart upload, sha256, storage on a mounted volume).
- [ ] `backend/src/search/`: FTS query across notes/findings/observations/hosts/credentials/attachments; `GET /search?q=&types=`.
- [ ] Frontend: finding editor (Markdown fields), note editor with `markdown-it` render, attachment upload/gallery on host/finding, global search bar + results page.
- **Exit check:** attach a screenshot to a host, write a finding referencing it, search a keyword and get results across notes + findings + observations.

## Phase 11 — Dashboard
- [ ] Backend: `GET /engagements/:id/dashboard` aggregating host-status counts, checklist completion %, findings-by-severity, credential stats, timeline vs scope window.
- [ ] Frontend: dashboard tiles (use the `dataviz` skill for chart/stat-tile styling) + attack-graph mini-view + top suggested next steps.
- **Exit check:** dashboard reflects live counts as hosts/observations/findings change.

## Phase 12 — Report generation
- [ ] `backend/src/report/`: engagement → HTML template (Executive Summary, Overview/Methodology, Scope & Duration, Findings grouped by severity) → PDF (headless Chromium or a Rust PDF crate, e.g. `wkhtmltopdf`/`headless_chrome`).
- [ ] `GET /reports/:engagement_id` (HTML preview) + `?format=pdf`.
- [ ] Frontend: "Generate Report" button on engagement page, preview + download.
- **Exit check:** generate a PDF for a populated engagement; sections match `notes/on-reporting.txt` structure.

## Phase 13 — Importers
- [ ] `backend/src/import/nmap.rs`: parse Nmap XML → hosts/services.
- [ ] `backend/src/import/nessus.rs`: parse `.nessus` → hosts/findings/observations.
- [ ] `backend/src/import/bloodhound.rs`: parse BloodHound JSON → hosts + `trust_relationships`.
- [ ] `POST /import/{nmap,nessus,bloodhound}` (multipart upload), dedupe against existing hosts by IP/hostname.
- [ ] Frontend: import page per engagement, upload + preview-before-commit.
- **Exit check:** import a sample Nmap XML and see hosts/services created without duplicating an existing host.

## Phase 14 — MITRE / CVSS / audit / collaboration polish
- [ ] `mitre_techniques` reference table + link from `observation_types`/`attack_path_rules`/`findings`; display technique IDs in graph and findings.
- [ ] Client-side CVSS vector calculator component feeding `findings.cvss_vector`/`cvss_score`.
- [ ] Audit middleware: write to `audit_log` on create/update/delete for key resources; simple audit viewer page (admin-only).
- [ ] Row-versioning (e.g. `notes`/`findings` history table + diff view) for version history.
- **Exit check:** an edit to a finding shows up in an audit trail; a technique ID is visible on a graph node.

## Phase 15 — Hardening & deployment
- [ ] Replace dev self-signed cert story with real cert mounting instructions (Let's Encrypt/reverse-proxy note) in README.
- [ ] Rate-limit `/auth/login`; CSRF protection for state-changing requests if cookies aren't `SameSite=Strict`-safe.
- [ ] Backup/restore doc for the Postgres volume; `.env` secrets rotation notes.
- [ ] `docker compose -f docker-compose.prod.yml` variant (no dev bind-mounts, resource limits, restart policies).
- **Exit check:** fresh clone + `.env` + `docker compose up -d` gives a working, authenticated, HTTPS instance with seeded catalogs/templates ready to use.

## Phase 16 — Timeline & Replay view
Depends on Phase 8 (graph builder) for the map side and Phase 14 (`audit_log`)
for status-change events; can start earlier with creation-only events if
prioritized ahead of Phase 14. See `DESIGN.md` §8 for the full design.
- [ ] Migration: add `confirmed_at` to `observations`, `tested_at` to `credential_usage`, `discovered_at` to `trust_relationships`.
- [ ] `backend/src/db`: `timeline_events` SQL view (`UNION ALL` across hosts/observations/credentials/notes/findings `created_at`, plus `audit_log` rows once Phase 14 lands).
- [ ] `GET /engagements/:id/timeline` (`?since=&until=`) returning the unified feed.
- [ ] Extend the Phase 8 graph builder / `GET /engagements/:id/graph` to accept `?as_of=<timestamp>`, filtering every contributing query by the new timestamp columns.
- [ ] Frontend: `routes/engagements/[id]/replay/+page.svelte` — split pane (Cytoscape graph + scrollable feed), a shared `asOfTimestamp` store, scrubber + play/pause, click-to-jump between feed and graph.
- **Exit check:** dragging the scrubber to a past point shows the graph without hosts/observations added after that point, and the feed scrolls to the matching event; clicking a feed event moves the scrubber to it.
- [ ] *(v2, later)* `presentable: bool` + `caption_md` override on relevant tables; full-screen curated "slide" mode for client-facing walkthroughs, reusing the same view and `as_of` filter.

---

## Suggested working style
- Land each phase as its own commit/PR-sized chunk; keep `docker compose up` green at every phase boundary.
- Backend and frontend for a phase can be built in parallel once the API shape for that phase is sketched (routes + DTOs before full logic).
- Seed data (Phases 6, 8, 9) should be added incrementally as new notes get mined — treat `DESIGN.md` §12 as a living checklist, not a one-time import.
