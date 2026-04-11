use chrono::Utc;
use mongodb::bson::oid::ObjectId;
use punchcraft::profiles::models::{
    Profile, ProfileResponse, ProfileStatus, ProfileVisibility, VerificationTier,
};

fn make_profile(status: ProfileStatus, visibility: ProfileVisibility) -> Profile {
    Profile {
        id: Some(ObjectId::new()),
        user_id: ObjectId::new(),
        role: "fighter".to_string(),
        display_name: "Test Fighter".to_string(),
        bio: None,
        location: None,
        status,
        visibility,
        verification_tier: VerificationTier::Unverified,
        searchable: false,
        rejection_reason: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ── Defaults ──────────────────────────────────────────────────────────────────

#[test]
fn profile_status_default_is_draft() {
    assert_eq!(ProfileStatus::default(), ProfileStatus::Draft);
}

#[test]
fn profile_visibility_default_is_private() {
    assert_eq!(ProfileVisibility::default(), ProfileVisibility::Private);
}

#[test]
fn verification_tier_default_is_unverified() {
    assert_eq!(VerificationTier::default(), VerificationTier::Unverified);
}

// ── ProfileResponse conversion ────────────────────────────────────────────────

#[test]
fn profile_response_maps_fields_correctly() {
    let profile = make_profile(ProfileStatus::Draft, ProfileVisibility::Private);
    let id = profile.id.unwrap().to_hex();
    let user_id = profile.user_id.to_hex();

    let response = ProfileResponse::from(profile);

    assert_eq!(response.id, id);
    assert_eq!(response.user_id, user_id);
    assert_eq!(response.role, "fighter");
    assert_eq!(response.display_name, "Test Fighter");
    assert_eq!(response.status, ProfileStatus::Draft);
    assert_eq!(response.visibility, ProfileVisibility::Private);
    assert_eq!(response.verification_tier, VerificationTier::Unverified);
}

#[test]
fn new_profile_is_not_searchable() {
    let profile = make_profile(ProfileStatus::Draft, ProfileVisibility::Private);
    assert!(!profile.searchable);
}

#[test]
fn new_profile_has_no_rejection_reason() {
    let profile = make_profile(ProfileStatus::Draft, ProfileVisibility::Private);
    assert!(profile.rejection_reason.is_none());
}
