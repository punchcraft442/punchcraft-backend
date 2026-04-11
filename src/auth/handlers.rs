use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::Database;
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

use crate::common::{email::EmailService, errors::AppError, middleware::require_auth, response};
use super::{
    models::{ChangePasswordRequest, ForgotPasswordRequest, LoginRequest, RefreshTokenRequest, RegisterRequest, ResetPasswordRequest},
    service,
};

#[derive(Deserialize)]
pub struct VerifyEmailQuery {
    pub token: String,
}

pub async fn register(
    db: web::Data<Database>,
    email_svc: web::Data<EmailService>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let req = body.into_inner();
    let (resp, activation_token) = service::register(db.get_ref(), req).await?;
    email_svc.send_activation_email(resp.email.clone(), activation_token);
    Ok(HttpResponse::Created().json(json!({
        "success": true,
        "message": "Account created successfully. Please check your email to activate your account.",
        "data": resp,
    })))
}

pub async fn verify_email(
    db: web::Data<Database>,
    query: web::Query<VerifyEmailQuery>,
) -> Result<HttpResponse, AppError> {
    service::verify_email(db.get_ref(), &query.token).await?;
    Ok(response::ok_message("Account activated successfully. You can now log in."))
}

pub async fn login(
    db: web::Data<Database>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let data = service::login(db.get_ref(), body.into_inner()).await?;
    Ok(response::ok(data))
}

pub async fn refresh_token(
    db: web::Data<Database>,
    body: web::Json<RefreshTokenRequest>,
) -> Result<HttpResponse, AppError> {
    let new_access_token = service::refresh_access_token(db.get_ref(), body.into_inner()).await?;
    Ok(response::ok(json!({ "accessToken": new_access_token })))
}

pub async fn logout(
    req: HttpRequest,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::logout(db.get_ref(), &claims.sub).await?;
    Ok(response::ok_message("Logged out successfully."))
}

pub async fn forgot_password(
    db: web::Data<Database>,
    email_svc: web::Data<EmailService>,
    body: web::Json<ForgotPasswordRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    // Always return 200 regardless — prevents email enumeration
    if let Some((user_email, token)) = service::forgot_password(db.get_ref(), body.into_inner()).await? {
        email_svc.send_password_reset(user_email, token);
    }
    Ok(response::ok(json!({
        "message": "If that email is registered, a reset link has been sent."
    })))
}

pub async fn reset_password(
    db: web::Data<Database>,
    body: web::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    service::reset_password(db.get_ref(), body.into_inner()).await?;
    Ok(response::ok_message("Password reset successfully."))
}

pub async fn change_password(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let claims = require_auth(&req)?;
    service::change_password(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok_message("Password changed successfully."))
}
