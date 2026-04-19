use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::{bson::oid::ObjectId, Database};
use validator::Validate;

use crate::common::{errors::AppError, middleware::{require_admin, require_auth}, response};
use super::{models::{CreateReportRequest, ReportDecisionRequest}, service};

pub async fn create_report(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateReportRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let reporter_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let report = service::create_report(db.get_ref(), reporter_id, body.into_inner()).await?;
    Ok(response::created(report))
}

pub async fn list_reports(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let reports = service::list_reports(db.get_ref()).await?;
    Ok(response::ok(reports))
}

pub async fn decide_report(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<ReportDecisionRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let report = service::decide_report(db.get_ref(), &path.into_inner(), body.into_inner()).await?;
    Ok(response::ok(report))
}
