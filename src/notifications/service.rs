use mongodb::{bson::oid::ObjectId, Database};

use crate::common::errors::AppError;
use super::{models::Notification, repository};

pub async fn list(db: &Database, user_id: ObjectId) -> Result<Vec<Notification>, AppError> {
    repository::list_for_user(db, user_id).await
}

pub async fn mark_read(db: &Database, id_str: &str, user_id: ObjectId) -> Result<(), AppError> {
    let id = ObjectId::parse_str(id_str).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let found = repository::mark_read(db, id, user_id).await?;
    if !found {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn create(
    db: &Database,
    user_id: ObjectId,
    title: impl Into<String>,
    message: impl Into<String>,
) -> Result<Notification, AppError> {
    let n = Notification::new(user_id, title, message);
    let id = repository::insert(db, &n).await?;
    let mut n = n;
    n.id = Some(id);
    Ok(n)
}
