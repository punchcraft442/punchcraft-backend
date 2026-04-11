use mongodb::{bson::{doc, oid::ObjectId}, Database};

use crate::common::errors::AppError;
use super::models::Profile;

pub async fn insert(db: &Database, profile: &Profile) -> Result<ObjectId, AppError> {
    let col = db.collection::<Profile>("profiles");
    let result = col.insert_one(profile).await?;
    Ok(result.inserted_id.as_object_id().unwrap())
}

pub async fn find_by_id(db: &Database, id: ObjectId) -> Result<Option<Profile>, AppError> {
    let col = db.collection::<Profile>("profiles");
    Ok(col.find_one(doc! { "_id": id }).await?)
}

pub async fn find_by_user_id(db: &Database, user_id: ObjectId) -> Result<Option<Profile>, AppError> {
    let col = db.collection::<Profile>("profiles");
    Ok(col.find_one(doc! { "user_id": user_id }).await?)
}

pub async fn update(db: &Database, id: ObjectId, update_doc: mongodb::bson::Document) -> Result<(), AppError> {
    let col = db.collection::<Profile>("profiles");
    col.update_one(doc! { "_id": id }, doc! { "$set": update_doc }).await?;
    Ok(())
}
