# FeedNormalize — Deployment Guide

Production target: **Railway** (primary). Fly.io is optional — see [Fly.io (optional)](#flyio-optional).

## Prerequisites

- [Git](https://git-scm.com/) and a [GitHub](https://github.com) account
- [Railway](https://railway.app) account
- This repository pushed to GitHub

---

## Step 1 — Initialize Git and push to GitHub

If the project is not yet a git repository:

```bash
cd /path/to/feednormalize

git init
git add .
git commit -m "Initial FeedNormalize release"
```

Create an empty repository on GitHub (no README/license — avoid merge conflicts), then:

```bash
git branch -M main
git remote add origin git@github.com:YOUR_USER/feednormalize.git
git push -u origin main
```

If a remote already exists, commit any pending changes and push:

```bash
git add .
git commit -m "Prepare Railway deployment"
git push origin main
```

**Never commit** `.env` — it is listed in `.gitignore`. Use `.env.example` as a template for local dev.

---

## Step 2 — Create a Railway project

1. Open [Railway](https://railway.app) → **New Project**.
2. Choose **Deploy from GitHub repo** and authorize Railway if prompted.
3. Select your `feednormalize` repository.
4. Railway detects `railway.toml` and builds with the **Dockerfile** (multi-stage, cargo-chef cached).

---

## Step 3 — Add PostgreSQL

1. In the project canvas, click **+ New** → **Database** → **PostgreSQL**.
2. Wait for the database service to provision.
3. Open the **PostgreSQL** service → **Variables** (or **Connect**) and copy `DATABASE_URL`.

---

## Step 4 — Configure FeedNormalize service variables

Open the **FeedNormalize** (app) service → **Variables**:

| Variable | Required | Value |
|----------|----------|-------|
| `DATABASE_URL` | **yes** | Reference the Postgres plugin variable, or paste the connection string |
| `API_KEY_SEED` | recommended | Strong random secret (e.g. `openssl rand -hex 32`) — used on **first boot only** when `api_keys` is empty |
| `RUST_LOG` | no | `info` (default in-app when unset) |
| `HOST` | no | `0.0.0.0` (Dockerfile default; required for container networking) |
| `PORT` | auto | Railway injects this — **do not override** |
| `REDIS_URL` | no | Optional Redis for distributed rate limits |
| `STORAGE_BACKEND` | prod | `s3` for durable uploads (see [Object storage](#object-storage-s3--r2)) |
| `S3_BUCKET` | s3 | Your bucket name |
| `S3_REGION` | s3 | AWS region (or `auto` for R2) |
| `AWS_ACCESS_KEY_ID` | s3 | Access key |
| `AWS_SECRET_ACCESS_KEY` | s3 | Secret key |
| `S3_ENDPOINT` | R2/MinIO | Custom HTTPS endpoint |

### `API_KEY_SEED` behavior

On startup, `db::init_pool` runs migrations, then `ApiKeyService::ensure_seed`:

- If `api_keys` already has rows → no change.
- If the table is empty → inserts one key derived from `API_KEY_SEED` (default: `dev-test-api-key`).

In production, set `API_KEY_SEED` **before the first deploy**. The app logs a warning if Railway production is detected and the seed is missing or still the dev default.

---

## Step 5 — Deploy and verify

Railway redeploys on every push to the connected branch. The first build may take several minutes (Rust compile).

### Health check

`railway.toml` configures `GET /health` with a 120s timeout.

```bash
curl -s https://YOUR-RAILWAY-DOMAIN.up.railway.app/health
# {"status":"ok"}
```

### Logs

In Railway → FeedNormalize service → **Deployments** → **View logs**. Expect:

```
tracing initialized (RUST_LOG defaults to info when unset)
database connected and migrations applied
starting FeedNormalize API
FeedNormalize API ready
```

### API docs

- Swagger UI: `https://YOUR-RAILWAY-DOMAIN.up.railway.app/swagger-ui`
- OpenAPI JSON: `https://YOUR-RAILWAY-DOMAIN.up.railway.app/api-docs/openapi.json`

### Authenticated request

Use the value you set in `API_KEY_SEED` (only known at insert time unless you stored it):

```bash
curl -s -H "x-api-key: YOUR_API_KEY_SEED" \
  https://YOUR-RAILWAY-DOMAIN.up.railway.app/health
```

---

## Environment variables reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | yes | — | PostgreSQL connection string |
| `PORT` | no | `3000` | HTTP port; Railway sets automatically |
| `HOST` | no | `0.0.0.0` | Bind address; keep `0.0.0.0` in containers |
| `RUST_LOG` | no | `info` | `tracing` filter (`debug`, `feednormalize=debug`, etc.) |
| `API_KEY_SEED` | no* | `dev-test-api-key` | Initial API key when `api_keys` table is empty |
| `API_KEY_SEED_NAME` | no | `default-dev-key` | Display name for the seeded key |
| `REDIS_URL` | no | — | Optional Redis for distributed rate limits |
| `STORAGE_BACKEND` | no | `local` | `local` (filesystem `./uploads/`) or `s3` |
| `S3_BUCKET` | s3 only | — | Bucket name when `STORAGE_BACKEND=s3` |
| `S3_REGION` | s3 only | — | AWS region (or set `AWS_REGION`) |
| `AWS_ACCESS_KEY_ID` | s3 only | — | S3/R2 access key |
| `AWS_SECRET_ACCESS_KEY` | s3 only | — | S3/R2 secret key |
| `S3_ENDPOINT` | no | — | Custom endpoint for R2/MinIO (HTTPS URL) |

\*Strongly recommended in production.

See `.env.example` for local development copy-paste values.

---

## Database migrations

Migrations run automatically on boot. In `src/db/mod.rs`:

```rust
sqlx::migrate!().run(&pool).await?;
```

`sqlx::migrate!` embeds SQL from `migrations/` at **compile time** (the Dockerfile copies `migrations/` into the builder stage before `cargo build`). The runtime image also includes `migrations/` for visibility; the running binary does not read those files from disk.

No separate `sqlx prepare` step is required unless you adopt `query!` compile-time checked macros.

---

## Docker (local smoke test)

```bash
docker build -t feednormalize .
docker run --rm -p 3000:3000 \
  -e DATABASE_URL=postgres://feednormalize:feednormalize@host.docker.internal:5432/feednormalize \
  -e API_KEY_SEED=local-test-key \
  feednormalize
```

Requires Postgres reachable from the container (`docker compose up -d` from this repo).

---

## Object storage (S3 / R2)

Railway container disk is **ephemeral** — use S3-compatible object storage in production.

### AWS S3

1. Create an S3 bucket in your AWS account.
2. Create an IAM user with `s3:PutObject`, `s3:GetObject`, `s3:HeadObject` on that bucket.
3. Set on the FeedNormalize service:

| Variable | Example |
|----------|---------|
| `STORAGE_BACKEND` | `s3` |
| `S3_BUCKET` | `feednormalize-prod` |
| `S3_REGION` | `eu-central-1` |
| `AWS_ACCESS_KEY_ID` | `AKIA...` |
| `AWS_SECRET_ACCESS_KEY` | `...` |

### Cloudflare R2

1. Create an R2 bucket in the Cloudflare dashboard.
2. Create R2 API tokens with read/write access.
3. Set:

| Variable | Example |
|----------|---------|
| `STORAGE_BACKEND` | `s3` |
| `S3_BUCKET` | `feednormalize` |
| `S3_REGION` | `auto` |
| `AWS_ACCESS_KEY_ID` | R2 access key ID |
| `AWS_SECRET_ACCESS_KEY` | R2 secret access key |
| `S3_ENDPOINT` | `https://ACCOUNT_ID.r2.cloudflarestorage.com` |

Object keys: `{job_id}/original_file` and `{job_id}/normalized_output.json`.

Local dev (`STORAGE_BACKEND=local` or unset) continues to use `./uploads/` with no credentials.

---

## RapidAPI — next steps

After FeedNormalize is live with a stable public URL:

1. **Create a provider account** at [RapidAPI Provider](https://rapidapi.com/provider).
2. **Add New API → Define API** with base URL `https://YOUR-RAILWAY-DOMAIN.up.railway.app`.
3. **Import OpenAPI** from `/api-docs/openapi.json` or define endpoints manually to match Swagger (`/swagger-ui`).
4. **Authentication**: API Key header — clients send `x-api-key` (or map RapidAPI’s `X-RapidAPI-Proxy-Secret` / `x-rapidapi-key` in your gateway layer).
5. **Pricing**: Add free/starter/pro tiers; set rate limits aligned with `rate_limit_per_minute` on seeded keys.
6. **Test** endpoints from the RapidAPI playground against your Railway deployment.
7. **Publish** to the marketplace when docs and pricing are ready.

Marketing copy and endpoint summaries: see `MARKETING.md` and `rapidapi_description.txt`.

---

## Fly.io (optional)

```bash
fly apps create your-feednormalize-name   # update app in fly.toml first
fly postgres create                       # or use an external DATABASE_URL
fly secrets set DATABASE_URL=postgres://... API_KEY_SEED=...
fly deploy
```

Health check and port settings are in `fly.toml`.

---

## Local development

Docker deployment does not replace local `cargo run`:

```bash
cp .env.example .env
# edit DATABASE_URL for your local Postgres
docker compose up -d          # Postgres only
cargo run
```

Build with warnings denied (matches Docker builder):

```bash
RUSTFLAGS="-D warnings" cargo build --release
```
