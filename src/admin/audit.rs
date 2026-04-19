use chrono::Utc;
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};
use serde::{Deserialize, Serialize};

use crate::common::errors::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuditLog {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub admin_id: String,
    pub action: String,
    pub target_id: String,
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub created_at: String,
}

fn col(db: &Database) -> mongodb::Collection<AuditLog> {
    db.collection("auditLogs")
}

/// Fire-and-forget audit log write — called from handlers after successful admin actions.
pub async fn record(
    db: &Database,
    admin_id: &str,
    action: &str,
    target_id: &str,
    target_type: &str,
    detail: Option<String>,
) {
    let entry = AuditLog {
        id: None,
        admin_id: admin_id.to_string(),
        action: action.to_string(),
        target_id: target_id.to_string(),
        target_type: target_type.to_string(),
        detail,
        created_at: Utc::now().to_rfc3339(),
    };
    if let Err(e) = col(db).insert_one(&entry).await {
        tracing::warn!("audit log write failed: {e}");
    }
}

pub async fn list(
    db: &Database,
    admin_id_filter: Option<&str>,
    action_filter: Option<&str>,
    page: u32,
    limit: u32,
) -> Result<(Vec<AuditLog>, u64), AppError> {
    let mut filter = doc! {};
    if let Some(aid) = admin_id_filter {
        filter.insert("adminId", aid);
    }
    if let Some(act) = action_filter {
        filter.insert("action", act);
    }

    let c = col(db);
    let total = c.count_documents(filter.clone()).await?;
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .skip(((page.saturating_sub(1)) * limit) as u64)
        .limit(limit as i64)
        .build();
    let items: Vec<AuditLog> = c.find(filter).with_options(opts).await?.try_collect().await?;
    Ok((items, total))
}
