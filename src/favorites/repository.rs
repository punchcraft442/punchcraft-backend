use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::models::Favorite;

fn col(db: &Database) -> mongodb::Collection<Favorite> {
    db.collection("favorites")
}

pub async fn insert(db: &Database, f: &Favorite) -> Result<ObjectId, AppError> {
    let r = col(db).insert_one(f).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_by_user_and_profile(
    db: &Database,
    user_id: ObjectId,
    profile_id: ObjectId,
) -> Result<Option<Favorite>, AppError> {
    Ok(col(db)
        .find_one(doc! { "userId": user_id, "profileId": profile_id })
        .await?)
}

pub async fn find_by_id(db: &Database, id: ObjectId) -> Result<Option<Favorite>, AppError> {
    Ok(col(db).find_one(doc! { "_id": id }).await?)
}

pub async fn delete_by_id(db: &Database, id: ObjectId) -> Result<bool, AppError> {
    let r = col(db).delete_one(doc! { "_id": id }).await?;
    Ok(r.deleted_count > 0)
}

pub async fn list_for_user(db: &Database, user_id: ObjectId) -> Result<Vec<Favorite>, AppError> {
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .build();
    let items: Vec<Favorite> = col(db)
        .find(doc! { "userId": user_id })
        .with_options(opts)
        .await?
        .try_collect()
        .await?;
    Ok(items)
}
