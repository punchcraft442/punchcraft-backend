use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::Database;
use validator::Validate;

use crate::common::{errors::AppError, middleware::require_auth, response};
use super::{models::UpdateMeRequest, service};

pub async fn get_me(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user = service::get_me(db.get_ref(), &claims.sub).await?;
    Ok(response::ok(user))
}

pub async fn update_me(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<UpdateMeRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let user = service::update_me(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(user))
}
