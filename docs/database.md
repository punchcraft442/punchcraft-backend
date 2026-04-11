# PunchCraft MongoDB Database Design

## System Architecture Perspective

This database design follows a **hybrid document modeling strategy** optimized for:

* V1 simplicity and fast development
* Strong admin-controlled trust workflows
* Efficient public directory search
* Clean separation of concerns
* Future scalability (matchmaking, rankings, monetization)

---

# 1. Core Design Philosophy

## Hybrid Model

Instead of one large collection or fully fragmented collections:

* `users` → authentication & identity
* `profiles` → shared searchable layer
* Role-specific collections → detailed domain data

### Why this works

* Enables **fast directory queries**
* Keeps schemas **clean and maintainable**
* Supports **admin workflows easily**
* Prevents schema bloat

---

# 2. Collections Overview

## Core Collections (V1)

* users
* profiles
* fighterDetails
* gymDetails
* coachDetails
* officialDetails
* promoterDetails
* matchmakerDetails
* mediaAssets
* verificationDocuments
* contactRequests
* favorites
* reports
* notifications
* categories
* auditLogs
* profileAssociations (important for relationships)

---

# 3. Enums

```js
role = [
  "super_admin",
  "admin",
  "fighter",
  "gym",
  "coach",
  "referee_judge",
  "promoter",
  "matchmaker",
  "fan"
]

profileStatus = ["draft", "submitted", "approved", "rejected"]
visibility = ["private", "public"]
verificationTier = ["unverified", "tier_2_verified", "tier_1_managed_verified"]
accountStatus = ["active", "suspended", "disabled", "deleted"]
```

---

# 4. Users Collection

```js
{
  _id: ObjectId,
  fullName: String,
  email: String,
  passwordHash: String,
  primaryRole: String,
  accountStatus: String,

  phone: String,
  profilePhotoUrl: String,

  socialLinks: {
    instagram: String,
    tiktok: String,
    youtube: String
  },

  emailVerified: Boolean,
  createdAt: Date,
  updatedAt: Date
}
```

---

# 5. Profiles Collection (CORE)

This is the **most important collection**.

```js
{
  _id: ObjectId,
  userId: ObjectId,
  role: "fighter",

  displayName: String,
  slug: String,
  bio: String,

  profileImageUrl: String,
  coverImageUrl: String,

  location: {
    country: String,
    region: String,
    city: String
  },

  contactDetails: {
    email: String,
    phone: String,
    showEmailPublicly: Boolean
  },

  socialLinks: {},

  status: "draft",
  visibility: "private",
  verificationTier: "unverified",

  searchable: Boolean,

  search: {
    weightClass: String,
    verifiedRank: Number
  },

  createdAt: Date,
  updatedAt: Date
}
```

### Key Role

* Drives **directory search**
* Controls **visibility + verification**
* Powers **admin approval workflow**

---

# 6. Fighter Details

```js
{
  profileId: ObjectId,

  fullName: String,
  ringName: String,
  nationality: String,

  weightClass: String,
  stance: String,

  heightCm: Number,
  reachCm: Number,

  record: {
    wins: Number,
    losses: Number,
    draws: Number,
    kos: Number
  },

  titles: [String],

  linkedGymProfileId: ObjectId,
  linkedCoachProfileId: ObjectId,

  fightHistory: [
    {
      opponentName: String,
      result: String,
      eventDate: Date
    }
  ]
}
```

---

# 7. Gym Details

```js
{
  profileId: ObjectId,

  gymName: String,
  address: String,

  services: [String],
  facilities: [String],

  linkedCoachProfileIds: [ObjectId],
  rosterFighterProfileIds: [ObjectId]
}
```

---

# 8. Coach Details

```js
{
  profileId: ObjectId,

  fullName: String,
  specialties: [String],

  linkedGymProfileIds: [ObjectId]
}
```

---

# 9. Officials (Referee/Judge)

```js
{
  profileId: ObjectId,

  officialType: ["referee", "judge"],
  experienceYears: Number,
  coverageArea: [String]
}
```

---

# 10. Media Assets

```js
{
  profileId: ObjectId,
  url: String,
  type: "image",
  category: "gallery",
  moderationStatus: "visible"
}
```

---

# 11. Verification Documents

```js
{
  profileId: ObjectId,
  fileUrl: String,
  documentType: String,
  reviewStatus: "pending"
}
```

---

# 12. Contact Requests

```js
{
  senderUserId: ObjectId,
  recipientProfileId: ObjectId,
  message: String,
  status: "pending",
  createdAt: Date
}
```

---

# 13. Favorites

```js
{
  userId: ObjectId,
  profileId: ObjectId
}
```

---

# 14. Reports

```js
{
  reporterUserId: ObjectId,
  profileId: ObjectId,
  reason: String,
  status: "open"
}
```

---

# 15. Notifications

```js
{
  userId: ObjectId,
  title: String,
  message: String,
  isRead: Boolean
}
```

---

# 16. Profile Associations (IMPORTANT)

Handles relationships like:

* fighter ↔ gym
* coach ↔ gym

```js
{
  fromProfileId: ObjectId,
  toProfileId: ObjectId,
  type: "member_of",
  status: "pending"
}
```

---

# 17. Query Strategy

## Directory Search

Query only `profiles`

Filters:

* role
* region
* verificationTier
* weightClass

---

# 18. Approval Workflow

Flow:

Draft → Submitted → Approved / Rejected

```js
status
submittedAt
approvedAt
rejectedAt
```

---

# 19. Indexing Strategy

### Critical indexes

```js
users.email (unique)
profiles.slug (unique)
profiles.role + searchable
profiles.location.region
profiles.search.weightClass
```

---

# 20. Key Architectural Decisions

## Why this design

* Clean separation of domain logic
* Fast directory queries
* Strong admin control
* Scalable to millions of users

## Biggest mistake to avoid

❌ One giant profile collection with all fields
❌ Storing relationships only in arrays

---

# 21. Future Expansion Ready

Supports:

* Matchmaking
* Event system
* Rankings
* Sponsorship marketplace
* Subscriptions
* Analytics

---

# Final Summary

**Core principle:**

> Shared profile layer + role-specific collections + controlled denormalization

This gives PunchCraft:

* performance
* flexibility
* maintainability
* scalability\