# Engagement Manager

Self-hosted pentest engagement management app: clients, engagements, scope,
hosts, credentials, observations, findings, attack-path graph, reporting.
Backend is Rust/Axum + PostgreSQL, frontend is a SvelteKit SPA, nginx serves
the SPA and reverse-proxies/terminates TLS.

## Quick start (development)

1. `cp .env.example .env` and fill in real values for `POSTGRES_PASSWORD` and
   `CRED_ENC_KEY` (a random 32+ character string — this key encrypts stored
   credential secrets at rest, see "Secrets rotation" below).
2. `docker compose up -d --build`
3. Create the first admin user:
   ```
   docker compose exec backend ./create-admin <email> "<display name>" <password>
   ```
4. Visit `https://localhost` (or `https://localhost:${HTTPS_PORT}` if you
   changed the port). The dev stack serves a self-signed certificate
   generated automatically at image build time — your browser will warn
   about it; that's expected in dev.

No manual certificate setup is required for development: `nginx/Dockerfile`
generates a fresh self-signed `fullchain.pem`/`privkey.pem` on every build.

## TLS certificates in production

The dev image's self-signed cert is fine for local use but browsers/clients
should never trust it for a real deployment. For production, supply a real
certificate (e.g. from Let's Encrypt) and mount it over the container's
`/etc/nginx/certs`, overriding the build-time self-signed one:

1. Obtain a certificate for your domain, e.g. with `certbot certonly
   --standalone -d your.domain`, which writes to
   `/etc/letsencrypt/live/your.domain/{fullchain.pem,privkey.pem}`.
2. Point `PROD_CERTS_DIR` in `.env` at that directory (or copy the two files
   into `./prod-certs/` next to this README, which is the default).
3. Start the production stack instead of the dev one:
   ```
   docker compose -f docker-compose.prod.yml up -d --build
   ```
4. Renew certs on your usual schedule (e.g. a host cron running `certbot
   renew`) and `docker compose -f docker-compose.prod.yml restart nginx`
   afterwards to pick up the renewed files — the bind mount is read-only and
   live, but nginx only reads the cert at startup.

`docker-compose.prod.yml` also drops in resource limits (`deploy.resources.limits`
per service) and keeps `restart: unless-stopped` on every service; it has no
dev bind-mounts.

## Login rate limiting

`POST /api/auth/login` is rate-limited per source IP (from `X-Forwarded-For`,
set by nginx): 10 attempts per rolling 60-second window, after which the
backend returns `429 Too Many Requests`. This is in-memory per backend
process, so it resets on restart — acceptable for the anti-credential-stuffing
purpose it serves here.

## CSRF

The session cookie is `HttpOnly`, `Secure`, and `SameSite=Strict`. Since the
SPA and API are same-origin and there are no cross-site login flows to
support, `Strict` alone is sufficient CSRF protection for state-changing
requests — no separate CSRF token scheme is needed.

## Backup and restore (PostgreSQL volume)

Data lives in the `db_data` named volume. Attachments live in a separate
`attachments_data` volume — back both up together so restored findings still
have their evidence files.

**Backup:**
```
docker compose exec db pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" -F c -f /tmp/backup.dump
docker compose cp db:/tmp/backup.dump ./backup-$(date +%Y%m%d).dump
docker run --rm -v application_attachments_data:/data -v "$PWD":/backup alpine \
  tar czf /backup/attachments-$(date +%Y%m%d).tar.gz -C /data .
```

**Restore** (into a fresh stack, i.e. after `docker compose down` — not
`-v`, so volumes still exist, or after recreating them):
```
docker compose cp ./backup-YYYYMMDD.dump db:/tmp/backup.dump
docker compose exec db pg_restore -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists /tmp/backup.dump
docker run --rm -v application_attachments_data:/data -v "$PWD":/backup alpine \
  sh -c "rm -rf /data/* && tar xzf /backup/attachments-YYYYMMDD.tar.gz -C /data"
```

Test restores periodically — a backup you haven't restored is a guess, not a
backup.

## Secrets rotation

`.env` holds three real secrets (never commit it):

- `POSTGRES_PASSWORD` — rotate by changing it in `.env` and running
  `docker compose exec db psql -U "$POSTGRES_USER" -c "ALTER USER \"$POSTGRES_USER\" WITH PASSWORD '<new>';"`,
  then updating `DATABASE_URL` in `.env` to match and restarting the
  `backend` service.
- `CRED_ENC_KEY` — encrypts stored credential secrets (AES-256-GCM) at rest.
  **Rotating this key requires re-encrypting every row in `credentials`**
  (decrypt with the old key, re-encrypt with the new one) before swapping
  the env var, or those secrets become unreadable. There is no built-in
  rotation tool for this yet; treat it as a one-time-generated, long-lived
  key unless you script a migration.
- `POSTGRES_USER` / `POSTGRES_DB` — cosmetic identifiers, not sensitive; no
  rotation needed.

Session tokens themselves are opaque random values hashed (SHA-256) before
storage — there's no shared session-signing secret to rotate. Rotating any
of the above takes effect on the next `docker compose up -d` (or restart of
the affected service); existing logged-in sessions in the database are
unaffected by `.env` changes.

## Further documentation

- **`docs/ARCHITECTURE.md`** — maintainer-facing notes on how the system
  fits together, the non-obvious design decisions, and known gaps/sharp
  edges. Read this before making non-trivial changes.
- **`docs/DESIGN.md`** — the original design spec (schema, API shape,
  rationale) this app was built against.
- **`docs/BUILD_PLAN.md`** — the phased build log (what was built when,
  and why, in the order it happened).
