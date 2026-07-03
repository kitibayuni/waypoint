# Engagement Manager — Architecture & Design Plan

> A self-hosted, Dockerized web application for **penetration-testing engagement
> management**. Engagement-centric (not report-centric), structured-data-first,
> with a rule-driven attack graph that turns observations into suggested
> exploitation paths.

This document is the planning blueprint. It is derived from the goals in
`instructions.txt` and synthesized from the enumeration/exploitation notes in
this directory (AD, SMB, LLMNR, Kerberoasting, credential hunting, pivoting,
reporting, the "8 steps" methodology, and the author's own `myweaknesses.txt`).

---

## 1. Design principles (inferred from the notes)

The existing notes are dense, service-organized enumeration references. Two
themes dominate and should drive the product:

1. **Structure beats free-text.** `myweaknesses.txt` states plainly:
   *"Biggest weakness is LACK OF STRUCTURE! You need to make checklists."* The
   app must make the structured path the easy path — templates that auto-populate
   checklists and note skeletons, structured observations over prose.
2. **Credentials and observations are the pivot currency.** Recurring lessons:
   *reuse creds across domains/services*, *act immediately on new usernames*,
   *run Responder/Inveigh on every new host*. Credentials, observations, and the
   relationships between hosts are first-class entities — not buried in notes.

Resulting principles:

- **Normalized, database-driven.** No hardcoded workflows; catalogs, rules, and
  templates live in the DB and are user-editable.
- **Everything attaches to a host, and every host attaches to an engagement.**
- **Observations are typed and catalog-backed**, so they can feed the attack
  graph and (later) MITRE ATT&CK / CVSS without schema changes.
- **Rich Markdown is allowed everywhere but never the primary source of truth.**
- **Extensible seams from day one:** importers, exporters (reports), and the
  attack-path rule engine are pluggable modules.

---

## 2. Technology stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Reverse proxy / TLS | **Nginx** | Terminates HTTPS, serves static frontend, proxies `/api` to backend. |
| Backend | **Rust + Axum** | Per requirements. Tokio async, tower middleware for auth/logging. |
| DB access | **SQLx** (compile-time checked queries) + **`sqlx migrate`** | Type-safe SQL, migrations in-repo, no ORM lock-in for the normalized schema. |
| Database | **PostgreSQL 16** | Relational core + `tsvector` full-text search + `jsonb` for flexible fields + `ltree`/recursive CTEs for graph/trust paths. |
| Frontend | **SvelteKit (SPA mode) + TypeScript** | Decided. Least boilerplate of the options considered (no hooks/useEffect ceremony, file-based routing, built-in reactivity) — see §2.1. Served as static assets by Nginx. |
| Attack-graph viz | **Cytoscape.js** | Purpose-built for node/edge graphs, layouts, styling; better fit than D3 for interactive attack graphs. |
| Markdown | server sanitizes + stores raw; render client-side (`markdown-it`) | Keep raw MD as source of truth; render on read. |
| Auth | Session cookie (Argon2 password hash) + RBAC | Simple, self-hosted friendly; JWT optional later for API tokens. |
| Orchestration | **Docker Compose** | Four services: `nginx`, `backend`, `frontend-build`, `db`. |

### 2.1 Frontend: SvelteKit (decided)

SvelteKit in SPA/static-adapter mode. Rationale over the alternatives considered
(React+Vite, Leptos/Rust-WASM): plain reactive `let`/`$:` state instead of
hooks/`useEffect` ceremony, no separate server-state library needed, built-in
file-based routing, and first-class stores for the live-updating attack graph.
React+Vite carries more boilerplate (hooks, dependency arrays, TanStack Query);
Leptos, despite sharing Rust types with the backend, ends up with the most
boilerplate of the three due to Rust's strictness and a much smaller ecosystem
for things like graph-viz bindings.

Structure:
```
frontend/
├── svelte.config.js         # adapter-static
├── vite.config.ts
└── src/
    ├── routes/
    │   ├── +layout.svelte           # shell, nav, auth guard
    │   ├── engagements/+page.svelte # list/dashboard entry
    │   ├── engagements/[id]/
    │   │   ├── +page.svelte         # engagement dashboard
    │   │   ├── hosts/[hostId]/+page.svelte
    │   │   ├── graph/+page.svelte   # Cytoscape attack graph
    │   │   ├── replay/+page.svelte  # timeline + graph replay (§8)
    │   │   ├── templates/+page.svelte
    │   │   └── search/+page.svelte
    │   └── login/+page.svelte
    └── lib/
        ├── api/              # typed fetch client per resource
        ├── stores/           # auth, active engagement, graph state
        └── components/       # HostCard, ObservationChip, ChecklistPanel, GraphView…
```

---

## 3. Repository / container layout

```
enum-webserver/
├── docker-compose.yml
├── .env.example                # DB creds, TLS paths, secrets
├── nginx/
│   ├── nginx.conf              # HTTPS, /api proxy, static SPA fallback
│   └── certs/                  # self-signed or provided TLS certs
├── backend/                    # Rust + Axum
│   ├── Cargo.toml
│   ├── migrations/             # sqlx SQL migrations (versioned)
│   ├── seeds/                  # observation catalog + attack-path rules (SQL/JSON)
│   └── src/
│       ├── main.rs             # router, middleware, TLS/proxy-aware setup
│       ├── config.rs
│       ├── db.rs               # pool
│       ├── auth/               # sessions, RBAC, password hashing
│       ├── domain/             # entity models (structs) + DTOs
│       ├── routes/             # one module per resource (engagements, hosts, …)
│       ├── graph/              # attack-graph builder + rule engine
│       ├── search/             # FTS query builder
│       ├── import/             # nmap / nessus / bloodhound parsers (feature-gated)
│       ├── report/             # engagement → HTML → PDF renderer
│       └── audit/              # audit-log middleware (future-ready)
├── frontend/
│   ├── package.json
│   └── src/
│       ├── routes/             # engagement list, dashboard, host, graph, templates
│       ├── lib/components/     # HostCard, ObservationChip, ChecklistPanel, GraphView…
│       └── lib/api/            # typed API client
└── docs/
    └── DESIGN.md               # (this file)
```

Compose services & flow:

```
Browser ──HTTPS──▶ nginx ──/──▶ static SPA (frontend build artifacts)
                     └────/api──▶ backend (Axum) ──▶ postgres
```

---

## 4. Data model (normalized, PostgreSQL)

Core hierarchy: **Engagement → Host → {Service, Observation, Credential,
Finding, Note, Attachment, Checklist}**. Templates and catalogs are separate,
reusable, engagement-agnostic tables.

### 4.1 Identity & collaboration
- `users` — id, email, display_name, password_hash (Argon2), is_admin, created_at.
- `engagement_members` — engagement_id, user_id, role (`lead`|`tester`|`viewer`). Enables per-engagement RBAC + team member listing.

### 4.2 Engagement layer
- `clients` — id, name, contacts (jsonb), notes.
- `engagements` — id, client_id, name, status (`planning`|`active`|`reporting`|`closed`), start_date, end_date, global_notes_md, created_by.
- `scope_items` — id, engagement_id, kind (`ip`|`cidr`|`domain`|`url`|`asn`|`exclusion`), value, in_scope (bool), note. *(Directly supports the `myweaknesses.txt` lessons about double-checking scope and target IPs.)*

### 4.3 Asset layer
- `hosts` — id, engagement_id, label, hostname, os, os_family, criticality, status (`discovered`|`enumerating`|`exploited`|`owned`|`cleared`), general_info_md, template_origin_id (nullable).
- `host_addresses` — id, host_id, ip (inet), is_primary. *(A host can have many IPs — "large networked environments might know us by different names.")*
- `services` — id, host_id, port, protocol (`tcp`|`udp`), name, product, version, banner, state. Seeds observations & attack surface.
- `tags` + `host_tags` (M:N) — free tagging, reused across search/filter.

### 4.4 Credentials (first-class)
- `credentials` — id, engagement_id, username, domain, secret (encrypted at rest), secret_type (`plaintext`|`ntlm`|`kerb`|`ssh_key`|`hash_other`), source_host_id, origin (`captured`|`cracked`|`sprayed`|`default`|`created`), validated (bool), notes_md.
- `credential_usage` — credential_id, host_id, service_id, result (`works`|`fails`|`untested`), privilege (`user`|`admin`|`domain_admin`|`system`). *(Models cred reuse across hosts/services — the app should actively surface "this cred is untested here.")*

### 4.5 Structured observations (the differentiator)
- `observation_types` — **catalog**. id, key (e.g. `smb_signing_disabled`), title, category (`smb`|`ad`|`web`|`tls`|`creds`|…), default_severity, description_md, references (jsonb), mitre_technique_ids (jsonb, future), editable by users.
- `observations` — id, host_id, service_id (nullable), observation_type_id, severity_override, status (`suspected`|`confirmed`|`remediated`|`false_positive`), evidence_md, created_by. Confirming an observation is what fires the attack-graph rules.

### 4.6 Findings & reporting
- `findings` — id, engagement_id, host_id (nullable, or M:N via `finding_hosts`), title, cve, cvss_vector, cvss_score, severity, description_md, remediation_md, poc_md, references (jsonb), status (`open`|`triaged`|`accepted_risk`|`fixed`), source_observation_id (nullable). *(Fields taken straight from `on-reporting.txt`: Name, CVE, CVSS, Description, References, Remediation, PoC, Affected Systems.)*
- `finding_hosts` — M:N "affected systems".

### 4.7 Notes, evidence, checklists
- `notes` — id, engagement_id, subject_type (`engagement`|`host`|`finding`|`observation`|`credential`), subject_id, title, body_md, created_by. Polymorphic Markdown attached to any entity.
- `attachments` — id, engagement_id, subject_type, subject_id, filename, mime, size, storage_path, sha256, caption. Screenshots/evidence on any entity.
- `checklists` — id, host_id (or engagement_id), name, template_origin_id.
- `checklist_items` — id, checklist_id, text, state (`todo`|`doing`|`done`|`na`), position, linked_note_id (nullable). *(Auto-created when a host is instantiated from a template — the structural backbone the author says they lack.)*

### 4.8 Templates (reusable, user-created)
- `templates` — id, kind (`host`|`checklist`|`finding`|`note`|`engagement`), name, description, owner_id, is_shared.
- `template_payloads` — template_id, body (jsonb). Payload shape depends on kind:
  - **host** → default tags, seeded checklists, note skeletons, expected services.
  - **checklist** → ordered item texts (e.g. an "SMB enumeration" checklist synthesized from `common-services/smb.txt`).
  - **finding** → prefilled title/description/remediation/CVSS scaffold.
  - **note** → Markdown skeleton (e.g. the "login procedure" template the author explicitly wanted).
  - **engagement** → default scope kinds, member roles, standard findings, dashboard config.

"Create host from template" = copy payload → materialize host + checklists + notes + tags in one transaction.

### 4.9 Attack graph & rule engine
- `attack_path_rules` — **catalog, DB-driven.** id, trigger_observation_type_id, technique (e.g. `NTLM Relay`, `Responder`), outcome (e.g. `Lateral Movement`, `Hash Capture`), next_step_md, mitre_technique_id (future), enabled. Example rows encode the instructions' examples:
  - `smb_signing_disabled` → *NTLM Relay* → *Lateral Movement*
  - `llmnr_enabled` → *Responder* → *Hash Capture*
- `trust_relationships` — id, engagement_id, from_host_id, to_host_id, kind (`domain_trust`|`admin_of`|`shares_creds`|`session`), direction, note. Feeds AD/trust edges.

The graph itself is **derived, not stored**: a builder assembles nodes
(hosts, credentials, key observations) and edges (trust relationships, credential
usage, and rule-matched attack paths), returning a JSON graph the frontend renders
in Cytoscape.js. "Suggested next steps" = the set of `next_step_md` for every
enabled rule whose trigger observation exists but whose outcome isn't yet
achieved. Because rules live in the DB, the catalog grows without code changes.

### 4.10 Cross-cutting
- `audit_log` — id, actor_id, action, subject_type, subject_id, before/after (jsonb), at. (Future feature, table defined now.)
- `search_index` — materialized `tsvector` per searchable row, or per-table generated `tsvector` columns + GIN indexes. Covers notes, findings, observations, hosts, credentials (username/domain only), attachments (filename/caption).

---

## 5. Attack-graph pipeline

```
Observation confirmed ──▶ rule engine matches attack_path_rules
        │                        │
        ▼                        ▼
   graph builder  ◀── trust_relationships + credential_usage + hosts
        │
        ▼
   GET /api/engagements/:id/graph  ─▶  { nodes[], edges[], suggestions[] }
        │
        ▼
   Cytoscape.js view: hosts, creds, trusts, attack paths, priv-esc, next steps
```

Node types color-coded: host / credential / observation / technique. Edge types:
trust, cred-reuse, attack-path (from rules), priv-esc. The view updates whenever
observations/creds/trusts change (poll or SSE).

---

## 6. API surface (REST, `/api`)

Resource-per-module, all engagement-scoped where applicable:

```
/auth/{login,logout,me}
/engagements                      CRUD + /:id/dashboard  /:id/graph  /:id/search
/engagements/:id/scope
/engagements/:id/members
/engagements/:id/timeline         GET → unified event feed (?since=&until=)
/engagements/:id/graph?as_of=     graph state reconstructed as of a timestamp
/hosts            (?engagement=)  CRUD + /:id/from-template
/hosts/:id/{services,observations,credentials,checklists,notes,attachments,findings}
/observations     + /catalog (observation_types CRUD)
/credentials      + /:id/usage
/findings
/templates        (?kind=)        CRUD + /:id/instantiate
/attack-rules                     CRUD (rule catalog)
/import/{nmap,nessus,bloodhound}  POST file → normalized rows (feature-gated)
/reports/:engagement_id           GET → HTML/PDF
/search?q=&types=                 cross-entity FTS
```

---

## 7. Dashboard (per engagement)

Synthesized from the "8 steps" methodology and reporting notes. Tiles:
- Progress: hosts by status, checklist completion %, observations confirmed vs suspected.
- Findings by severity (feeds executive summary).
- Credentials captured / validated / reuse coverage.
- Attack-graph mini-view + top suggested next steps.
- Timeline vs. scope window (start/end dates, days remaining).

---

## 8. Timeline & Replay view

A synchronized **map + feed** presentation of how an engagement unfolded —
scrub a timeline and watch the attack graph reconstruct itself as it looked at
that moment, with the notes/observations/findings feed for that moment
alongside it. Two audiences, one foundation:

1. **Internal review (v1, build this first):** testers/leads reconstructing
   their own path through an engagement — "what did we know on day 3?"
2. **Client-facing presentation (v2, later):** a curated, narrated walkthrough
   for demoing findings — same underlying data, added curation (pick which
   events surface) and polish (captions, full-screen, maybe export/share).

### 8.1 Data foundation — derived, not duplicated

No new tables for the raw feed. Every timestamped entity already carries a
`created_at` (hosts, observations, credentials, notes, findings, attachments),
and status transitions (observation confirmed, finding triaged) are captured
by the `audit_log` table (§4.10, built in Phase 14). The **timeline is a SQL
view**, not a stored table — a `UNION ALL` across these tables' timestamp
columns, projected into a common shape:

```
timeline_events: (engagement_id, at, event_type, subject_type, subject_id, title, summary)
```

`event_type` distinguishes creation events (`host_added`, `observation_added`,
`credential_captured`, `note_added`, `finding_added`) from status-change
events sourced from `audit_log` once Phase 14 lands (`observation_confirmed`,
`finding_triaged`). This mirrors how the attack graph itself is already
derived at request time (§5) rather than stored — consistent with the
normalized, database-driven design principle in §1.

### 8.2 Replay: graph state "as of" a timestamp

The graph builder (§5) already assembles nodes/edges from live table state.
Replay extends it with an optional `as_of` timestamp: filter every
contributing query (`hosts.created_at <= as_of`, `observations` whose
`confirmed_at <= as_of`, `credential_usage.tested_at <= as_of`,
`trust_relationships.discovered_at <= as_of`) before assembling the same JSON
graph shape. No new graph logic — the existing builder becomes
time-parameterized. Requires adding `confirmed_at` / `tested_at` /
`discovered_at` timestamp columns alongside the existing `created_at` ones
where a status transition matters (observations, credential_usage,
trust_relationships) — a small addition to the §4 migrations, not a schema
redesign.

### 8.3 UI: split-pane scrubber

```
routes/engagements/[id]/replay/+page.svelte
┌─────────────────────────────┬───────────────────────────┐
│  Cytoscape graph             │  Feed (scrollable)         │
│  (as_of = scrubber position) │  ◆ Host added: DC01        │
│                               │  ◆ SMB Signing Disabled    │
│                               │  ◆ Credential captured     │
│                               │  ◆ Suggested: NTLM Relay   │
├─────────────────────────────┴───────────────────────────┤
│  [====================o------------------] Day 3 of 14   │
│  ▶ Play    ⏮ Jump to event                                │
└───────────────────────────────────────────────────────────┘
```

A shared Svelte store (`asOfTimestamp`) drives both panes: dragging the
scrubber re-fetches `/graph?as_of=` and scrolls the feed to the matching
event; clicking a feed event moves the scrubber. "Play" auto-advances through
events at a fixed interval for a hands-off walkthrough.

### 8.4 Later: curated presentation mode (v2)

Once v1 (internal, audit-log-driven) is in use, layer a presentation mode on
the same foundation: testers mark which timeline events are "presentable",
attach a short caption (defaults to the observation/finding's own
description), and the replay view gains a full-screen "slide" layout stepping
through only the curated events — still backed by the same
`timeline_events` view and `as_of` graph filter, just with a
`presentable: bool` flag and optional `caption_md` override added to the
relevant tables. No separate data pipeline needed.

---

## 9. Extensibility seams (future features, no schema churn)

- **MITRE ATT&CK:** `mitre_technique_ids` already on `observation_types` / `attack_path_rules` / `findings`; add a `mitre_techniques` reference table when needed.
- **CVSS:** `cvss_vector` + `cvss_score` already on `findings`; add a client-side calculator.
- **Importers:** `import/` module parses Nmap XML / Nessus / BloodHound into hosts, services, observations, trust_relationships. Each parser is independent.
- **Report generation:** `report/` renders engagement → HTML template (Executive Summary, Overview, Scope & Duration, Findings) → PDF. Structure mirrors `on-reporting.txt`.
- **Collaboration / version history / audit:** `engagement_members` (RBAC) and `audit_log` tables exist now; add row-versioning triggers later.

---

## 10. Security & deployment notes

- HTTPS enforced at Nginx; HSTS; backend trusts `X-Forwarded-*` only from proxy.
- Passwords: Argon2id. Session cookies: `HttpOnly`, `Secure`, `SameSite=Lax`.
- **Credential secrets encrypted at rest** (app-level AEAD with a key from env/secret), never returned in list endpoints, redacted in logs.
- RBAC enforced per engagement (`lead`/`tester`/`viewer`).
- Secrets & TLS certs via `.env` / mounted volumes, never committed.
- This is engagement data for *authorized* testing; treat the DB as sensitive.

---

## 11. Suggested build order (milestones)

1. **Scaffold:** Docker Compose (nginx + Axum + Postgres), health check, TLS, migration runner.
2. **Auth + engagements + members** (RBAC skeleton).
3. **Hosts + addresses + services + tags**, with dashboard v1.
4. **Structured observations + observation-type catalog** (seed from notes).
5. **Credentials + usage tracking.**
6. **Attack-path rules + graph builder + Cytoscape view.**
7. **Templates (all 5 kinds) + "create host from template".**
8. **Findings + Markdown notes + attachments; cross-entity FTS search.**
9. **Report generation.**
10. **Importers, MITRE, CVSS calculator, audit/version history.**

---

## 12. Seed data to extract from existing notes

The `observation_types`, `attack_path_rules`, `checklist` templates, and
`note` templates should be pre-seeded from this directory's notes:

| Source note | Seeds |
|-------------|-------|
| `active-directory/LLMNR-NBT-NS-poisoning.txt` | `llmnr_enabled` → Responder → Hash Capture rule; AD checklist |
| `common-services/smb.txt` | SMB enumeration checklist; `smb_signing_disabled` observation |
| `active-directory/kerberoasting.txt` | `kerberoastable_spn` observation → Kerberoast → Offline Crack |
| `active-directory/initial-recon.txt` | Engagement/AD recon checklist template |
| `notes/on-reporting.txt` | Finding template fields + report section layout |
| `notes/8-steps.txt` | Engagement-template default workflow / dashboard phases |
| `notes/myweaknesses.txt` | "Login procedure" note template; cred-reuse reminders; scope double-check prompts |

