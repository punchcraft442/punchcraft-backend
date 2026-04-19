use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
}

impl Default for ReviewStatus {
    fn default() -> Self { Self::Pending }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VerificationDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub profile_id: ObjectId,
    pub file_url: String,
    pub document_type: String,
    pub review_status: ReviewStatus,
    pub admin_note: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SubmitDocumentRequest {
    #[validate(length(min = 1, message = "profileId is required"))]
    pub profile_id: String,
    #[validate(url(message = "fileUrl must be a valid URL"))]
    pub file_url: String,
    #[validate(length(min = 1, message = "documentType is required"))]
    pub document_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDocumentRequest {
    pub status: String,
    pub admin_note: Option<String>,
}
