# PunchCraft Deployment Guide

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Environment Variables](#2-environment-variables)
3. [Building the Docker Image](#3-building-the-docker-image)
4. [Running the Container](#4-running-the-container)
5. [Reverse Proxy Setup](#5-reverse-proxy-setup)
6. [MongoDB Atlas Configuration](#6-mongodb-atlas-configuration)
7. [Cloudinary Configuration](#7-cloudinary-configuration)
8. [Resend Email Configuration](#8-resend-email-configuration)
9. [Production Checklist](#9-production-checklist)
10. [CI/CD with GitHub Actions](#10-cicd-with-github-actions)
11. [Logs and Monitoring](#11-logs-and-monitoring)
12. [Updating the Application](#12-updating-the-application)

---

## 1. Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| Docker | 24+ | With BuildKit enabled |
| A Linux VPS or cloud VM | — | Ubuntu 22.04 LTS recommended |
| Domain name | — | `thepunchcraft.com` |
| MongoDB Atlas account | — | M0 free tier works for staging |
| Cloudinary account | — | Free tier for development |
| Resend account | — | `thepunchcraft.com` domain verified |

---

## 2. Environment Variables

All configuration is injected at runtime. The image contains **no secrets**.

Create an `.env.production` file on your server (never commit this file):

```env
# ── Server ────────────────────────────────────────────────────────────────────
BIND_ADDR=0.0.0.0:8080
APP_ENV=production
RUST_LOG=info

# ── Frontend ──────────────────────────────────────────────────────────────────
FRONTEND_URL=https://thepunchcraft.com

# ── MongoDB Atlas ─────────────────────────────────────────────────────────────
MONGODB_URI=mongodb+srv://<user>:<password>@<cluster>.mongodb.net/?appName=PunchCraft
DB_NAME=punchcraft

# ── JWT ───────────────────────────────────────────────────────────────────────
# Generate with: openssl rand -hex 64
JWT_SECRET=<64-byte-hex-secret>

# ── Resend (email) ────────────────────────────────────────────────────────────
RESEND_API_KEY=re_<your_key>
EMAIL_FROM=PunchCraft <noreply@thepunchcraft.com>
ADMIN_EMAIL=admin@thepunchcraft.com

# ── Cloudinary ────────────────────────────────────────────────────────────────
CLOUDINARY_CLOUD_NAME=<cloud_name>
CLOUDINARY_API_KEY=<api_key>
CLOUDINARY_API_SECRET=<api_secret>
```

### Generating a secure JWT secret

```bash
openssl rand -hex 64
```

---

## 3. Building the Docker Image

### Local build

```bash
docker build -t punchcraft:latest .
```

### Tagging for a registry (e.g. GitHub Container Registry)

```bash
docker build -t ghcr.io/<your-org>/punchcraft:latest .
docker push ghcr.io/<your-org>/punchcraft:latest
```

### Build notes

- The first build takes ~5–10 minutes (compiling Rust + all dependencies).
- Subsequent builds that only change `src/` take ~1–2 minutes — dependencies are cached in a separate layer by `cargo-chef`.
- `Punchcraft-openapi.yaml` is embedded into the binary at compile time and does **not** need to be present on the server at runtime.

---

## 4. Running the Container

### Basic run

```bash
docker run -d \
  --name punchcraft \
  --restart unless-stopped \
  -p 8080:8080 \
  --env-file /etc/punchcraft/.env.production \
  punchcraft:latest
```

Store the env file in a protected location:

```bash
sudo mkdir -p /etc/punchcraft
sudo cp .env.production /etc/punchcraft/.env.production
sudo chmod 600 /etc/punchcraft/.env.production
```

### Verify it started

```bash
docker logs punchcraft
docker ps
curl http://localhost:8080/api/v1/auth/login   # expects 400 (no body), not 404
```

---

## 5. Reverse Proxy Setup

Never expose port 8080 directly. Use Nginx or Caddy as a TLS-terminating reverse proxy.

### Option A — Caddy (recommended, automatic HTTPS)

Install Caddy on the server:

```bash
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update && sudo apt install caddy
```

`/etc/caddy/Caddyfile`:

```
api.thepunchcraft.com {
    reverse_proxy localhost:8080
}
```

```bash
sudo systemctl reload caddy
```

Caddy automatically provisions and renews the TLS certificate via Let's Encrypt.

### Option B — Nginx + Certbot

`/etc/nginx/sites-available/punchcraft`:

```nginx
server {
    listen 80;
    server_name api.thepunchcraft.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name api.thepunchcraft.com;

    ssl_certificate     /etc/letsencrypt/live/api.thepunchcraft.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.thepunchcraft.com/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    client_max_body_size 20M;

    location / {
        proxy_pass         http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header   Host              $host;
        proxy_set_header   X-Real-IP         $remote_addr;
        proxy_set_header   X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
    }
}
```

```bash
sudo ln -s /etc/nginx/sites-available/punchcraft /etc/nginx/sites-enabled/
sudo certbot --nginx -d api.thepunchcraft.com
sudo nginx -t && sudo systemctl reload nginx
```

### DNS

Point `api.thepunchcraft.com` to your server's public IP with an A record:

```
api.thepunchcraft.com  A  <server-ip>   TTL 300
```

---

## 6. MongoDB Atlas Configuration

1. **Create a cluster** — M0 (free) is fine for staging; M10+ for production.
2. **Create a database user** — Database Access → Add New User → password auth. Grant `readWriteAnyDatabase` or scope to the `punchcraft` database only.
3. **Allow network access** — Network Access → Add IP Address. Add your server's static IP (preferred) or `0.0.0.0/0` (less secure).
4. **Get the connection string** — Connect → Drivers → copy the `mongodb+srv://...` URI. Replace `<password>` with your user's password and set `DB_NAME=punchcraft`.

### Recommended Atlas indexes

Run once against the `punchcraft` database via Atlas or mongosh:

```js
// users collection
db.users.createIndex({ email: 1 }, { unique: true })
db.users.createIndex({ activation_token: 1 }, { sparse: true })
db.users.createIndex({ reset_token: 1 }, { sparse: true })
db.users.createIndex({ refresh_token: 1 }, { sparse: true })

// profiles collection (whichever pattern your schema uses)
db.profiles.createIndex({ user_id: 1 })
db.profiles.createIndex({ status: 1 })
db.profiles.createIndex({ role: 1, status: 1 })
```

---

## 7. Cloudinary Configuration

1. Log in at [cloudinary.com](https://cloudinary.com) and open **Dashboard**.
2. Copy **Cloud name**, **API Key**, and **API Secret** into the env file.
3. (Recommended) Create a restricted API key scoped to upload-only for the backend.
4. Set up an **upload preset** if you want server-side unsigned uploads.

No additional Cloudinary server configuration is required — the API is called via HTTP from the backend.

---

## 8. Resend Email Configuration

1. Log in at [resend.com](https://resend.com).
2. **Domains** → Add `thepunchcraft.com` → copy the DNS records (SPF, DKIM) to your DNS provider.
3. Wait for verification (usually under 5 minutes after DNS propagates).
4. **API Keys** → Create a key scoped to `thepunchcraft.com`. Set this as `RESEND_API_KEY`.
5. Set `EMAIL_FROM=PunchCraft <noreply@thepunchcraft.com>` and `ADMIN_EMAIL=admin@thepunchcraft.com`.

### Required DNS records (example)

| Type | Name | Value |
|---|---|---|
| TXT | `@` | `v=spf1 include:amazonses.com ~all` |
| CNAME | `resend._domainkey` | `<dkim-value>.dkim.resend.com` |
| TXT | `_dmarc` | `v=DMARC1; p=none;` |

Exact values are provided by Resend after adding the domain.

---

## 9. Production Checklist

Before going live, verify each item:

### Security
- [ ] `JWT_SECRET` is at least 64 random bytes — never the dev default
- [ ] `.env.production` is owned by root, mode `600`, not in the Docker image
- [ ] MongoDB Atlas IP allowlist contains only the server IP
- [ ] Resend API key is domain-scoped (not a full-account key)
- [ ] `APP_ENV=production` — restricts CORS to `FRONTEND_URL` only
- [ ] Server firewall allows only ports 22, 80, 443 inbound

### Application
- [ ] `GET https://api.thepunchcraft.com/api-docs` loads Swagger UI
- [ ] `POST /api/v1/auth/register` sends activation email successfully
- [ ] `GET /api/v1/auth/verify-email?token=...` activates account
- [ ] `POST /api/v1/auth/login` returns `accessToken` + `refreshToken`
- [ ] `POST /api/v1/auth/refresh` issues a new `accessToken`
- [ ] `POST /api/v1/auth/logout` clears the refresh token
- [ ] `POST /api/v1/auth/forgot-password` sends reset email

### Infrastructure
- [ ] TLS certificate is valid (`https://` loads without browser warning)
- [ ] Container restarts automatically (`--restart unless-stopped`)
- [ ] Logs are accessible via `docker logs punchcraft`

---

## 10. CI/CD with GitHub Actions

Create `.github/workflows/deploy.yml`:

```yaml
name: Build and Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push image
        uses: docker/build-push-action@v5
        with:
          context: .
          push: true
          tags: ghcr.io/${{ github.repository }}:latest
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Deploy to server
        uses: appleboy/ssh-action@v1
        with:
          host: ${{ secrets.SERVER_HOST }}
          username: ${{ secrets.SERVER_USER }}
          key: ${{ secrets.SERVER_SSH_KEY }}
          script: |
            docker pull ghcr.io/${{ github.repository }}:latest
            docker stop punchcraft || true
            docker rm punchcraft || true
            docker run -d \
              --name punchcraft \
              --restart unless-stopped \
              -p 8080:8080 \
              --env-file /etc/punchcraft/.env.production \
              ghcr.io/${{ github.repository }}:latest
```

### Required GitHub secrets

| Secret | Description |
|---|---|
| `SERVER_HOST` | Server public IP or hostname |
| `SERVER_USER` | SSH user (e.g. `ubuntu`) |
| `SERVER_SSH_KEY` | Private SSH key for the server |

---

## 11. Logs and Monitoring

### Viewing logs

```bash
# Live logs
docker logs -f punchcraft

# Last 200 lines
docker logs --tail 200 punchcraft
```

Log level is controlled by `RUST_LOG`:

| Value | Output |
|---|---|
| `error` | Errors only |
| `warn` | Warnings + errors |
| `info` | Normal operation (recommended for production) |
| `debug` | Verbose — includes request details |

### Log format

Logs are structured via `tracing-subscriber` and output to stdout in the format:

```
2026-04-11T08:00:00.000000Z  INFO punchcraft: Starting PunchCraft API on 0.0.0.0:8080
2026-04-11T08:00:01.000000Z ERROR punchcraft::common::email: Resend rejected email ...
```

Pipe to a log aggregator (Datadog, Loki, CloudWatch) by configuring your Docker log driver or using a sidecar.

---

## 12. Updating the Application

### Manual update

```bash
# On the server
docker pull ghcr.io/<your-org>/punchcraft:latest
docker stop punchcraft && docker rm punchcraft
docker run -d \
  --name punchcraft \
  --restart unless-stopped \
  -p 8080:8080 \
  --env-file /etc/punchcraft/.env.production \
  ghcr.io/<your-org>/punchcraft:latest
```

### Zero-downtime consideration

The current setup has a brief (~1–2 second) gap during container swap. For true zero-downtime, run two containers behind a load balancer and perform a rolling replacement. This can be added later with Docker Swarm or Kubernetes when scale requires it.
