# FeedNormalize — RapidAPI Launch Checklist

Copy-paste values for the [RapidAPI Provider Dashboard](https://rapidapi.com/provider). Confirm your live Railway URL before publishing.

---

## API Name

```
FeedNormalize
```

## Short Description

```
High-performance, Rust-powered API that normalizes any supplier product feed into one universal schema—no manual field mapping required.
```

## Long Description

```
FeedNormalize is e-commerce infrastructure for teams that integrate supplier catalogs at scale. Agencies, marketplaces, and storefront platforms receive product data in dozens of incompatible formats: product_price vs cost, article_number vs sku, mixed currencies, and broken encodings. Building and maintaining per-supplier parsers slows every integration.

FeedNormalize replaces that work with a single API. Upload a CSV or JSON feed, poll an async job, and download a clean, standardized catalog. A Rust/Tokio backend streams large files efficiently, maps common supplier field names automatically, and validates records as they are normalized—so you ship integrations faster without bespoke ETL scripts.

Key benefits:
• Rust-powered performance — Axum/Tokio streaming for large feeds
• Universal product schema — SKU, title, price, currency, EAN in one model
• Automatic field mapping — article_number, product_name, gross_price, and more
• Async job workflow — upload, poll, download with API key auth and rate limiting
• Webhook-ready — POST /webhooks/test for callback URL verification
```

## Category

Primary: **Data Enrichment**  
Secondary: **E-Commerce**

## Base URL

```
https://feednormalize-production.up.railway.app
```

> Replace with your current Railway public domain if different (Railway → Service → Settings → Networking → Public domain).

## Authentication

| Setting | Value |
|---------|-------|
| Auth type | API Key |
| Header name | `x-api-key` |
| Header location | Header |

**Notes for RapidAPI setup:**

- Direct-to-Railway testing: use the value of `API_KEY_SEED` from your Railway variables (default in dev: `dev-test-api-key`; production must be a strong random secret).
- RapidAPI Playground: RapidAPI injects `x-rapidapi-key` automatically. The API also accepts `x-rapidapi-key` as an alias for `x-api-key` when proxying.
- Configure RapidAPI to forward the subscription key or map `X-RapidAPI-Proxy-Secret` if using a custom gateway.

## OpenAPI Import

Import from your live deployment:

```
https://feednormalize-production.up.railway.app/api-docs/openapi.json
```

Or upload the repo file `openapi.yaml` manually.

Interactive docs (for manual review): `https://feednormalize-production.up.railway.app/swagger-ui`

## API Keys for Playground Testing

| Environment | Key source |
|-------------|------------|
| Railway production | `API_KEY_SEED` variable (set before first boot; seeded into `api_keys` table) |
| Local dev | `dev-test-api-key` (from `.env.example`) |

Generate a production seed:

```bash
openssl rand -hex 32
```

Set as `API_KEY_SEED` in Railway Variables, redeploy, then use that value as `x-api-key` when testing outside RapidAPI.

---

## Suggested Pricing Tiers

| Tier | Price | Requests/month | Rate limit | Notes |
|------|-------|----------------|------------|-------|
| **Basic (Free)** | $0 | 100 | 10/min | Preview + small feeds; playground testing |
| **Pro** | $19/mo | 5,000 | 60/min | Agencies, regular catalog syncs |
| **Ultra** | $79/mo | 50,000 | 300/min | Marketplaces, high-volume pipelines |
| **Mega** | Custom | Unlimited | Custom | Enterprise SLA, dedicated support |

Align RapidAPI tier rate limits with `rate_limit_per_minute` on seeded API keys in PostgreSQL.

---

## Step-by-Step Launch Checklist

1. **Verify Railway deployment** — `GET /health` returns `{"status":"ok"}`.
2. **Set production `API_KEY_SEED`** — strong random value in Railway Variables; redeploy if changed before first boot.
3. **Confirm OpenAPI** — open `/api-docs/openapi.json` and `/swagger-ui` on the live URL.
4. **Create RapidAPI provider account** — [rapidapi.com/provider](https://rapidapi.com/provider).
5. **Add New API → Define API** — paste Base URL from above.
6. **Import OpenAPI** — use `/api-docs/openapi.json` URL or upload `openapi.yaml`.
7. **Configure authentication** — API Key header `x-api-key`.
8. **Review endpoints** — upload, jobs, download, health, webhooks/test; edit summaries if needed.
9. **Set pricing tiers** — use table above as starting point.
10. **Test in RapidAPI Playground** — run curl commands below against production.
11. **Configure webhook test** (optional) — point RapidAPI callback URL to `POST /webhooks/test`.
12. **Add logo, terms, support email** — complete marketplace listing metadata.
13. **Submit for review** — publish when playground tests pass.

---

## Test Commands

Replace `YOUR_API_KEY` with your `API_KEY_SEED` value and `BASE_URL` with your Railway domain.

### Health (no auth)

```bash
curl -s "https://feednormalize-production.up.railway.app/health"
```

Expected: `{"status":"ok"}`

### Upload feed

```bash
curl -s -X POST "https://feednormalize-production.up.railway.app/feeds/upload" \
  -H "x-api-key: YOUR_API_KEY" \
  -F "file=@normalized-feed.csv"
```

Expected: `202` with `job_id`, `status`, `filename`, `size_bytes`.

### Preview job (no file upload)

```bash
curl -s -X POST "https://feednormalize-production.up.railway.app/jobs" \
  -H "x-api-key: YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"format":"csv","preview":{"article_number":"SKU-1","product_name":"Test Product","gross_price":"9.99","currency":"EUR"}}'
```

Expected: `202` with `job_id` and optional `preview_product`.

### Get job status

```bash
curl -s "https://feednormalize-production.up.railway.app/jobs/JOB_ID" \
  -H "x-api-key: YOUR_API_KEY"
```

Replace `JOB_ID` with UUID from upload/create response. Poll until `status` is `finished`.

### Download normalized output

```bash
curl -s "https://feednormalize-production.up.railway.app/jobs/JOB_ID/download" \
  -H "x-api-key: YOUR_API_KEY"
```

Expected: JSON array of `UniversalProduct` objects when job is `finished`.

### Webhook test (no auth)

```bash
curl -s -X POST "https://feednormalize-production.up.railway.app/webhooks/test" \
  -H "Content-Type: application/json" \
  -d '{"event":"test","source":"rapidapi"}'
```

Expected: `{"received":true}`

### OpenAPI document

```bash
curl -s "https://feednormalize-production.up.railway.app/api-docs/openapi.json" | head -c 200
```

Expected: JSON starting with `"openapi":"3.1.0"` (or similar).

---

## Error Responses (all protected endpoints)

| Status | Meaning | Example body |
|--------|---------|--------------|
| 400 | Bad request | `{"error":"missing file field"}` |
| 401 | Missing/invalid API key | `{"error":"missing API key"}` |
| 404 | Resource not found | `{"error":"job not found"}` |
| 429 | Rate limit exceeded | `{"error":"rate limit exceeded"}` |
| 500 | Server error | `{"error":"internal database error"}` |

Async endpoints return **202 Accepted** (not 201) for job creation and upload.
