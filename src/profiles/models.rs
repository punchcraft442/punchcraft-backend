use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    Draft,
    Submitted,
    Approved,
    Rejected,
}

impl Default for ProfileStatus {
    fn default() -> Self { Self::Draft }
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FighterStance {
    Orthodox,
    Southpaw,
    Switch,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FightResult {
    Win,
    Loss,
    Draw,
    NoContest,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OfficialType {
    Referee,
    Judge,
}

// ── Shared sub-structs ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SocialLinks {
    pub instagram: Option<String>,
    pub tiktok: Option<String>,
    pub youtube: Option<String>,
    pub facebook: Option<String>,
    pub x: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContactDetails {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub show_email_publicly: bool,
    pub show_phone_publicly: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FighterRecord {
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    pub kos: u32,
}

/// Embedded in fighter_details.fight_history array.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FightHistoryEntry {
    pub id: String, // UUID string
    pub opponent_name: String,
    pub event_name: String,
    pub event_date: String, // YYYY-MM-DD
    pub result: FightResult,
    pub method: Option<String>,
    pub round: Option<u32>,
}

// ── Base profile document — `profiles` collection ─────────────────────────────

/// Shared across all roles. Drives directory search and admin workflow.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub role: String,
    /// Denormalised display name — maps to fullName/name/displayName from role details.
    pub display_name: String,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    /// True only when status=approved AND visibility=public.
    pub searchable: bool,
    /// Set to true when at least one verification document for this profile is approved.
    /// A profile is only publicly visible when both searchable=true and has_verified_document=true.
    #[serde(default)]
    pub has_verified_document: bool,
    /// Denormalised from fighter weight class — enables directory filtering without a join.
    pub weight_class: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Role-specific detail documents ────────────────────────────────────────────

/// `fighterDetails` collection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FighterDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub full_name: String,
    pub ring_name: Option<String>,
    pub nationality: Option<String>,
    pub weight_class: Option<String>,
    pub stance: Option<FighterStance>,
    pub height_cm: Option<i32>,
    pub reach_cm: Option<i32>,
    pub record: Option<FighterRecord>,
    pub titles: Vec<String>,
    pub linked_gym_id: Option<String>,
    pub linked_coach_id: Option<String>,
    pub fight_history: Vec<FightHistoryEntry>,
}

/// `gymDetails` collection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GymDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub name: String,
    pub address: Option<String>,
    pub services: Vec<String>,
    pub facilities: Vec<String>,
    pub linked_coach_ids: Vec<String>,
    pub roster_fighter_ids: Vec<String>,
}

/// Embedded in coachDetails.certifications and officialDetails.credentials arrays.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEntry {
    pub id: String,
    pub label: Option<String>,
    pub file_url: String,
    pub uploaded_at: String,
}

/// `coachDetails` collection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoachDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub full_name: String,
    pub experience_summary: Option<String>,
    pub specialties: Vec<String>,
    pub linked_gym_ids: Vec<String>,
    pub associated_fighter_ids: Vec<String>,
    #[serde(default)]
    pub certifications: Vec<DocumentEntry>,
}

/// `officialDetails` collection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OfficialDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub full_name: String,
    pub official_type: Vec<OfficialType>,
    pub experience_years: Option<i32>,
    pub events_worked: Vec<String>,
    pub licensing_details: Option<String>,
    pub coverage_area: Vec<String>,
    pub availability: Option<String>,
    #[serde(default)]
    pub credentials: Vec<DocumentEntry>,
}

/// `promoterDetails` collection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PromoterDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub organization_name: String,
    pub coverage_areas: Vec<String>,
    pub past_events: Vec<String>,
    pub references: Vec<String>,
}

/// `matchmakerDetails` collection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchmakerDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub full_name: String,
    pub regions_served: Vec<String>,
    pub weight_classes_focus: Vec<String>,
    pub experience_summary: Option<String>,
    pub past_matchups: Vec<String>,
}

/// `fanDetails` collection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FanDetails {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub display_name: String,
    pub favourite_weight_class: Option<String>,
}

// ── Pagination ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub keyword: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    #[serde(rename = "verificationTier")]
    pub verification_tier: Option<String>,
    #[serde(rename = "weightClass")]
    pub weight_class: Option<String>,
    pub sort: Option<String>,
}

impl PaginationParams {
    pub fn page(&self) -> u32 { self.page.unwrap_or(1).max(1) }
    pub fn limit(&self) -> u32 { self.limit.unwrap_or(20).min(100) }
    pub fn skip(&self) -> u64 { ((self.page() - 1) * self.limit()) as u64 }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub page: u32,
    pub limit: u32,
    pub total_items: u64,
    pub total_pages: u64,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub pagination: PaginationMeta,
}

// ── Request structs ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateFighterRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: String,
    pub weight_class: String,
    pub ring_name: Option<String>,
    pub nationality: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub stance: Option<FighterStance>,
    pub height_cm: Option<i32>,
    pub reach_cm: Option<i32>,
    pub record: Option<FighterRecord>,
    pub titles: Option<Vec<String>>,
    pub linked_gym_id: Option<String>,
    pub linked_coach_id: Option<String>,
    pub bio: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFighterRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: Option<String>,
    pub weight_class: Option<String>,
    pub ring_name: Option<String>,
    pub nationality: Option<String>,
    pub stance: Option<FighterStance>,
    pub height_cm: Option<i32>,
    pub reach_cm: Option<i32>,
    pub record: Option<FighterRecord>,
    pub titles: Option<Vec<String>>,
    pub linked_gym_id: Option<String>,
    pub linked_coach_id: Option<String>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub bio: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub location: Option<Option<Location>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub contact_details: Option<Option<ContactDetails>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub social_links: Option<Option<SocialLinks>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateFightHistoryRequest {
    #[validate(length(min = 1))]
    pub opponent_name: String,
    #[validate(length(min = 1))]
    pub event_name: String,
    pub event_date: String,
    pub result: FightResult,
    pub method: Option<String>,
    pub round: Option<u32>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateGymRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub address: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub services: Option<Vec<String>>,
    pub facilities: Option<Vec<String>>,
    pub bio: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGymRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    pub address: Option<String>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub location: Option<Option<Location>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub contact_details: Option<Option<ContactDetails>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub social_links: Option<Option<SocialLinks>>,
    pub services: Option<Vec<String>>,
    pub facilities: Option<Vec<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub bio: Option<Option<String>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCoachRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: String,
    pub bio: Option<String>,
    pub experience_summary: Option<String>,
    pub specialties: Option<Vec<String>>,
    pub linked_gym_ids: Option<Vec<String>>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCoachRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: Option<String>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub bio: Option<Option<String>>,
    pub experience_summary: Option<String>,
    pub specialties: Option<Vec<String>>,
    pub linked_gym_ids: Option<Vec<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub location: Option<Option<Location>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub contact_details: Option<Option<ContactDetails>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub social_links: Option<Option<SocialLinks>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateOfficialRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: String,
    pub official_type: Vec<OfficialType>,
    pub experience_years: Option<i32>,
    pub events_worked: Option<Vec<String>>,
    pub licensing_details: Option<String>,
    pub coverage_area: Option<Vec<String>>,
    pub availability: Option<String>,
    pub bio: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOfficialRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: Option<String>,
    pub official_type: Option<Vec<OfficialType>>,
    pub experience_years: Option<i32>,
    pub events_worked: Option<Vec<String>>,
    pub licensing_details: Option<String>,
    pub coverage_area: Option<Vec<String>>,
    pub availability: Option<String>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub bio: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub location: Option<Option<Location>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub contact_details: Option<Option<ContactDetails>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub social_links: Option<Option<SocialLinks>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePromoterRequest {
    #[validate(length(min = 1, max = 100))]
    pub organization_name: String,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub coverage_areas: Option<Vec<String>>,
    pub past_events: Option<Vec<String>>,
    pub references: Option<Vec<String>>,
    pub bio: Option<String>,
    pub location: Option<Location>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePromoterRequest {
    #[validate(length(min = 1, max = 100))]
    pub organization_name: Option<String>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub contact_details: Option<Option<ContactDetails>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub social_links: Option<Option<SocialLinks>>,
    pub coverage_areas: Option<Vec<String>>,
    pub past_events: Option<Vec<String>>,
    pub references: Option<Vec<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub bio: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub location: Option<Option<Location>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateMatchmakerRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: String,
    pub regions_served: Option<Vec<String>>,
    pub weight_classes_focus: Option<Vec<String>>,
    pub experience_summary: Option<String>,
    pub past_matchups: Option<Vec<String>>,
    pub bio: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMatchmakerRequest {
    #[validate(length(min = 1, max = 100))]
    pub full_name: Option<String>,
    pub regions_served: Option<Vec<String>>,
    pub weight_classes_focus: Option<Vec<String>>,
    pub experience_summary: Option<String>,
    pub past_matchups: Option<Vec<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub bio: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub location: Option<Option<Location>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub contact_details: Option<Option<ContactDetails>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub social_links: Option<Option<SocialLinks>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateFanRequest {
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,
    pub bio: Option<String>,
    pub location: Option<Location>,
    pub social_links: Option<SocialLinks>,
    pub favourite_weight_class: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFanRequest {
    #[validate(length(min = 1, max = 100))]
    pub display_name: Option<String>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub bio: Option<Option<String>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub location: Option<Option<Location>>,
    #[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]
    pub social_links: Option<Option<SocialLinks>>,
    pub favourite_weight_class: Option<String>,
}

// ── Shared summary response (used by admin + directory listing) ───────────────

/// Lightweight profile view — base fields only, no role-specific details.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub location: Option<Location>,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub searchable: bool,
    pub weight_class: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Profile> for ProfileSummary {
    fn from(p: Profile) -> Self {
        Self {
            id: p.id.map(|o| o.to_hex()).unwrap_or_default(),
            user_id: p.user_id.to_hex(),
            role: p.role,
            display_name: p.display_name,
            bio: p.bio,
            profile_image: p.profile_image,
            location: p.location,
            status: p.status,
            visibility: p.visibility,
            verification_tier: p.verification_tier,
            searchable: p.searchable,
            weight_class: p.weight_class,
            rejection_reason: p.rejection_reason,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

// ── Response structs ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FighterProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub full_name: String,
    pub ring_name: Option<String>,
    pub nationality: Option<String>,
    pub weight_class: Option<String>,
    pub stance: Option<FighterStance>,
    pub height_cm: Option<i32>,
    pub reach_cm: Option<i32>,
    pub record: Option<FighterRecord>,
    pub titles: Vec<String>,
    pub linked_gym_id: Option<String>,
    pub linked_coach_id: Option<String>,
    pub fight_history: Vec<FightHistoryEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GymProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub address: Option<String>,
    pub services: Vec<String>,
    pub facilities: Vec<String>,
    pub linked_coach_ids: Vec<String>,
    pub roster_fighter_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoachProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub full_name: String,
    pub experience_summary: Option<String>,
    pub specialties: Vec<String>,
    pub linked_gym_ids: Vec<String>,
    pub associated_fighter_ids: Vec<String>,
    pub certifications: Vec<DocumentEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub full_name: String,
    pub official_type: Vec<OfficialType>,
    pub experience_years: Option<i32>,
    pub events_worked: Vec<String>,
    pub licensing_details: Option<String>,
    pub coverage_area: Vec<String>,
    pub availability: Option<String>,
    pub credentials: Vec<DocumentEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoterProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub organization_name: String,
    pub coverage_areas: Vec<String>,
    pub past_events: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchmakerProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub full_name: String,
    pub regions_served: Vec<String>,
    pub weight_classes_focus: Vec<String>,
    pub experience_summary: Option<String>,
    pub past_matchups: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FanProfileResponse {
    pub id: String,
    pub user_id: String,
    pub role: String,
    pub status: ProfileStatus,
    pub visibility: ProfileVisibility,
    pub verification_tier: VerificationTier,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
    pub cover_image: Option<String>,
    pub location: Option<Location>,
    pub contact_details: Option<ContactDetails>,
    pub social_links: Option<SocialLinks>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub display_name: String,
    pub favourite_weight_class: Option<String>,
}

// ── Builder helpers ───────────────────────────────────────────────────────────

pub fn base_fields_from(p: &Profile) -> (
    String, String, String, ProfileStatus, ProfileVisibility, VerificationTier,
    Option<String>, Option<String>, Option<String>,
    Option<Location>, Option<ContactDetails>, Option<SocialLinks>,
    Option<String>, DateTime<Utc>, DateTime<Utc>,
) {
    (
        p.id.map(|o| o.to_hex()).unwrap_or_default(),
        p.user_id.to_hex(),
        p.role.clone(),
        p.status.clone(),
        p.visibility.clone(),
        p.verification_tier.clone(),
        p.bio.clone(),
        p.profile_image.clone(),
        p.cover_image.clone(),
        p.location.clone(),
        p.contact_details.clone(),
        p.social_links.clone(),
        p.rejection_reason.clone(),
        p.created_at,
        p.updated_at,
    )
}

impl FighterProfileResponse {
    pub fn from_parts(p: Profile, d: FighterDetails) -> Self {
        let (id, user_id, role, status, visibility, verification_tier,
             bio, profile_image, cover_image, location, contact_details,
             social_links, rejection_reason, created_at, updated_at) = base_fields_from(&p);
        Self {
            id, user_id, role, status, visibility, verification_tier,
            bio, profile_image, cover_image, location, contact_details,
            social_links, rejection_reason, created_at, updated_at,
            full_name: d.full_name,
            ring_name: d.ring_name,
            nationality: d.nationality,
            weight_class: d.weight_class,
            stance: d.stance,
            height_cm: d.height_cm,
            reach_cm: d.reach_cm,
            record: d.record,
            titles: d.titles,
            linked_gym_id: d.linked_gym_id,
            linked_coach_id: d.linked_coach_id,
            fight_history: d.fight_history,
        }
    }
}

impl GymProfileResponse {
    pub fn from_parts(p: Profile, d: GymDetails) -> Self {
        let (id, user_id, role, status, visibility, verification_tier,
             bio, profile_image, cover_image, location, contact_details,
             social_links, rejection_reason, created_at, updated_at) = base_fields_from(&p);
        Self {
            id, user_id, role, status, visibility, verification_tier,
            bio, profile_image, cover_image, location, contact_details,
            social_links, rejection_reason, created_at, updated_at,
            name: d.name,
            address: d.address,
            services: d.services,
            facilities: d.facilities,
            linked_coach_ids: d.linked_coach_ids,
            roster_fighter_ids: d.roster_fighter_ids,
        }
    }
}

impl CoachProfileResponse {
    pub fn from_parts(p: Profile, d: CoachDetails) -> Self {
        let (id, user_id, role, status, visibility, verification_tier,
             bio, profile_image, cover_image, location, contact_details,
             social_links, rejection_reason, created_at, updated_at) = base_fields_from(&p);
        Self {
            id, user_id, role, status, visibility, verification_tier,
            bio, profile_image, cover_image, location, contact_details,
            social_links, rejection_reason, created_at, updated_at,
            full_name: d.full_name,
            experience_summary: d.experience_summary,
            specialties: d.specialties,
            linked_gym_ids: d.linked_gym_ids,
            associated_fighter_ids: d.associated_fighter_ids,
            certifications: d.certifications,
        }
    }
}

impl OfficialProfileResponse {
    pub fn from_parts(p: Profile, d: OfficialDetails) -> Self {
        let (id, user_id, role, status, visibility, verification_tier,
             bio, profile_image, cover_image, location, contact_details,
             social_links, rejection_reason, created_at, updated_at) = base_fields_from(&p);
        Self {
            id, user_id, role, status, visibility, verification_tier,
            bio, profile_image, cover_image, location, contact_details,
            social_links, rejection_reason, created_at, updated_at,
            full_name: d.full_name,
            official_type: d.official_type,
            experience_years: d.experience_years,
            events_worked: d.events_worked,
            licensing_details: d.licensing_details,
            coverage_area: d.coverage_area,
            availability: d.availability,
            credentials: d.credentials,
        }
    }
}

impl PromoterProfileResponse {
    pub fn from_parts(p: Profile, d: PromoterDetails) -> Self {
        let (id, user_id, role, status, visibility, verification_tier,
             bio, profile_image, cover_image, location, contact_details,
             social_links, rejection_reason, created_at, updated_at) = base_fields_from(&p);
        Self {
            id, user_id, role, status, visibility, verification_tier,
            bio, profile_image, cover_image, location, contact_details,
            social_links, rejection_reason, created_at, updated_at,
            organization_name: d.organization_name,
            coverage_areas: d.coverage_areas,
            past_events: d.past_events,
            references: d.references,
        }
    }
}

impl MatchmakerProfileResponse {
    pub fn from_parts(p: Profile, d: MatchmakerDetails) -> Self {
        let (id, user_id, role, status, visibility, verification_tier,
             bio, profile_image, cover_image, location, contact_details,
             social_links, rejection_reason, created_at, updated_at) = base_fields_from(&p);
        Self {
            id, user_id, role, status, visibility, verification_tier,
            bio, profile_image, cover_image, location, contact_details,
            social_links, rejection_reason, created_at, updated_at,
            full_name: d.full_name,
            regions_served: d.regions_served,
            weight_classes_focus: d.weight_classes_focus,
            experience_summary: d.experience_summary,
            past_matchups: d.past_matchups,
        }
    }
}

impl FanProfileResponse {
    pub fn from_parts(p: Profile, d: FanDetails) -> Self {
        let (id, user_id, role, status, visibility, verification_tier,
             bio, profile_image, cover_image, location, contact_details,
             social_links, rejection_reason, created_at, updated_at) = base_fields_from(&p);
        Self {
            id, user_id, role, status, visibility, verification_tier,
            bio, profile_image, cover_image, location, contact_details,
            social_links, rejection_reason, created_at, updated_at,
            display_name: d.display_name,
            favourite_weight_class: d.favourite_weight_class,
        }
    }
}
