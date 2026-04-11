use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::Database;
use validator::Validate;

use crate::common::{email::EmailService, errors::AppError, middleware::require_auth, response};
use super::{
    models::{CreateProfileRequest, UpdateProfileRequest},
    service,
};

pub async fn create_profile(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateProfileRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_profile(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_profile(
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let profile = service::get_profile(db.get_ref(), &path.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn update_profile(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_profile(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_profile(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_profile(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.display_name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}
