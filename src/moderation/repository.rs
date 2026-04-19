use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::models::Report;

fn col(db: &Database) -> mongodb::Collection<Report> {
    db.collection("reports")
}

pub async fn insert(db: &Database, r: &Report) -> Result<ObjectId, AppError> {
    let res = col(db).insert_one(r).await?;
    Ok(res.inserted_id.as_object_id().unwrap())
}

pub async fn find_by_id(db: &Database, id: ObjectId) -> Result<Option<Report>, AppError> {
    Ok(col(db).find_one(doc! { "_id": id }).await?)
}

pub async fn list_all(db: &Database) -> Result<Vec<Report>, AppError> {
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .build();
    let items: Vec<Report> = col(db)
        .find(doc! {})
        .with_options(opts)
        .await?
        .try_collect()
        .await?;
    Ok(items)
}

pub async fn update_status(
    db: &Database,
    id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<bool, AppError> {
    let r = col(db)
        .update_one(doc! { "_id": id }, doc! { "$set": update })
        .await?;
    Ok(r.matched_count > 0)
}
