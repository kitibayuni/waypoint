# Architecture & maintainer notes

This document is for whoever picks up this codebase next. It explains the
non-obvious parts: why things are built the way they are, where the sharp
edges are, and what to check before changing certain subsystems. It assumes
you've read `README.md` (deployment/ops) already.

`docs/DESIGN.md` and `docs/BUILD_PLAN.md` are the original design spec and
the phase-by-phase build log this project was implemented against. They're
the primary source for *intent*; this document is the primary source for
*how that intent landed in the actual code, and what changed along the way*.

## 1. System overview

```
Browser ──HTTPS──▶ nginx ──┬─▶ /api/*  ──▶ backend (Axum, :8080, internal-only)
                            └─▶ /*      ──▶ static SvelteKit SPA build
                                              backend ──▶ PostgreSQL 16
                                              backend ──▶ attachments volume (filesystem)
```

- **Backend**: Rust + Axum. One binary (`backend`), one crate, no internal
  service boundaries — every route handler shares one `AppState` (a
  `PgPool`, the credential cipher, and the attachments directory path).
- **Frontend**: SvelteKit compiled to a **static SPA** (`ssr=false`,
  `adapter-static`). There is no Node server at runtime — nginx just serves
  the built files and falls back to `index.html` for client-side routing.
  This matters: anything that needs to happen per-request (auth checks,
  data fetching) happens client-side after the SPA boots, via `fetch` calls
  to `/api/*`. There's no server-rendered auth gate.
- **nginx**: reverse proxy + TLS terminator + static file server. It is the
  *only* thing that talks to the backend over plain HTTP; the backend
  itself has no TLS and isn't published on a host port (see
  `docker-compose.yml` — only nginx maps ports).
- **Database**: PostgreSQL 16. Fully normalized (see `docs/DESIGN.md` §4 for
  the schema). No ORM — every query is hand-written SQL via `sqlx`.

Nothing here talks to the public internet except through nginx, and nginx
only proxies `/api/`. If you're debugging "why doesn't X work in prod but
does in dev," check nginx's `location` blocks first.

## 2. Repository layout

```
backend/
  src/
    main.rs              — wiring: router assembly, login rate limiter, migrations-on-boot
    lib.rs                — module declarations (so `backend::` is usable from bin/create-admin.rs)
    state.rs              — AppState (pool, credential cipher, attachments dir)
    auth/                  — login, sessions, password hashing, rate limiting
    authz.rs               — EngagementRole + require_role() — the whole RBAC model
    crypto.rs               — AES-256-GCM for credential secrets at rest
    audit.rs                — log_action() — the one function every mutating route calls
    graph/mod.rs            — attack-graph builder (derived, not stored)
    search/mod.rs            — cross-entity full-text search
    report/mod.rs             — Markdown → HTML → PDF (wkhtmltopdf) report rendering
    import/                   — Nmap / Nessus / BloodHound parsers → one shared shape
    routes/                    — one file per resource; routes/mod.rs merges them all
    bin/create-admin.rs        — standalone binary, bootstraps the first admin user
  migrations/                  — 13 sqlx migrations, applied automatically on backend boot
frontend/
  src/
    lib/api/                   — one thin fetch wrapper per resource (mirrors backend/src/routes/)
    lib/components/            — shared Svelte components (Dashboard, graph legends, editors)
    lib/stores/                — auth.ts (current user) and replay.ts (as_of timestamp)
    routes/                     — file-based SvelteKit routing, mirrors the resource tree
nginx/
  nginx.conf                    — TLS + reverse proxy config
  Dockerfile                    — builds the frontend AND generates the self-signed dev cert
docs/
  DESIGN.md, BUILD_PLAN.md       — original spec and phased build log (historical, see below)
  ARCHITECTURE.md                — this file
```

`backend/src/routes/*.rs` and `frontend/src/lib/api/*.ts` are deliberately
1:1 — `routes/findings.rs` ↔ `lib/api/findings.ts`. If you add a resource,
keep that pairing; it's the main thing that makes this codebase navigable
without a map.

## 3. Request lifecycle & RBAC

Every request either hits a **public** route (`/healthz`, `/auth/login`) or
a **protected** route. Protected routes go through
`auth::middleware::require_auth` (`backend/src/auth/middleware.rs`), which:

1. Reads the `session_token` cookie.
2. Looks it up in `sessions` (token stored as a SHA-256 hash, never in
   plaintext — see §5).
3. Injects a `CurrentUser` extension into the request.

That gets you **authentication**. **Authorization** is a second, separate
step every route handler does explicitly by calling
`authz::require_role(pool, &user, engagement_id, EngagementRole::X)`. There
is no middleware-level authorization — it can't be, because the minimum
required role differs per route (viewers can read, testers can write,
leads can manage membership), and the `engagement_id` a request is scoped
to usually has to be resolved from a path param first (e.g. a finding ID →
its engagement ID) before the role check can even run. **If you add a new
mutating route, you must call `require_role` yourself — nothing else will
catch a missing check.** `EngagementRole` derives `Ord` (`Viewer < Tester <
Lead`) specifically so `role >= min` works as a one-line check.
`user.is_admin` bypasses this entirely and is checked first.

## 4. Auth & sessions

- Passwords: Argon2id (`auth/password.rs`).
- Sessions are **opaque random tokens**, not JWTs. `auth/session.rs`
  generates 32 random bytes, hex-encodes them as the value handed to the
  browser, and stores only the SHA-256 hash of that value in `sessions`.
  This means a database leak doesn't leak usable session tokens, and there's
  no session-signing secret to manage or rotate (this is also why the
  `SESSION_SECRET` env var that existed through Phase 14 was removed in
  Phase 15 — it was never actually read anywhere).
- Cookie flags: `HttpOnly`, `Secure`, `SameSite=Strict`. `Strict` (not
  `Lax`) was a deliberate Phase 15 hardening choice — this SPA has no
  cross-site login flows (no OAuth redirects, no external referrers that
  need the cookie attached), so `Strict` is sufficient CSRF protection on
  its own and avoids needing a separate CSRF token scheme. If you ever add
  a flow where a *different* site needs to navigate the user in with the
  cookie already attached, this will silently break — you'd know because
  the first request after such a redirect wouldn't be authenticated.
- `/auth/login` is rate-limited **per source IP** at 10 attempts/60s
  (`auth/rate_limit.rs`), reading the client IP from `X-Forwarded-For`
  (set by nginx — see `nginx/nginx.conf`). It's a plain in-memory
  `HashMap<IpAddr, Vec<Instant>>` behind a `Mutex`, scoped only to the login
  route via a middleware layer in `main.rs` — not shared AppState, so it
  resets on every backend restart. That's an intentional simplification: a
  new dependency (e.g. `tower_governor`) wasn't worth pulling in for a
  single route, and the reset-on-restart behavior is fine for an
  anti-credential-stuffing measure, not a hard security boundary.

## 5. Credential encryption at rest

`crypto.rs` implements AES-256-GCM over the `credentials.secret BYTEA`
column. Two things worth knowing:

- **Key derivation**: `CRED_ENC_KEY` (an arbitrary-length string from
  `.env`) is SHA-256-hashed down to exactly 32 bytes. This is so operators
  can put a memorable passphrase in `.env` instead of hand-generating an
  exact-length key — it is *not* a KDF like Argon2/scrypt and provides no
  extra protection against a weak passphrase. If `CRED_ENC_KEY` is short or
  guessable, the encryption is only as strong as that.
- **No key rotation support.** Rotating `CRED_ENC_KEY` requires decrypting
  every row in `credentials` with the old key and re-encrypting with the
  new one before swapping the env var, or those secrets become permanently
  unreadable. There's no migration tool for this (see `README.md`
  "Secrets rotation" — it's called out there as a known gap, not solved).
- Every route that returns credential data returns an already-redacted
  response struct that never includes `secret` — and that same struct is
  reused for audit-log snapshots (see §7), so there's exactly one place a
  credential secret could leak into a JSON response or audit trail, and
  that place has never had a `secret` field to leak in the first place.

## 6. The attack graph (`graph/mod.rs`)

The graph is **derived at request time**, not stored. `build_graph()` runs
five queries (hosts, credentials, observations, trust_relationships,
credential_usage) plus the `attack_path_rules` catalog, and assembles them
into Cytoscape.js's native `{ data: {...} }` element shape — the frontend
passes `[...nodes, ...edges]` straight into `cytoscape({ elements })` with
no reshaping.

**Technique nodes are conditional**: an attack-path rule only produces a
technique node + edge if its trigger observation type has a *confirmed*
observation on some host (not `suspected`, `remediated`, or
`false_positive`). This is the graph's only real "logic" — everything else
is a straight projection of table rows into nodes/edges.

**`as_of` replay** (`?as_of=<timestamp>`, added Phase 16) filters every one
of those five queries by a column meaning "this became true" rather than
"this row exists":

| Query | Live filter | Replay (`as_of`) filter |
|---|---|---|
| hosts | `engagement_id` | + `created_at <= as_of` |
| credentials | `engagement_id` | + `created_at <= as_of` |
| observations | `engagement_id` (via host) | + `confirmed_at IS NOT NULL AND confirmed_at <= as_of` |
| trust_relationships | `engagement_id` | + `discovered_at <= as_of` |
| credential_usage | `result = 'works'` | + `tested_at IS NOT NULL AND tested_at <= as_of` |

**This means the live graph and a replay at `as_of=now()` are not the same
shape.** The live graph shows every observation regardless of status
(suspected ones appear as plain nodes, just without a technique edge). A
replay at any timestamp shows *only confirmed* observations — an
observation that's still `suspected` never appears in replay mode, at any
`as_of` value, because it has no `confirmed_at`. This was a deliberate
reading of `docs/DESIGN.md` §8.2 (replay is about reconstructing *confirmed*
attack-path history, not open leads) — if a future maintainer wants replay
to also show suspected observations by their `created_at`, that's a
one-line change to the `WHERE` clause in `graph/mod.rs`, but it's worth
re-reading §8.2 first to make sure that's actually what's wanted.

`confirmed_at`/`tested_at` are set **once**, at the actual transition
(`observations.rs`: `WHEN $1 = 'confirmed' AND confirmed_at IS NULL THEN
now()`), not backfilled from `created_at` — except for rows that already
existed when migration `0013_timeline.sql` ran, where `created_at` was the
only information available and was used as a best-effort backfill. If you
see a `confirmed_at` that looks suspiciously identical to `created_at` on
an old row, that's why — it's an approximation for pre-migration data, not
a bug.

## 7. `audit_log` is load-bearing for three separate features

There is exactly one `audit_log` table (`actor_id, action, subject_type,
subject_id, before, after, at`), populated by one function
(`audit::log_action`, called from every create/update/delete handler for
hosts, observations, credentials, and findings). It backs:

1. **The audit trail itself** — `GET /api/audit-log` (admin-only, system-
   wide, `routes/audit.rs`).
2. **Finding version history** — `GET /api/findings/:id/history`
   (`routes/findings.rs`) is just `SELECT ... WHERE subject_type='finding'
   AND subject_id=$1`. There is no separate history/versioning table; this
   was a deliberate reuse per `docs/DESIGN.md` §9's own suggestion, not an
   oversight. If you need version history for a resource that doesn't call
   `log_action` yet (credential_usage, trust_relationships, notes,
   attachments, checklists — see the gap list below), it isn't there.
3. **Timeline status-change events** (`timeline_events` view, Phase 16) —
   derives `event_type` **generically** as `"<subject_type>_<new status>"`
   whenever `before->>'status' IS DISTINCT FROM after->>'status'`. This is
   why an observation transitioning to `confirmed` shows up as
   `observation_confirmed` and a finding transitioning to `triaged` shows up
   as `finding_triaged` in the timeline feed, without either string being
   hardcoded anywhere — the mechanism is generic over *any* status value on
   *either* table. If a third status-bearing table needs timeline coverage,
   add a `UNION ALL` branch to `timeline_events` following the same
   `before/after status diff` pattern, rather than adding a bespoke
   event-type enum.

**Gap**: `credential_usage`, `trust_relationships`, `notes`, `attachments`,
`checklists`, and `checklist_items` do **not** call `log_action`. Their
mutations produce no audit trail, no history, and no timeline events. This
was a scope decision (Phase 14 covered the four "highest value" resources
for review/audit purposes), not an accident — but it means, for example,
editing a note leaves no trace anywhere. Worth knowing before someone
assumes "the audit log has everything."

## 8. Import pipeline (`import/`, `routes/import.rs`)

Three independent parsers (Nmap XML, Nessus `.nessus`, BloodHound
`computers.json`) each normalize into one shared `ParsedImport` shape
(`hosts`, `findings`, `trust_relationships` — see `import/mod.rs`), so the
route handler's matching/dedup/write logic is written exactly once and
doesn't know or care which format produced the data.

Two things worth knowing if you touch this:

- **Preview/commit split.** `POST /import/:source/preview` parses the
  upload and returns a diff (what would be created vs. merged) *without
  writing anything*. `POST /import/:source/commit` re-parses and actually
  writes, inside a single transaction (`routes/import.rs`, `tx.commit()` at
  the end). The frontend's import page (`routes/[id]/import/+page.svelte`)
  shows the preview and only calls commit after the user confirms. There is
  no server-side state held between preview and commit — the file is
  re-uploaded and re-parsed for commit. This is simple but means preview and
  commit can theoretically disagree if the underlying data changed between
  the two calls (e.g. someone else created a colliding host in between) —
  low risk in practice given this is a single-team internal tool, but not
  literally atomic.
- **Hostname matching is case-insensitive on purpose, and this was a real
  bug once.** BloodHound exports FQDNs upper-case (`DC01.ACME.LOCAL`);
  Nmap/manual entry tends to be lower-case. Every hostname comparison in
  `routes/import.rs` (`match_host`, both `hostname_map` construction sites,
  and the post-creation insert) calls `.to_lowercase()` first. If you add a
  fourth importer or touch host matching, keep this — dropping it
  reintroduces silent host duplication across re-imports from different
  tools, which is exactly what happened during Phase 13 development before
  this fix landed.

## 9. Full-text search (`search/mod.rs`)

Six tables (`notes`, `findings`, `observations`, `hosts`, `credentials`,
`attachments`) each have a generated, stored `tsvector` column (see
migration `0007_cross_cutting.sql`) with a GIN index. `search()` queries
each table **independently** and concatenates results in a fixed order
(notes, findings, observations, hosts, credentials, attachments) — it does
**not** attempt to merge-sort by `ts_rank` across tables, because
`ts_rank` scores aren't meaningfully comparable across differently-shaped
documents (a short username field vs. a long finding description will rank
very differently even for equally-relevant matches). If a future maintainer
wants a single globally-ranked result list, that requires either
normalizing document length/field weights across tables or switching to a
different scoring approach — it isn't a small tweak to the current
per-table `ORDER BY ts_rank(...) DESC LIMIT 25`.

## 10. Report generation (`report/mod.rs`)

Markdown fields (`description_md`, `remediation_md`, `poc_md`, etc.) are
rendered to HTML via `pulldown-cmark`, then the assembled HTML document is
shelled out to **wkhtmltopdf** (`tokio::process::Command`, invoked with
`--disable-javascript`) to produce a PDF. This was chosen over a headless
Chromium approach (e.g. `headless_chrome`) specifically to avoid bundling a
~300MB+ browser into the runtime image — `wkhtmltopdf` is installed via
`apt` in the backend's Dockerfile runtime stage instead. `--disable-
javascript` is the actual XSS mitigation for report generation: any
`<script>` tag someone pasted into a Markdown note renders as inert markup,
not executable content, during PDF rendering.

## 11. MITRE ATT&CK integration

`mitre_techniques` (migration `0012_mitre_and_audit.sql`) is a small lookup
table (id, name, tactic, url), seeded with 5 techniques referenced by the
existing `attack_path_rules`. It is **not** a real foreign key relationship
— `attack_path_rules.mitre_technique_id`, `observation_types
.mitre_technique_ids`, and `findings.mitre_technique_ids` are all plain
text/JSONB fields matched to `mitre_techniques.id` by string equality at
query/display time only. This was deliberate (per `docs/DESIGN.md` §9):
technique IDs live in JSONB arrays and nullable text columns across several
tables, so a real FK isn't practical, and the lookup table's only job is
"what's the human-readable name/tactic for this ID" for display purposes.
Adding a new technique just means inserting a row; nothing enforces that
every ID referenced elsewhere actually exists in this table (a typo'd
technique ID silently just won't get a friendly name — no error).

## 12. CVSS calculator

Pure client-side (`frontend/src/lib/cvss.ts`), implementing the CVSS v3.1
base-score formula from the FIRST.org spec exactly (ISS → Impact →
Exploitability → Roundup). It's stateless and has no backend counterpart —
`findings.cvss_vector`/`cvss_score` are just plain columns the calculator's
"Apply" button writes into, same as if a user typed them in by hand. If
CVSS v4 support is ever needed, it's an additive second module, not a
rewrite of this one (findings don't record *which* CVSS version a score
came from — that's implicit in the vector string's `CVSS:3.1` prefix, which
nothing currently parses back out).

## 13. Templates (`routes/templates.rs`)

Five kinds (`host`, `checklist`, `note`, `finding`, `engagement`), each with
its own `instantiate_*` function. Only the kinds whose target table actually
has a `created_by` column take a `user: &CurrentUser` parameter to populate
it (`instantiate_host`, `instantiate_checklist`, `instantiate_note`,
`instantiate_engagement`) — `instantiate_finding` doesn't, because
`findings` has no `created_by` column at all. This asymmetry is easy to
misread as an inconsistency; it isn't, it's just following the schema. A
real bug did happen here once, though: instantiated notes had `created_by =
NULL` until the `CurrentUser` parameter was added to `instantiate_note` (it
was originally omitted). If you add a 6th template kind, check whether its
table has `created_by` first, and only thread `user` through if it does.

## 14. What's *not* here (known gaps, not oversights)

- **No automated test suite.** Every phase in `docs/BUILD_PLAN.md` was
  verified by hand against a live `docker compose up` stack (curl scripts,
  manual RBAC checks, manual UI passes) rather than `cargo test` /
  `vitest`. This is the single biggest risk area for a new maintainer:
  there's no regression safety net. If you're taking this project further,
  writing integration tests around `authz::require_role`, the graph
  builder's `as_of` filtering, and the import dedup logic would have the
  highest payoff — those are the three subsystems in this document with
  the most non-obvious behavior and the least self-evident correctness.
- **No CI pipeline** (no `.github/workflows`, no equivalent). Builds and
  checks in this repo's history were all run locally.
- **The `sqlx::query!`/`query_as!` compile-time macros are never used.**
  Every query in this codebase uses the runtime `sqlx::query`/`query_as`
  forms instead, deliberately — there's no `DATABASE_URL` or `.sqlx` offline
  cache configured for the build, so the compile-time macros would fail to
  build in CI/Docker without one. This was an actual mistake made and
  caught mid-project (`report/mod.rs` briefly used `query_as!`); if you add
  a query, copy the runtime-macro style from any neighboring file, not from
  `sqlx` documentation examples (which default to `query!`).
- **Docker base image pinning matters here.** The backend Dockerfile's
  builder stage is pinned to `rust:1.89-slim-bookworm` (not the floating
  `rust:1.89-slim` tag) because the floating tag silently moved to a newer
  Debian base with a newer glibc than the `debian:bookworm-slim` runtime
  stage, breaking the built binary at container startup
  (`GLIBC_2.39' not found`) with no code change on this project's side. If
  you bump the Rust version, re-pin to an explicit `-bookworm` (or whatever
  the runtime stage's Debian codename is) variant, not a floating tag.
- **Presentable/curated replay mode** (`docs/DESIGN.md` §8.4 — a
  `presentable: bool` + `caption_md` flag for a client-facing "slide mode"
  walkthrough) is explicitly deferred v2 and not implemented.
