use chrono::Utc;
use mongodb::{bson::oid::ObjectId, Database};

use crate::common::errors::AppError;
use super::{
    models::{
        CreateProfileRequest, Profile, ProfileResponse, ProfileStatus, ProfileVisibility,
        UpdateProfileRequest, VerificationTier,
    },
    repository,
};

pub async fn create_profile(
    db: &Database,
    user_id: &str,
    req: CreateProfileRequest,
) -> Result<ProfileResponse, AppError> {
    let uid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid user id".into()))?;

    // One profile per user
    if repository::find_by_user_id(db, uid).await?.is_some() {
        return Err(AppError::Conflict("Profile already exists".into()));
    }

    let now = Utc::now();
    let profile = Profile {
        id: None,
        user_id: uid,
        role: req.role,
        display_name: req.display_name,
        bio: req.bio,
        location: req.location,
        status: ProfileStatus::Draft,
        visibility: ProfileVisibility::Private,
        verification_tier: VerificationTier::Unverified,
        searchable: false,
        rejection_reason: None,
        created_at: now,
        updated_at: now,
    };

    let inserted_id = repository::insert(db, &profile).await?;
    let mut saved = profile;
    saved.id = Some(inserted_id);
    Ok(ProfileResponse::from(saved))
}

pub async fn get_profile(db: &Database, profile_id: &str) -> Result<ProfileResponse, AppError> {
    let oid = ObjectId::parse_str(profile_id).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let profile = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileResponse::from(profile))
}

pub async fn update_profile(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdateProfileRequest,
) -> Result<ProfileResponse, AppError> {
    let oid = ObjectId::parse_str(profile_id).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let uid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid user id".into()))?;

    let profile = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;

    // Ownership check
    if profile.user_id != uid {
        return Err(AppError::Forbidden);
    }

    // Cannot edit approved profile without resubmission
    if profile.status == ProfileStatus::Approved {
        return Err(AppError::BadRequest(
            "Approved profile cannot be edited without resubmission".into(),
        ));
    }

    let mut update_doc = mongodb::bson::Document::new();
    if let Some(name) = req.display_name {
        update_doc.insert("display_name", name);
    }
    if let Some(bio) = req.bio {
        update_doc.insert("bio", bio);
    }
    if let Some(vis) = req.visibility {
        let v = serde_json::to_value(&vis).unwrap().as_str().unwrap().to_string();
        update_doc.insert("visibility", v);
    }
    update_doc.insert("updated_at", Utc::now().to_rfc3339());

    repository::update(db, oid, update_doc).await?;

    let updated = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileResponse::from(updated))
}

/// User submits a draft profile for admin review.
pub async fn submit_profile(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<ProfileResponse, AppError> {
    let oid = ObjectId::parse_str(profile_id).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let uid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid user id".into()))?;

    let profile = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;

    if profile.user_id != uid {
        return Err(AppError::Forbidden);
    }

    // Only draft or rejected profiles can be submitted
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }

    let mut update_doc = mongodb::bson::Document::new();
    update_doc.insert("status", "submitted");
    update_doc.insert("rejection_reason", mongodb::bson::Bson::Null);
    update_doc.insert("updated_at", Utc::now().to_rfc3339());

    repository::update(db, oid, update_doc).await?;

    let updated = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileResponse::from(updated))
}
