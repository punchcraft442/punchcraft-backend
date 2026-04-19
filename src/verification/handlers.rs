use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::Database;
use validator::Validate;

use crate::common::{errors::AppError, middleware::{require_admin, require_auth}, response};
use super::{models::{ReviewDocumentRequest, SubmitDocumentRequest}, service};

pub async fn submit_document(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<SubmitDocumentRequest>,
) -> Result<HttpResponse, AppError> {
    require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let doc = service::submit_document(db.get_ref(), body.into_inner()).await?;
    Ok(response::created(doc))
}

pub async fn list_pending(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let docs = service::list_pending(db.get_ref()).await?;
    Ok(response::ok(docs))
}

pub async fn review_document(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<ReviewDocumentRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let doc = service::review_document(db.get_ref(), &path.into_inner(), body.into_inner()).await?;
    Ok(response::ok(doc))
}
