Here is your **converted Markdown (`.md`) version** of the uploaded document. I’ve cleaned formatting, added proper headings, code blocks, and structure for your `/docs` usage.

---

# PunchCraft API Documentation

**Version:** v1
**Base URL:** `/api/v1`
**Format:** JSON
**Authentication:** Bearer Token (JWT)
**Source:** PunchCraft project brief and scope 

---

# 1. Overview

PunchCraft is a boxing ecosystem platform for profiling, verifying, and connecting stakeholders including:

* fighters
* gyms
* coaches
* referees/judges
* promoters
* matchmakers
* fans

The platform emphasizes:

* credibility through admin approvals
* verification tiers
* controlled visibility

## This API supports:

* Authentication and account management
* Role-based profile creation and management
* Admin approval and moderation workflows
* Verification tiers
* Public/private profile visibility
* Directory search and filters
* Contact requests and reporting
* Future expansion (matchmaking, rankings, analytics, monetization)

---

# 2. Roles

Supported roles:

* super_admin
* admin
* fighter
* gym
* coach
* referee_judge
* promoter
* matchmaker
* fan

---

# 3. Authentication

## 3.1 Sign Up

`POST /auth/register`

### Request

```json
{
  "fullName": "Kwame Mensah",
  "email": "kwame@example.com",
  "password": "StrongPass123!",
  "role": "fighter"
}
```

### Response

```json
{
  "success": true,
  "message": "Account created successfully",
  "data": {
    "userId": "usr_12345",
    "email": "kwame@example.com",
    "role": "fighter",
    "accountStatus": "active"
  }
}
```

---

## 3.2 Login

`POST /auth/login`

### Request

```json
{
  "email": "kwame@example.com",
  "password": "StrongPass123!"
}
```

### Response

```json
{
  "success": true,
  "message": "Login successful",
  "data": {
    "accessToken": "jwt_access_token",
    "refreshToken": "jwt_refresh_token",
    "user": {
      "id": "usr_12345",
      "fullName": "Kwame Mensah",
      "role": "fighter"
    }
  }
}
```

---

## 3.3 Refresh Token

`POST /auth/refresh`

```json
{
  "refreshToken": "jwt_refresh_token"
}
```

---

## 3.4 Forgot Password

`POST /auth/forgot-password`

```json
{
  "email": "kwame@example.com"
}
```

---

## 3.5 Reset Password

`POST /auth/reset-password`

```json
{
  "token": "reset_token",
  "newPassword": "NewStrongPass123!"
}
```

---

## 3.6 Change Password

`PATCH /auth/change-password` *(Auth required)*

```json
{
  "currentPassword": "OldPass123!",
  "newPassword": "NewPass123!"
}
```

---

# 4. User Account Management

## 4.1 Get Current User

`GET /users/me`

```json
{
  "success": true,
  "data": {
    "id": "usr_12345",
    "fullName": "Kwame Mensah",
    "email": "kwame@example.com",
    "role": "fighter",
    "accountStatus": "active"
  }
}
```

---

## 4.2 Update Account

`PATCH /users/me`

```json
{
  "phone": "+233500000000",
  "profilePhoto": "https://cdn.punchcraft.com/photos/user.jpg",
  "socialLinks": {
    "instagram": "https://instagram.com/kwameboxing",
    "youtube": "https://youtube.com/@kwameboxing"
  }
}
```

---

# 5. Common Profile Model

```json
{
  "id": "prf_001",
  "userId": "usr_12345",
  "role": "fighter",
  "status": "submitted",
  "visibility": "private",
  "verificationTier": "unverified",
  "profileImage": "...",
  "coverImage": "...",
  "bio": "Professional boxer from Accra.",
  "location": {
    "country": "Ghana",
    "region": "Greater Accra",
    "city": "Accra"
  },
  "contactDetails": {},
  "socialLinks": {},
  "mediaGallery": [],
  "createdAt": "ISO",
  "updatedAt": "ISO"
}
```

---

# 6. Status & Visibility

## Status

* draft
* submitted
* approved
* rejected

**Profile lifecycle:** draft → submitted → approved | rejected → draft (via request-revision)

An owner can call `POST /{type}/{id}/request-revision` on any **approved** profile to revert it to draft for editing and resubmission. This sets status=draft, visibility=private, searchable=false.

## Visibility

* private
* public

## Verification

* unverified
* tier2_verified
* tier1_managed_verified

Note: no underscore before the digit — these are the exact string values used in the database and API.

---

# 7. Fighter Endpoints

* `POST /profiles/fighters` — create (linkedGymId and linkedCoachId are optional)
* `GET/PATCH /profiles/fighters/{id}`
* `POST /profiles/fighters/{id}/submit`
* `POST /profiles/fighters/{id}/request-revision` — pull approved profile back to draft
* `POST /profiles/fighters/{id}/profile-image` — multipart, field name `file`
* `POST /profiles/fighters/{id}/cover-image` — multipart, field name `file`
* `POST /profiles/fighters/{id}/fight-history`
* `DELETE /profiles/fighters/{id}/fight-history/{fightId}`

## Add Fight History

```json
{
  "opponentName": "Kofi Asare",
  "eventName": "Accra Fight Night",
  "eventDate": "2026-03-15",
  "result": "win",
  "method": "KO",
  "round": 4
}
```

---

# 8. Gym Endpoints

* `POST /profiles/gyms`
* `GET/PATCH /profiles/gyms/{id}`
* `POST /profiles/gyms/{id}/submit`
* `POST /profiles/gyms/{id}/request-revision`
* `POST /profiles/gyms/{id}/profile-image`
* `POST /profiles/gyms/{id}/cover-image`
* `POST /profiles/gyms/{gymId}/coaches/{coachId}` — link coach (bidirectional)
* `DELETE /profiles/gyms/{gymId}/coaches/{coachId}` — unlink coach
* `POST /profiles/gyms/{gymId}/fighters/{fighterId}` — add fighter to roster
* `DELETE /profiles/gyms/{gymId}/fighters/{fighterId}` — remove fighter from roster

**Gym–fighter linking rules:**
- A gym can only link a fighter who has no current gym. Returns 409 if fighter already belongs to another gym.
- A fighter changes gyms by updating their own profile (`PATCH /profiles/fighters/{id}` with new `linkedGymId`). This auto-removes them from the old gym's roster.
- `linkedGymId` and `linkedCoachId` are optional on fighter creation (free agents supported).

---

# 9. Coach Endpoints

* `POST /profiles/coaches`
* `POST /profiles/coaches/{id}/certifications`

---

# 10. Official Endpoints

* `POST /profiles/officials`
* `POST /profiles/officials/{id}/credentials`

---

# 11. Promoter Endpoints

* `POST /profiles/promoters`

---

# 12. Matchmaker Endpoints

* `POST /profiles/matchmakers`

---

# 13. Directory & Search

`GET /directories`

### Filters:

* role
* keyword
* region
* city
* verificationTier
* weightClass

---

# 14. Contact Requests

* `POST /contact-requests`
* `GET /contact-requests`
* `PATCH /contact-requests/{id}`

Statuses:

* pending
* accepted
* declined

---

# 15. Favorites

* `POST /favorites`
* `DELETE /favorites/{id}`
* `GET /favorites`

---

# 16. Reports

`POST /reports`

---

# 17. Admin Endpoints

* approval queue
* approve/reject profile
* assign verification
* suspend user
* review reports
* audit logs

---

# 18. Metadata

* weight classes
* regions
* verification tiers

---

# 19. Uploads

* `POST /uploads/media`
* `POST /uploads/documents/verification`

---

# 20. Notifications

* `GET /notifications`
* `PATCH /notifications/{id}/read`

---

# 21. Error Format

```json
{
  "success": false,
  "message": "Validation failed",
  "errors": []
}
```

---

# 22. Access Control

## Public

* view only approved public profiles

## Users

* manage own profiles

## Admin

* full control

---

# 23. V1 vs Future

## V1

* auth
* profiles
* directories
* admin workflows

## Future

* matchmaking
* rankings
* analytics
* monetization

---

# 24. Naming Conventions

* plural endpoints
* kebab-case URLs
* camelCase JSON

---

# 25. Example Flow

1. register
2. create profile
3. draft
4. submit
5. admin review
6. approve/reject
7. becomes public

---