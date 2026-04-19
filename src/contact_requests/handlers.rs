use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::{bson::oid::ObjectId, Database};
use validator::Validate;

use crate::common::{errors::AppError, middleware::require_auth, response};
use super::{models::{CreateContactRequest, UpdateContactRequest}, service};

pub async fn create(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateContactRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let sender_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let cr = service::create(db.get_ref(), sender_id, body.into_inner()).await?;
    Ok(response::created(cr))
}

pub async fn list(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let items = service::list(db.get_ref(), user_id).await?;
    Ok(response::ok(items))
}

pub async fn update_status(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateContactRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let cr = service::update_status(db.get_ref(), &path.into_inner(), user_id, body.into_inner()).await?;
    Ok(response::ok(cr))
}
