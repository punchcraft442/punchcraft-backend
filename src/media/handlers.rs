use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::TryStreamExt;
use mongodb::{bson::oid::ObjectId, Database};

use crate::common::{errors::AppError, middleware::require_auth, response};
use super::service;

pub async fn upload_media(
    req: HttpRequest,
    db: web::Data<Database>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let _ = claims;

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename = "upload".to_string();
    let mut profile_id_str: Option<String> = None;
    let mut category = "gallery".to_string();

    while let Some(mut field) = payload
        .try_next()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                if let Some(cd) = field.content_disposition() {
                    if let Some(fname) = cd.get_filename() {
                        filename = fname.to_string();
                    }
                }
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
                    bytes.extend_from_slice(&chunk);
                }
                file_data = Some(bytes);
            }
            "profileId" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
                    bytes.extend_from_slice(&chunk);
                }
                profile_id_str = Some(String::from_utf8_lossy(&bytes).to_string());
            }
            "category" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
                    bytes.extend_from_slice(&chunk);
                }
                category = String::from_utf8_lossy(&bytes).to_string();
            }
            _ => {
                while field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))?.is_some() {}
            }
        }
    }

    let data = file_data.ok_or_else(|| AppError::BadRequest("Missing file field".into()))?;
    let pid_str = profile_id_str.ok_or_else(|| AppError::BadRequest("Missing profileId field".into()))?;
    let profile_id = ObjectId::parse_str(&pid_str).map_err(|_| AppError::BadRequest("Invalid profileId".into()))?;

    let asset = service::upload_media(db.get_ref(), profile_id, data, filename, category).await?;
    Ok(response::created(asset))
}

pub async fn delete_media(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let _ = claims;
    service::delete_media(db.get_ref(), &path.into_inner(), None).await?;
    Ok(response::no_content())
}
