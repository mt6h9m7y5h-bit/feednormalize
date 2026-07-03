# FeedNormalize

Enterprise-grade product data normalization and validation API.

Upload supplier product feeds (CSV, JSON, TSV, NDJSON), track async normalization jobs, and download a standardized JSON catalog with field-level validation reports. Built with Rust (Axum/Tokio) for high-throughput feed processing.

## Quick start

### 1. API key

Protected endpoints require `x-api-key` (or `x-rapidapi-key` when proxied through RapidAPI).

**Local development** — copy `.env.example` to `.env`, start Postgres, and run the API:

```bash
cp .env.example .env
docker compose up -d
cargo run
```

On first boot, if the `api_keys` table is empty, the server seeds a key from `API_KEY_SEED` (default: `dev-test-api-key`).

**Production** — set `API_KEY_SEED` to a strong random value in Railway Variables before the first deploy. See [DEPLOYMENT.md](DEPLOYMENT.md) for details.

### 2. Health check (no auth)

```bash
curl -s http://localhost:3000/health
# {"status":"ok"}
```

### 3. Upload a feed

```bash
curl -s -X POST http://localhost:3000/feeds/upload \
  -H "x-api-key: dev-test-api-key" \
  -F "file=@products.csv"
```

Response (`202 Accepted`):

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "processing",
  "filename": "products.csv",
  "size_bytes": 12345
}
```

Poll `GET /jobs/{id}` until `status` is `finished` or `completed_with_errors`, then download normalized output:

```bash
curl -s -H "x-api-key: dev-test-api-key" \
  http://localhost:3000/jobs/{job_id}/download -o normalized.json
```

## API documentation

- Swagger UI: `/swagger-ui`
- OpenAPI JSON: `/api-docs/openapi.json`

## Partner integrations

For RapidAPI marketplace setup, pricing tiers, and copy-paste provider dashboard values, see [RAPIDAPI_LAUNCH.md](RAPIDAPI_LAUNCH.md).

## Deployment

Production target is **Railway**. Public URL pattern:

```
https://YOUR-SERVICE-NAME.up.railway.app
```

Required variables: `DATABASE_URL`. Recommended: `API_KEY_SEED`, `STORAGE_BACKEND=s3` with S3/R2 credentials for durable uploads.

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | PostgreSQL connection string |
| `API_KEY_SEED` | Initial API key when `api_keys` is empty |
| `RUST_LOG` | Tracing level (default: `info`) |
| `STORAGE_BACKEND` | `local` (dev) or `s3` (production) |
| `REDIS_URL` | Optional distributed rate limiting |

Full deployment steps: [DEPLOYMENT.md](DEPLOYMENT.md).

## Build

```bash
RUSTFLAGS="-D warnings" cargo build --release
```
