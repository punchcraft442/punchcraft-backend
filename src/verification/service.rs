use chrono::Utc;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use crate::profiles::repository as profiles_repo;
use super::{
    models::{ReviewDocumentRequest, ReviewStatus, VerificationDocument},
    repository,
};

pub async fn submit_document(
    db: &Database,
    profile_id_str: String,
    document_type: String,
    file_url: String,
) -> Result<VerificationDocument, AppError> {
    let profile_id = ObjectId::parse_str(&profile_id_str)
        .map_err(|_| AppError::BadRequest("Invalid profileId".into()))?;

    let doc = VerificationDocument {
        id: None,
        profile_id,
        file_url,
        document_type,
        review_status: ReviewStatus::Pending,
        admin_note: None,
        submitted_at: Utc::now(),
        reviewed_at: None,
    };

    let id = repository::insert(db, &doc).await?;
    let mut doc = doc;
    doc.id = Some(id);
    Ok(doc)
}

pub async fn get_document(db: &Database, id_str: &str) -> Result<VerificationDocument, AppError> {
    let id = ObjectId::parse_str(id_str).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)
}

pub async fn list_pending(db: &Database) -> Result<Vec<VerificationDocument>, AppError> {
    repository::list_pending(db).await
}

pub async fn list_all(db: &Database, status: Option<&str>) -> Result<Vec<VerificationDocument>, AppError> {
    if let Some(s) = status {
        match s {
            "pending" | "approved" | "rejected" => {}
            _ => return Err(AppError::BadRequest("status must be pending, approved, or rejected".into())),
        }
    }
    repository::list_all(db, status).await
}

pub async fn review_document(
    db: &Database,
    id_str: &str,
    req: ReviewDocumentRequest,
) -> Result<VerificationDocument, AppError> {
    let id = ObjectId::parse_str(id_str).map_err(|_| AppError::BadRequest("Invalid id".into()))?;

    let status_str = match req.status.as_str() {
        "approved" => "approved",
        "rejected" => "rejected",
        _ => return Err(AppError::BadRequest("status must be 'approved' or 'rejected'".into())),
    };

    let mut update = doc! {
        "reviewStatus": status_str,
        "reviewedAt": Utc::now().to_rfc3339(),
    };
    if let Some(note) = req.admin_note {
        update.insert("adminNote", note);
    }

    let found = repository::update_review(db, id, update).await?;
    if !found {
        return Err(AppError::NotFound);
    }

    let doc = repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)?;

    // When a document is approved, mark the profile as having a verified document
    // so it becomes publicly visible (requires both admin profile approval + this flag).
    if status_str == "approved" {
        let _ = profiles_repo::set_has_verified_document(db, doc.profile_id, true).await;
    }

    Ok(doc)
}
