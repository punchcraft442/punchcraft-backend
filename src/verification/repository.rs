use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::models::VerificationDocument;

fn col(db: &Database) -> mongodb::Collection<VerificationDocument> {
    db.collection("verificationDocuments")
}

pub async fn insert(db: &Database, d: &VerificationDocument) -> Result<ObjectId, AppError> {
    let r = col(db).insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_by_id(db: &Database, id: ObjectId) -> Result<Option<VerificationDocument>, AppError> {
    Ok(col(db).find_one(doc! { "_id": id }).await?)
}

pub async fn list_pending(db: &Database) -> Result<Vec<VerificationDocument>, AppError> {
    list_by_status(db, Some("pending")).await
}

pub async fn list_all(db: &Database, status: Option<&str>) -> Result<Vec<VerificationDocument>, AppError> {
    list_by_status(db, status).await
}

async fn list_by_status(db: &Database, status: Option<&str>) -> Result<Vec<VerificationDocument>, AppError> {
    let filter = match status {
        Some(s) => doc! { "reviewStatus": s },
        None => doc! {},
    };
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "submittedAt": -1 })
        .build();
    let items: Vec<VerificationDocument> = col(db)
        .find(filter)
        .with_options(opts)
        .await?
        .try_collect()
        .await?;
    Ok(items)
}

pub async fn update_review(
    db: &Database,
    id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<bool, AppError> {
    let r = col(db)
        .update_one(doc! { "_id": id }, doc! { "$set": update })
        .await?;
    Ok(r.matched_count > 0)
}
