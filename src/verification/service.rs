use chrono::Utc;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::{
    models::{ReviewDocumentRequest, ReviewStatus, SubmitDocumentRequest, VerificationDocument},
    repository,
};

pub async fn submit_document(
    db: &Database,
    req: SubmitDocumentRequest,
) -> Result<VerificationDocument, AppError> {
    let profile_id = ObjectId::parse_str(&req.profile_id)
        .map_err(|_| AppError::BadRequest("Invalid profileId".into()))?;

    let doc = VerificationDocument {
        id: None,
        profile_id,
        file_url: req.file_url,
        document_type: req.document_type,
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

pub async fn list_pending(db: &Database) -> Result<Vec<VerificationDocument>, AppError> {
    repository::list_pending(db).await
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

    repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)
}
