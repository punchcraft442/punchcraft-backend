use serde::Deserialize;
use mongodb::Database;

use crate::common::errors::AppError;
use crate::profiles::{models::ProfileSummary, service as profile_service};

#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

pub async fn approve_profile(db: &Database, profile_id: &str) -> Result<ProfileSummary, AppError> {
    profile_service::admin_approve(db, profile_id).await
}

pub async fn reject_profile(
    db: &Database,
    profile_id: &str,
    req: RejectRequest,
) -> Result<ProfileSummary, AppError> {
    profile_service::admin_reject(db, profile_id, &req.reason).await
}
