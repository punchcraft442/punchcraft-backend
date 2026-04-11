use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::Database;

use crate::auth::service::find_user_email;
use crate::common::{email::EmailService, errors::AppError, middleware::require_admin, response};
use super::service::{self, RejectRequest};

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
