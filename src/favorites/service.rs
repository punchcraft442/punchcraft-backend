use chrono::Utc;
use mongodb::{bson::oid::ObjectId, Database};

use crate::common::errors::AppError;
use super::{models::{AddFavoriteRequest, Favorite}, repository};

pub async fn add(
    db: &Database,
    user_id: ObjectId,
    req: AddFavoriteRequest,
) -> Result<Favorite, AppError> {
    let profile_id = ObjectId::parse_str(&req.profile_id)
        .map_err(|_| AppError::BadRequest("Invalid profileId".into()))?;

    if repository::find_by_user_and_profile(db, user_id, profile_id).await?.is_some() {
        return Err(AppError::Conflict("Already in favorites".into()));
    }

    let fav = Favorite {
        id: None,
        user_id,
        profile_id,
        created_at: Utc::now(),
    };

    let id = repository::insert(db, &fav).await?;
    let mut fav = fav;
    fav.id = Some(id);
    Ok(fav)
}

pub async fn remove(db: &Database, id_str: &str, user_id: ObjectId) -> Result<(), AppError> {
    let id = ObjectId::parse_str(id_str).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let fav = repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)?;
    if fav.user_id != user_id {
        return Err(AppError::Forbidden);
    }
    repository::delete_by_id(db, id).await?;
    Ok(())
}

pub async fn list(db: &Database, user_id: ObjectId) -> Result<Vec<Favorite>, AppError> {
    repository::list_for_user(db, user_id).await
}
