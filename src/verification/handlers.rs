use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::TryStreamExt;
use mongodb::Database;
use serde::Deserialize;

use crate::common::{errors::AppError, middleware::{require_admin, require_auth}, response};
use crate::media::cloudinary::CloudinaryClient;
use super::{models::ReviewDocumentRequest, service};

#[derive(Deserialize)]
pub struct StatusQuery {
    pub status: Option<String>,
}

pub async fn submit_document(
    req: HttpRequest,
    db: web::Data<Database>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    require_auth(&req)?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = String::from("upload.bin");
    let mut profile_id: Option<String> = None;
    let mut document_type: Option<String> = None;

    while let Some(mut field) = payload
        .try_next()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                filename = field
                    .content_disposition()
                    .and_then(|cd| cd.get_filename().map(|s| s.to_string()))
                    .unwrap_or_else(|| "upload.bin".to_string());
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
                    bytes.extend_from_slice(&chunk);
                }
                file_bytes = Some(bytes);
            }
            "profileId" => {
                let mut buf = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
                    buf.extend_from_slice(&chunk);
                }
                profile_id = Some(String::from_utf8_lossy(&buf).trim().to_string());
            }
            "documentType" => {
                let mut buf = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
                    buf.extend_from_slice(&chunk);
                }
                document_type = Some(String::from_utf8_lossy(&buf).trim().to_string());
            }
            _ => {
                while field.try_next().await.map_err(|e| AppError::BadRequest(e.to_string()))?.is_some() {}
            }
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| AppError::BadRequest("Missing 'file' field".into()))?;
    let profile_id = profile_id.filter(|s| !s.is_empty()).ok_or_else(|| AppError::BadRequest("Missing 'profileId' field".into()))?;
    let document_type = document_type.filter(|s| !s.is_empty()).ok_or_else(|| AppError::BadRequest("Missing 'documentType' field".into()))?;

    let cloudinary = CloudinaryClient::from_env()?;
    let upload = cloudinary.upload_auto(file_bytes, filename, "verification_documents").await?;

    let doc = service::submit_document(db.get_ref(), profile_id, document_type, upload.secure_url).await?;
    Ok(response::created(doc))
}

pub async fn list_all(
    req: HttpRequest,
    db: web::Data<Database>,
    query: web::Query<StatusQuery>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let docs = service::list_all(db.get_ref(), query.status.as_deref()).await?;
    Ok(response::ok(docs))
}

pub async fn list_pending(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let docs = service::list_pending(db.get_ref()).await?;
    Ok(response::ok(docs))
}

pub async fn get_document(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let doc = service::get_document(db.get_ref(), &path.into_inner()).await?;
    Ok(response::ok(doc))
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
