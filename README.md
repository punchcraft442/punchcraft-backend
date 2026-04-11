# PunchCraft API

Backend API for PunchCraft — a boxing ecosystem platform for profiling, verifying, and connecting fighters, gyms, coaches, referees/judges, promoters, matchmakers, and fans.

---

## Tech Stack

| Concern | Choice |
|---|---|
| Language | Rust |
| Web framework | actix-web 4 |
| Database | MongoDB (Atlas) via `mongodb` v3 |
| Image storage | Cloudinary (via `reqwest`) |
| Auth | JWT (`jsonwebtoken`) + bcrypt passwords |
| Email | Resend (via `reqwest`, no SDK) |
| Validation | `validator` |
| Async runtime | Tokio |

---

## Project Structure

```
src/
  auth/           # Registration, login, token management, password flows
  users/          # Current user profile (/users/me)
  profiles/       # Fighter, gym, coach, official, promoter, matchmaker, fan profiles
  directories/    # Public-facing profile listings and search
  admin/          # Admin approval, moderation, verification, audit logs
  verification/   # Verification document submission (stub)
  moderation/     # Report and content moderation (stub)
  media/          # Media upload handling (stub)
  notifications/  # User notifications (stub)
  common/         # DB connection, error types, middleware, email service, response helpers
  docs/           # Swagger UI + OpenAPI YAML serving
  main.rs         # Server bootstrap
  lib.rs          # Crate root (exposes all modules for integration tests)

tests/
  unit/           # Domain logic and validation unit tests
  integration/    # Full API flow integration tests (auth, profiles, admin)
  common/         # Shared test helpers (build_app!, make_jwt, setup_db, teardown_db)

docs/
  api.yml             # Canonical OpenAPI 3.0.3 specification
  database.md         # MongoDB schema and collection design
  api-docs.md         # API documentation notes
  deployment.md       # Production deployment guide

Punchcraft-openapi.yaml   # Served spec (embedded into binary at compile time)
PunchCraft.postman_collection.json
Dockerfile
```

---

## What Has Been Implemented

### Authentication (`/api/v1/auth`)

Full authentication system with email verification, short-lived access tokens, and persistent refresh tokens.

| Method | Endpoint | Auth | Description |
|---|---|---|---|
| POST | `/auth/register` | — | Create account. Account starts inactive; activation email sent. |
| GET | `/auth/verify-email?token=` | — | Activate account via emailed link. |
| POST | `/auth/login` | — | Returns `accessToken` (15 min) + `refreshToken` (30 days) + user summary. |
| POST | `/auth/refresh` | — | Exchange a valid refresh token for a new access token. |
| POST | `/auth/logout` | Bearer | Invalidates the refresh token. |
| POST | `/auth/forgot-password` | — | Sends password reset link. Always returns 200 (prevents enumeration). |
| POST | `/auth/reset-password` | — | Resets password using a one-time token. |
| PATCH | `/auth/change-password` | Bearer | Change password while authenticated. |

**Token design:**
- Access tokens: 15-minute JWTs signed with `JWT_SECRET`
- Refresh tokens: 30-day UUIDs stored on the user document in MongoDB
- Activation tokens: 7-day UUIDs stored on the user document, cleared on activation
- Reset tokens: 1-hour UUIDs stored on the user document, cleared on use

**camelCase JSON contract:**
- Requests: `newPassword`, `currentPassword`, `refreshToken`
- Responses: `accessToken`, `refreshToken`, `userId`, `accountStatus`
- `accountStatus` values: `inactive` | `active` | `suspended` | `disabled` | `deleted`

---

### Profiles (`/api/v1/profiles`)

Full CRUD and submission workflow for all profile types.

**Supported role types:** `fighter`, `gym`, `coach`, `official`, `promoter`, `matchmaker`, `fan`

**Profile lifecycle:** `draft` → `submitted` → `approved` | `rejected`

- Users create and edit profiles in `draft` state
- Submitting sends for admin review
- Admin approves or rejects with a reason
- Visibility is admin-controlled only (not exposed to users)

---

### Admin (`/api/v1/admin`)

Admin-only operations (requires `admin` or `super_admin` role).

- Approve and reject profiles
- Assign verification tier to profiles
- Control profile visibility
- View and resolve moderation reports
- Hide media
- Review verification documents
- Audit log access

---

### Directory (`/api/v1/directories`)

Public-facing paginated profile listings with filtering by keyword, region, city, verification tier, weight class, and sort order.

---

### Email

Transactional email via Resend using the verified `thepunchcraft.com` domain.

| Trigger | Recipient | Description |
|---|---|---|
| Register | User | Account activation link |
| Profile submitted | Admin | Notification of pending review |
| Profile approved | User | Approval confirmation |
| Profile rejected | User | Rejection reason |
| Forgot password | User | Password reset link |

All email sends are fire-and-forget (`tokio::spawn`) and do not block request handling.

---

### API Documentation

- **Swagger UI:** `GET /api-docs`
- **Raw OpenAPI YAML:** `GET /api-docs/openapi.yaml`

The spec is embedded into the binary at compile time from `Punchcraft-openapi.yaml`.

---

### CORS

| `APP_ENV` | Allowed Origins |
|---|---|
| `development` | All origins |
| `production` | `FRONTEND_URL` only |

---

## Environment Variables

```env
# Server
BIND_ADDR=0.0.0.0:8080
APP_ENV=development          # or production
RUST_LOG=info
FRONTEND_URL=http://localhost:3000

# MongoDB
MONGODB_URI=mongodb+srv://<user>:<password>@<cluster>.mongodb.net/
DB_NAME=punchcraft

# Auth
JWT_SECRET=<min 64 random bytes>

# Email (Resend)
RESEND_API_KEY=re_<key>
EMAIL_FROM=PunchCraft <noreply@thepunchcraft.com>
ADMIN_EMAIL=admin@thepunchcraft.com

# Cloudinary
CLOUDINARY_CLOUD_NAME=<cloud>
CLOUDINARY_API_KEY=<key>
CLOUDINARY_API_SECRET=<secret>
```

---

## Running Locally

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Copy and fill in environment variables
cp .env.example .env

# Run the server
cargo run

# Run tests (stop the server first on Windows — binary lock)
cargo test
```

The server starts on `http://localhost:8080` by default.

---

## Running with Docker

```bash
# Build the image
docker build -t punchcraft .

# Run with environment variables
docker run -d \
  --name punchcraft \
  --restart unless-stopped \
  -p 8080:8080 \
  --env-file .env.production \
  punchcraft
```

See `docs/deployment.md` for the full production deployment guide including reverse proxy, MongoDB Atlas, Resend, CI/CD, and monitoring setup.

---

## Testing

```bash
# All tests
cargo test

# Integration tests only
cargo test --test integration_tests

# Unit tests only
cargo test --test unit_tests

# Specific module
cargo test --test integration_tests auth
```

**Test conventions:**
- Each integration test spins up its own isolated MongoDB database (`pct_<uuid>`) and drops it after the test
- JWT secret is fixed to `"punchcraft_test_secret"` across all tests
- `register_and_activate(app, db, email, password)` must be used instead of `register_user` wherever a login follows — accounts start inactive

---

## Stub Modules (Not Yet Implemented)

Routes are wired but handlers are empty:

| Module | Endpoints |
|---|---|
| `verification` | Verification document submission |
| `moderation` | Report submission |
| `media` | Media upload |
| `notifications` | User notifications |

---

## Key Design Decisions

- **Minimal dependencies** — no heavy ORM or framework beyond actix-web. MongoDB queries written directly against the driver.
- **Modular structure** — each domain (auth, profiles, admin, etc.) is a self-contained module with its own handlers, service, models, and routes.
- **No secrets in the image** — the Docker image contains only the binary. All config is injected at runtime via environment variables.
- **Fire-and-forget email** — email sends never block the HTTP response. Failures are logged but do not surface as API errors.
- **Documentation-first** — `docs/` is the source of truth. Implementation follows the spec, not the other way around.
