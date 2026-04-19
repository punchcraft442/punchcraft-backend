use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::{bson::oid::ObjectId, Database};
use validator::Validate;

use crate::common::{errors::AppError, middleware::require_auth, response};
use super::{models::AddFavoriteRequest, service};

pub async fn add_favorite(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<AddFavoriteRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let fav = service::add(db.get_ref(), user_id, body.into_inner()).await?;
    Ok(response::created(fav))
}

pub async fn remove_favorite(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    service::remove(db.get_ref(), &path.into_inner(), user_id).await?;
    Ok(response::no_content())
}

pub async fn list_favorites(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let items = service::list(db.get_ref(), user_id).await?;
    Ok(response::ok(items))
}
