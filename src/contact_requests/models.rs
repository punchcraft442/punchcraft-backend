use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContactRequestStatus {
    Pending,
    Accepted,
    Declined,
}

impl Default for ContactRequestStatus {
    fn default() -> Self { Self::Pending }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContactRequest {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub sender_user_id: ObjectId,
    pub recipient_profile_id: ObjectId,
    pub message: String,
    pub status: ContactRequestStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactRequest {
    #[validate(length(min = 1, message = "recipientProfileId is required"))]
    pub recipient_profile_id: String,
    #[validate(length(min = 1, message = "message is required"))]
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContactRequest {
    pub status: String,
}
