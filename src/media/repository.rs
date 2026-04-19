use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::models::MediaAsset;

fn col(db: &Database) -> mongodb::Collection<MediaAsset> {
    db.collection("mediaAssets")
}

pub async fn insert(db: &Database, asset: &MediaAsset) -> Result<ObjectId, AppError> {
    let r = col(db).insert_one(asset).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_by_id(db: &Database, id: ObjectId) -> Result<Option<MediaAsset>, AppError> {
    Ok(col(db).find_one(doc! { "_id": id }).await?)
}

pub async fn delete_by_id(db: &Database, id: ObjectId) -> Result<bool, AppError> {
    let r = col(db).delete_one(doc! { "_id": id }).await?;
    Ok(r.deleted_count > 0)
}
