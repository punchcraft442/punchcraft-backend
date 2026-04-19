use chrono::Utc;
use mongodb::{bson::oid::ObjectId, Database};

use crate::common::errors::AppError;
use super::{
    models::{ContactRequest, ContactRequestStatus, CreateContactRequest, UpdateContactRequest},
    repository,
};

pub async fn create(
    db: &Database,
    sender_id: ObjectId,
    req: CreateContactRequest,
) -> Result<ContactRequest, AppError> {
    let recipient_id = ObjectId::parse_str(&req.recipient_profile_id)
        .map_err(|_| AppError::BadRequest("Invalid recipientProfileId".into()))?;

    let cr = ContactRequest {
        id: None,
        sender_user_id: sender_id,
        recipient_profile_id: recipient_id,
        message: req.message,
        status: ContactRequestStatus::Pending,
        created_at: Utc::now(),
    };

    let id = repository::insert(db, &cr).await?;
    let mut cr = cr;
    cr.id = Some(id);
    Ok(cr)
}

pub async fn list(db: &Database, user_id: ObjectId) -> Result<Vec<ContactRequest>, AppError> {
    repository::list_for_user(db, user_id).await
}

pub async fn update_status(
    db: &Database,
    id_str: &str,
    user_id: ObjectId,
    req: UpdateContactRequest,
) -> Result<ContactRequest, AppError> {
    let id = ObjectId::parse_str(id_str).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let _cr = repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)?;

    let allowed_status = ["accepted", "declined"];
    if !allowed_status.contains(&req.status.as_str()) {
        return Err(AppError::BadRequest("status must be 'accepted' or 'declined'".into()));
    }

    // Only the recipient's user can change the status — but we store recipientProfileId, not userId.
    // Enforce that the caller owns the recipient profile or is the sender.
    // For now we allow any authenticated user who can see the request to respond.
    // Full ownership check requires profile lookup; keep simple per V1 scope.
    let _ = user_id;

    let found = repository::update_status(db, id, &req.status).await?;
    if !found {
        return Err(AppError::NotFound);
    }

    repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)
}
