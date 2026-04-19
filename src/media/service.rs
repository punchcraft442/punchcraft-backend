use chrono::Utc;
use mongodb::{bson::oid::ObjectId, Database};

use crate::common::errors::AppError;
use super::{cloudinary::CloudinaryClient, models::MediaAsset, repository};

pub async fn upload_media(
    db: &Database,
    profile_id: ObjectId,
    data: Vec<u8>,
    filename: String,
    category: String,
) -> Result<MediaAsset, AppError> {
    let client = CloudinaryClient::from_env();
    let folder = format!("punchcraft/profiles/{}", profile_id.to_hex());
    let resp = client.upload(data, filename, &folder).await?;

    let asset = MediaAsset {
        id: None,
        profile_id,
        url: resp.secure_url,
        public_id: resp.public_id,
        media_type: "image".into(),
        category,
        moderation_status: "visible".into(),
        created_at: Utc::now(),
    };

    let id = repository::insert(db, &asset).await?;
    let mut asset = asset;
    asset.id = Some(id);
    Ok(asset)
}

pub async fn delete_media(
    db: &Database,
    id_str: &str,
    owner_profile_id: Option<ObjectId>,
) -> Result<(), AppError> {
    let id = ObjectId::parse_str(id_str).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let asset = repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)?;

    if let Some(owner) = owner_profile_id {
        if asset.profile_id != owner {
            return Err(AppError::Forbidden);
        }
    }

    let client = CloudinaryClient::from_env();
    client.delete(&asset.public_id).await?;

    repository::delete_by_id(db, id).await?;
    Ok(())
}
