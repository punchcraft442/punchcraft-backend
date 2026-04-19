use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Open,
    Reviewed,
    Dismissed,
}

impl Default for ReportStatus {
    fn default() -> Self { Self::Open }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub reporter_user_id: ObjectId,
    pub profile_id: ObjectId,
    pub reason: String,
    pub status: ReportStatus,
    pub admin_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportRequest {
    #[validate(length(min = 1, message = "profileId is required"))]
    pub profile_id: String,
    #[validate(length(min = 5, message = "reason must be at least 5 characters"))]
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDecisionRequest {
    pub status: String,
    pub admin_note: Option<String>,
}
