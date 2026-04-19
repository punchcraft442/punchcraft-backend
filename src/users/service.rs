use chrono::Utc;
use mongodb::{bson::{doc, oid::ObjectId}, Database};

use crate::auth::models::User;
use crate::common::errors::AppError;
use super::models::{UpdateMeRequest, UserMeResponse};

pub async fn get_me(db: &Database, user_id: &str) -> Result<UserMeResponse, AppError> {
    let oid = ObjectId::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid userId".into()))?;
    let user = db
        .collection::<User>("users")
        .find_one(doc! { "_id": oid })
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(UserMeResponse::from_user(&user))
}

pub async fn update_me(db: &Database, user_id: &str, req: UpdateMeRequest) -> Result<UserMeResponse, AppError> {
    let oid = ObjectId::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid userId".into()))?;

    let mut update = doc! { "updatedAt": Utc::now().to_rfc3339() };
    if let Some(phone) = &req.phone {
        update.insert("phone", phone.as_str());
    }
    if let Some(photo) = &req.profile_photo {
        update.insert("profilePhoto", photo.as_str());
    }
    if let Some(links) = &req.social_links {
        let links_doc = mongodb::bson::to_document(links)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;
        update.insert("socialLinks", links_doc);
    }

    db.collection::<User>("users")
        .update_one(doc! { "_id": oid }, doc! { "$set": update })
        .await?;

    get_me(db, user_id).await
}
