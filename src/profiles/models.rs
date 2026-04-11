use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Profile status — mirrors the workflow in 05-profile-workflows.md
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    Draft,
    Submitted,
    Approved,
    Rejected,
}

/// Verification tier — mirrors 06-verification-and-trust.md
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationTier {
    Unverified,
    Tier2Verified,
    Tier1ManagedVerified,
}

impl Default for VerificationTier {
    fn default() -> Self { Self::Unverified }
}

impl Default for ProfileStatus {
    fn default() -> Self { Self::Draft }
}

/// Core shared profile document stored in the `profiles` collection.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub role: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub location: Option<Location>,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub searchable: bool,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileVisibility {
    Public,
    Private,
}

impl Default for ProfileVisibility {
    fn default() -> Self { Self::Private }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Location {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}

/// Request body for POST /profiles
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProfileRequest {
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,
    pub role: String,
    pub bio: Option<String>,
    pub location: Option<Location>,
}

/// Request body for PATCH /profiles/:id
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(min = 1, max = 100))]
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub location: Option<Location>,
    pub visibility: Option<ProfileVisibility>,
}

/// Public-facing profile response (strips internal fields)
#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub location: Option<Location>,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
}

impl From<Profile> for ProfileResponse {
    fn from(p: Profile) -> Self {
        Self {
            id: p.id.map(|o| o.to_hex()).unwrap_or_default(),
            user_id: p.user_id.to_hex(),
            role: p.role,
            display_name: p.display_name,
            bio: p.bio,
            location: p.location,
            status: p.status,
            visibility: p.visibility,
            verification_tier: p.verification_tier,
        }
    }
}
