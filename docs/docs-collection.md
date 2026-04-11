# PunchCraft Documentation Set (Production-Level)

This document represents the full, expanded documentation set. Each section corresponds to an individual file in `/docs`.

---

# 01-system-architecture.md

## Architecture Style

Modular Monolith (V1) with clear domain boundaries.

## Core Domains

* Identity (Auth, Users)
* Profiles (All stakeholder types)
* Trust (Verification, Approval)
* Directory (Search & discovery)
* Communication (Contact requests, notifications)
* Moderation
* Media
* Admin Operations

## Layered Design

* Presentation Layer (clients)
* API Layer (routing + validation)
* Domain Layer (business rules)
* Data Layer (MongoDB)
* Infrastructure Layer (storage, email, logging)

## Key Principle

> No profile becomes public without explicit admin approval.

---

# 02-database-design.md

## Strategy

Hybrid model:

* shared `profiles` collection
* role-specific collections

## Key Collections

### users

Stores authentication and identity.

### profiles

Core searchable entity. Includes:

* role
* visibility
* verificationTier
* status
* location

### role-specific collections

* fighterDetails
* gymDetails
* coachDetails
* officialDetails
* promoterDetails
* matchmakerDetails

### supporting collections

* mediaAssets
* verificationDocuments
* contactRequests
* favorites
* reports
* notifications
* categories
* auditLogs
* profileAssociations

## Critical Rules

* never expose verificationDocuments publicly
* searchable = approved + public
* relationships must not rely only on arrays

---

# 03-api-spec.md

## API Principles

* RESTful
* stateless
* consistent responses

## Response Format

Success:

```json
{ "success": true, "data": {} }
```

Error:

```json
{ "success": false, "message": "error" }
```

## Core Routes

### Auth

* POST /auth/register
* POST /auth/login

### Profiles

* POST /profiles
* PATCH /profiles/:id
* POST /profiles/:id/submit

### Directory

* GET /directories

### Admin

* POST /admin/profiles/:id/approve
* POST /admin/profiles/:id/reject

## Rules

* all writes require auth
* admin routes require admin role
* public routes must filter by visibility

---

# 04-auth-and-access-control.md

## Roles

* public
* authenticated user
* admin
* super_admin

## Access Matrix

| Action                 | Public | User | Admin |
| ---------------------- | ------ | ---- | ----- |
| View public profiles   | ✓      | ✓    | ✓     |
| Edit profile           | ✗      | own  | ✓     |
| Approve profiles       | ✗      | ✗    | ✓     |
| View verification docs | ✗      | ✗    | ✓     |

## Rules

* ownership must be validated on every write
* role must be checked on protected routes

---

# 05-profile-workflows.md

## States

* draft
* submitted
* approved
* rejected

## Transitions

* draft → submitted (user)
* submitted → approved (admin)
* submitted → rejected (admin)
* rejected → draft (user edits)

## Constraints

* cannot edit approved profile without resubmission
* rejected profiles must include reason

---

# 06-verification-and-trust.md

## Verification Tiers

* unverified
* tier_2_verified
* tier_1_managed_verified

## Flow

1. user uploads documents
2. admin reviews
3. admin assigns tier

## Rules

* documents are private
* tier is public
* only admin assigns tier

---

# 07-moderation.md

## Inputs

* user reports

## Process

1. report created
2. admin reviews
3. decision made

## Outcomes

* dismiss
* hide content
* suspend user

## Rule

Every moderation action must create an audit log.

---

# 08-directory-and-search.md

## Filters

* role
* region
* city
* verification tier
* weight class

## Rules

* only return approved + public + searchable
* sorting must prioritize verified profiles

## Future

* full-text search
* ranking relevance

---

# 09-notifications.md

## Types

* profile submitted
* profile approved
* profile rejected
* contact request received

## Delivery

* in-app (required)
* email (optional async)

## Rule

Notification creation must not block main request.

---

# 10-media-handling.md

## Storage

* Cloudinary for files
* MongoDB for metadata

## Types

* profile image
* cover image
* gallery
* verification docs (private)

## Rules

* never expose private documents
* store only URLs in DB

---

# 11-testing-strategy.md

## Unit Tests

* validation
* business logic
* state transitions

## End-to-End Tests

* auth flows
* profile lifecycle
* admin workflows
* permissions

## Rule

Tests must validate docs, not assumptions.

---

# 12-deployment.md

## V1

* single backend service
* MongoDB
* Cloudinary

## Environments

* dev
* staging
* production

## Future

* extract services
* add queue

---

# 13-security.md

## Core Controls

* RBAC
* validation
* encryption in transit

## Sensitive Data

* passwords
* verification docs
* audit logs

## Rules

* never expose private data
* enforce least privilege

---

# 14-coding-guidelines.md

## Principles

* minimal dependencies
* modular design
* clear naming

## Rust Direction

* avoid heavy frameworks
* prefer simple crates

## Rules

* no business logic in transport layer
* keep modules isolated

---

# 15-review-checklist.md

## Must Check

* docs alignment
* access control
* workflow correctness
* test coverage

## Reject if

* undocumented behavior
* missing authorization
* broken workflow
* unnecessary complexity

---

# FINAL PRINCIPLE

> The system is documentation-driven. Code must follow docs, not the other way around.
