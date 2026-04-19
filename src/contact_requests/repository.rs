use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::models::ContactRequest;

fn col(db: &Database) -> mongodb::Collection<ContactRequest> {
    db.collection("contactRequests")
}

pub async fn insert(db: &Database, r: &ContactRequest) -> Result<ObjectId, AppError> {
    let res = col(db).insert_one(r).await?;
    Ok(res.inserted_id.as_object_id().unwrap())
}

pub async fn find_by_id(db: &Database, id: ObjectId) -> Result<Option<ContactRequest>, AppError> {
    Ok(col(db).find_one(doc! { "_id": id }).await?)
}

pub async fn list_for_user(db: &Database, user_id: ObjectId) -> Result<Vec<ContactRequest>, AppError> {
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .build();
    let filter = doc! {
        "$or": [
            { "senderUserId": user_id },
        ]
    };
    let items: Vec<ContactRequest> = col(db)
        .find(filter)
        .with_options(opts)
        .await?
        .try_collect()
        .await?;
    Ok(items)
}

pub async fn update_status(
    db: &Database,
    id: ObjectId,
    status: &str,
) -> Result<bool, AppError> {
    let r = col(db)
        .update_one(doc! { "_id": id }, doc! { "$set": { "status": status } })
        .await?;
    Ok(r.matched_count > 0)
}
