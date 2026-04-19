use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id: ObjectId,
    pub title: String,
    pub message: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(user_id: ObjectId, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: None,
            user_id,
            title: title.into(),
            message: message.into(),
            is_read: false,
            created_at: Utc::now(),
        }
    }
}
