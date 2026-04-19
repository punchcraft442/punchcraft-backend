use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::models::Notification;

fn col(db: &Database) -> mongodb::Collection<Notification> {
    db.collection("notifications")
}

pub async fn insert(db: &Database, n: &Notification) -> Result<ObjectId, AppError> {
    let r = col(db).insert_one(n).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn list_for_user(db: &Database, user_id: ObjectId) -> Result<Vec<Notification>, AppError> {
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .build();
    let items: Vec<Notification> = col(db)
        .find(doc! { "userId": user_id })
        .with_options(opts)
        .await?
        .try_collect()
        .await?;
    Ok(items)
}

pub async fn mark_read(db: &Database, id: ObjectId, user_id: ObjectId) -> Result<bool, AppError> {
    let r = col(db)
        .update_one(
            doc! { "_id": id, "userId": user_id },
            doc! { "$set": { "isRead": true } },
        )
        .await?;
    Ok(r.matched_count > 0)
}
