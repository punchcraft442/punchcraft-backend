use chrono::Utc;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};

use crate::common::errors::AppError;
use super::{
    models::{CreateReportRequest, Report, ReportDecisionRequest, ReportStatus},
    repository,
};

pub async fn create_report(
    db: &Database,
    reporter_id: ObjectId,
    req: CreateReportRequest,
) -> Result<Report, AppError> {
    let profile_id = ObjectId::parse_str(&req.profile_id)
        .map_err(|_| AppError::BadRequest("Invalid profileId".into()))?;

    let report = Report {
        id: None,
        reporter_user_id: reporter_id,
        profile_id,
        reason: req.reason,
        status: ReportStatus::Open,
        admin_note: None,
        created_at: Utc::now(),
        reviewed_at: None,
    };

    let id = repository::insert(db, &report).await?;
    let mut report = report;
    report.id = Some(id);
    Ok(report)
}

pub async fn list_reports(db: &Database) -> Result<Vec<Report>, AppError> {
    repository::list_all(db).await
}

pub async fn decide_report(
    db: &Database,
    id_str: &str,
    req: ReportDecisionRequest,
) -> Result<Report, AppError> {
    let id = ObjectId::parse_str(id_str).map_err(|_| AppError::BadRequest("Invalid id".into()))?;

    let status = match req.status.as_str() {
        "reviewed" => ReportStatus::Reviewed,
        "dismissed" => ReportStatus::Dismissed,
        _ => return Err(AppError::BadRequest("status must be 'reviewed' or 'dismissed'".into())),
    };

    let mut update = doc! {
        "status": req.status,
        "reviewedAt": Utc::now().to_rfc3339(),
    };
    if let Some(note) = req.admin_note {
        update.insert("adminNote", note);
    }

    let found = repository::update_status(db, id, update).await?;
    if !found {
        return Err(AppError::NotFound);
    }

    let _ = status;
    repository::find_by_id(db, id).await?.ok_or(AppError::NotFound)
}
