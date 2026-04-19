use chrono::Utc;
use mongodb::bson::oid::ObjectId;
use punchcraft::profiles::models::{
    Profile, ProfileStatus, ProfileSummary, ProfileVisibility, VerificationTier,
};

fn make_profile(status: ProfileStatus, visibility: ProfileVisibility) -> Profile {
    Profile {
        id: Some(ObjectId::new()),
        user_id: ObjectId::new(),
        role: "fighter".to_string(),
        display_name: "Test Fighter".to_string(),
        bio: None,
        profile_image: None,
        cover_image: None,
        location: None,
        contact_details: None,
        social_links: None,
        status,
        visibility,
        verification_tier: VerificationTier::Unverified,
        searchable: false,
        has_verified_document: false,
        weight_class: None,
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

// ── ProfileSummary conversion ─────────────────────────────────────────────────

#[test]
fn profile_summary_maps_fields_correctly() {
    let profile = make_profile(ProfileStatus::Draft, ProfileVisibility::Private);
    let id = profile.id.unwrap().to_hex();
    let user_id = profile.user_id.to_hex();

    let summary = ProfileSummary::from(profile);

    assert_eq!(summary.id, id);
    assert_eq!(summary.user_id, user_id);
    assert_eq!(summary.role, "fighter");
    assert_eq!(summary.display_name, "Test Fighter");
    assert_eq!(summary.status, ProfileStatus::Draft);
    assert_eq!(summary.visibility, ProfileVisibility::Private);
    assert_eq!(summary.verification_tier, VerificationTier::Unverified);
    assert!(!summary.searchable);
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

#[test]
fn approved_public_profile_summary_reflects_correct_state() {
    let profile = make_profile(ProfileStatus::Approved, ProfileVisibility::Public);
    let summary = ProfileSummary::from(profile);
    assert_eq!(summary.status, ProfileStatus::Approved);
    assert_eq!(summary.visibility, ProfileVisibility::Public);
}
