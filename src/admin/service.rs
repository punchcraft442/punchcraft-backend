use chrono::Utc;
use mongodb::{bson::{doc, oid::ObjectId}, Database};
use serde::Deserialize;

use crate::common::errors::AppError;
use crate::profiles::{models::{ProfileResponse, ProfileStatus}, repository};

#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

pub async fn approve_profile(db: &Database, profile_id: &str) -> Result<ProfileResponse, AppError> {
    let oid = ObjectId::parse_str(profile_id).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let profile = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;

    if profile.status != ProfileStatus::Submitted {
        return Err(AppError::BadRequest("Only submitted profiles can be approved".into()));
    }

    let mut update_doc = mongodb::bson::Document::new();
    update_doc.insert("status", "approved");
    update_doc.insert("visibility", "public");
    update_doc.insert("searchable", true);
    update_doc.insert("rejection_reason", mongodb::bson::Bson::Null);
    update_doc.insert("updated_at", Utc::now().to_rfc3339());

    repository::update(db, oid, update_doc).await?;
    let updated = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileResponse::from(updated))
}

pub async fn reject_profile(
    db: &Database,
    profile_id: &str,
    req: RejectRequest,
) -> Result<ProfileResponse, AppError> {
    let oid = ObjectId::parse_str(profile_id).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let profile = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;

    if profile.status != ProfileStatus::Submitted {
        return Err(AppError::BadRequest("Only submitted profiles can be rejected".into()));
    }

    if req.reason.trim().is_empty() {
        return Err(AppError::BadRequest("Rejection reason is required".into()));
    }

    let mut update_doc = mongodb::bson::Document::new();
    update_doc.insert("status", "rejected");
    update_doc.insert("rejection_reason", req.reason);
    update_doc.insert("updated_at", Utc::now().to_rfc3339());

    repository::update(db, oid, update_doc).await?;
    let updated = repository::find_by_id(db, oid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileResponse::from(updated))
}
