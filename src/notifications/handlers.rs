use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::{bson::oid::ObjectId, Database};

use crate::common::{errors::AppError, middleware::require_auth, response};
use super::service;

pub async fn list_notifications(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let items = service::list(db.get_ref(), user_id).await?;
    Ok(response::ok(items))
}

pub async fn mark_read(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    service::mark_read(db.get_ref(), &path.into_inner(), user_id).await?;
    Ok(response::ok_message("Notification marked as read"))
}
