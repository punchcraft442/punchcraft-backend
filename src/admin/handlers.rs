use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::Database;
use serde::Deserialize;
use validator::Validate;

use crate::auth::service::find_user_email;
use crate::common::{
    email::EmailService,
    errors::AppError,
    middleware::{require_admin, require_super_admin},
    response,
};
use super::service::{
    self, ChangeRoleRequest, CreateUserDirectRequest, RejectRequest, SetVerificationTierRequest,
};

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl PageQuery {
    fn page(&self) -> u32 { self.page.unwrap_or(1).max(1) }
    fn limit(&self) -> u32 { self.limit.unwrap_or(20).min(100) }
}

// ── Profile approval ──────────────────────────────────────────────────────────

pub async fn get_approval_queue(
    req: HttpRequest,
    db: web::Data<Database>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let (items, total) = service::get_approval_queue(db.get_ref(), query.page(), query.limit()).await?;
    Ok(response::ok(serde_json::json!({ "items": items, "total": total })))
}

pub async fn approve_profile(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let profile = service::approve_profile(db.get_ref(), &path.into_inner()).await?;
    if let Some(user_email) = find_user_email(db.get_ref(), &profile.user_id).await {
        email.send_profile_approved(user_email, profile.display_name.clone());
    }
    Ok(response::ok(profile))
}

pub async fn reject_profile(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
    body: web::Json<RejectRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let reason = body.reason.clone();
    let profile = service::reject_profile(db.get_ref(), &path.into_inner(), body.into_inner()).await?;
    if let Some(user_email) = find_user_email(db.get_ref(), &profile.user_id).await {
        email.send_profile_rejected(user_email, profile.display_name.clone(), reason);
    }
    Ok(response::ok(profile))
}

pub async fn set_verification_tier(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<SetVerificationTierRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let profile = service::set_verification_tier(db.get_ref(), &path.into_inner(), body.into_inner()).await?;
    Ok(response::ok(profile))
}

// ── User management (admin + super_admin) ─────────────────────────────────────

pub async fn list_users(
    req: HttpRequest,
    db: web::Data<Database>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let (items, total) = service::list_users(db.get_ref(), query.page(), query.limit()).await?;
    Ok(response::ok(serde_json::json!({ "items": items, "total": total })))
}

pub async fn get_user(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let user = service::get_user(db.get_ref(), &path.into_inner()).await?;
    Ok(response::ok(user))
}

pub async fn suspend_user(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_admin(&req)?;
    let user = service::suspend_user(db.get_ref(), &path.into_inner(), &claims.role).await?;
    Ok(response::ok(user))
}

pub async fn activate_user(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin(&req)?;
    let user = service::activate_user(db.get_ref(), &path.into_inner()).await?;
    Ok(response::ok(user))
}

// ── Super admin only ──────────────────────────────────────────────────────────

pub async fn create_user_direct(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateUserDirectRequest>,
) -> Result<HttpResponse, AppError> {
    require_super_admin(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let user = service::create_user_direct(db.get_ref(), body.into_inner()).await?;
    Ok(response::created(user))
}

pub async fn ban_user(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_super_admin(&req)?;
    service::ban_user(db.get_ref(), &path.into_inner()).await?;
    Ok(response::no_content())
}

pub async fn change_user_role(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<ChangeRoleRequest>,
) -> Result<HttpResponse, AppError> {
    require_super_admin(&req)?;
    let user = service::change_user_role(db.get_ref(), &path.into_inner(), body.into_inner()).await?;
    Ok(response::ok(user))
}
