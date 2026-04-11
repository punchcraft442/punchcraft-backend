use actix_web::{body::MessageBody, dev::{Service, ServiceResponse}, http::StatusCode, test};
use actix_http::Request;
use mongodb::{bson::doc, Database};
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};
use punchcraft::auth::models::User;

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn register_user<S, B>(app: &S, email: &str, password: &str) -> Value
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": email, "password": password, "role": "fighter" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    test::read_body_json(resp).await
}

/// Activates the account for the given email by reading the token from the DB
/// and calling the verify-email endpoint.
async fn activate_user<S, B>(app: &S, db: &Database, email: &str)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let user = db
        .collection::<User>("users")
        .find_one(doc! { "email": email })
        .await
        .unwrap()
        .unwrap();
    let token = user.activation_token.expect("activation_token must be set after register");
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/auth/verify-email?token={token}"))
        .to_request();
    test::call_service(app, req).await;
}

async fn register_and_activate<S, B>(app: &S, db: &Database, email: &str, password: &str) -> Value
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let body = register_user(app, email, password).await;
    activate_user(app, db, email).await;
    body
}

async fn login_user<S, B>(app: &S, email: &str, password: &str) -> ServiceResponse<B>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({ "email": email, "password": password }))
        .to_request();
    test::call_service(app, req).await
}

// ── Register ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_register_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "kwame@example.com", "password": "StrongPass123!", "role": "fighter" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["email"], "kwame@example.com");
    assert_eq!(body["data"]["role"], "fighter");
    assert_eq!(body["data"]["accountStatus"], "inactive");
    assert!(body["data"]["userId"].is_string());

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_register_sets_account_inactive_in_db() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "inactive@example.com", "StrongPass123!").await;

    let user = db
        .collection::<User>("users")
        .find_one(doc! { "email": "inactive@example.com" })
        .await.unwrap().unwrap();

    assert!(!user.is_active, "account must be inactive until email is verified");
    assert!(user.activation_token.is_some(), "activation_token must be stored");
    assert!(user.activation_token_expires.is_some());

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_register_duplicate_email_returns_409() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "dup@example.com", "StrongPass123!").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "dup@example.com", "password": "StrongPass123!", "role": "fighter" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_register_invalid_email_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "not-an-email", "password": "StrongPass123!", "role": "fighter" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_register_short_password_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "valid@example.com", "password": "short", "role": "fighter" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

// ── Verify Email ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_verify_email_activates_account() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "activate@example.com", "StrongPass123!").await;

    let user_before = db.collection::<User>("users")
        .find_one(doc! { "email": "activate@example.com" })
        .await.unwrap().unwrap();
    assert!(!user_before.is_active);

    activate_user(&app, &db, "activate@example.com").await;

    let user_after = db.collection::<User>("users")
        .find_one(doc! { "email": "activate@example.com" })
        .await.unwrap().unwrap();
    assert!(user_after.is_active);
    assert!(user_after.activation_token.is_none(), "token must be cleared after activation");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_verify_email_invalid_token_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/verify-email?token=completely-invalid-token")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_verify_email_token_cannot_be_reused() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "reuse@example.com", "StrongPass123!").await;
    let user = db.collection::<User>("users")
        .find_one(doc! { "email": "reuse@example.com" })
        .await.unwrap().unwrap();
    let token = user.activation_token.unwrap();

    // First use — succeeds
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/auth/verify-email?token={token}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Second use — must fail (token cleared from DB)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/auth/verify-email?token={token}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

// ── Login ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_login_success_returns_tokens_and_user() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_and_activate(&app, &db, "login@example.com", "StrongPass123!").await;
    let resp = login_user(&app, "login@example.com", "StrongPass123!").await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["data"]["accessToken"].is_string());
    assert!(body["data"]["refreshToken"].is_string());
    assert_eq!(body["data"]["user"]["email"], "login@example.com");
    assert_eq!(body["data"]["user"]["accountStatus"], "active");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_login_inactive_account_returns_403() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "notactive@example.com", "StrongPass123!").await;
    // Do NOT activate
    let resp = login_user(&app, "notactive@example.com", "StrongPass123!").await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_and_activate(&app, &db, "wrongpw@example.com", "StrongPass123!").await;
    let resp = login_user(&app, "wrongpw@example.com", "WrongPass999!").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_login_unknown_email_returns_401() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let resp = login_user(&app, "ghost@example.com", "StrongPass123!").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

// ── Refresh Token ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_refresh_token_returns_new_access_token() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_and_activate(&app, &db, "refresh@example.com", "StrongPass123!").await;
    let login_resp = login_user(&app, "refresh@example.com", "StrongPass123!").await;
    let login_body: Value = test::read_body_json(login_resp).await;
    let refresh_token = login_body["data"]["refreshToken"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .set_json(json!({ "refreshToken": refresh_token }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert!(body["data"]["accessToken"].is_string());

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_refresh_token_invalid_returns_401() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .set_json(json!({ "refreshToken": "not-a-real-token" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

// ── Logout ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_logout_clears_refresh_token() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_and_activate(&app, &db, "logout@example.com", "StrongPass123!").await;
    let login_resp = login_user(&app, "logout@example.com", "StrongPass123!").await;
    let login_body: Value = test::read_body_json(login_resp).await;
    let access_token = login_body["data"]["accessToken"].as_str().unwrap().to_string();
    let refresh_token = login_body["data"]["refreshToken"].as_str().unwrap().to_string();

    // Logout
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Refresh token no longer valid
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .set_json(json!({ "refreshToken": refresh_token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_logout_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

// ── Forgot Password ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_forgot_password_always_returns_200_for_unknown_email() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(json!({ "email": "ghost@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Must never reveal whether the email exists
    assert_eq!(resp.status(), StatusCode::OK);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_forgot_password_stores_reset_token_on_user() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "forgot@example.com", "StrongPass123!").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(json!({ "email": "forgot@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = db
        .collection::<User>("users")
        .find_one(doc! { "email": "forgot@example.com" })
        .await.unwrap().unwrap();

    assert!(user.reset_token.is_some());
    assert!(user.reset_token_expires.is_some());

    teardown_db(&db).await;
}

// ── Reset Password ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_reset_password_invalid_token_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(json!({ "token": "completely-invalid-token", "newPassword": "NewStrongPass123!" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_reset_password_full_flow() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_and_activate(&app, &db, "reset@example.com", "OldPass123!").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(json!({ "email": "reset@example.com" }))
        .to_request();
    test::call_service(&app, req).await;

    let user = db
        .collection::<User>("users")
        .find_one(doc! { "email": "reset@example.com" })
        .await.unwrap().unwrap();
    let token = user.reset_token.unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(json!({ "token": token, "newPassword": "NewPass123!" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Old password no longer works
    assert_eq!(login_user(&app, "reset@example.com", "OldPass123!").await.status(), StatusCode::UNAUTHORIZED);
    // New password works
    assert_eq!(login_user(&app, "reset@example.com", "NewPass123!").await.status(), StatusCode::OK);

    // Token is cleared
    let updated = db.collection::<User>("users")
        .find_one(doc! { "email": "reset@example.com" })
        .await.unwrap().unwrap();
    assert!(updated.reset_token.is_none());

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_reset_password_token_cannot_be_reused() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "reuse@example.com", "OldPass123!").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(json!({ "email": "reuse@example.com" }))
        .to_request();
    test::call_service(&app, req).await;

    let user = db.collection::<User>("users")
        .find_one(doc! { "email": "reuse@example.com" })
        .await.unwrap().unwrap();
    let token = user.reset_token.unwrap();

    // Use token once
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(json!({ "token": &token, "newPassword": "NewPass123!" }))
        .to_request();
    test::call_service(&app, req).await;

    // Attempt to reuse — must fail
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(json!({ "token": &token, "newPassword": "AnotherPass123!" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

// ── Change Password ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_change_password_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::patch()
        .uri("/api/v1/auth/change-password")
        .set_json(json!({ "currentPassword": "OldPass123!", "newPassword": "NewPass123!" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_change_password_wrong_current_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_user(&app, "changepw@example.com", "OldPass123!").await;
    let user = db.collection::<User>("users")
        .find_one(doc! { "email": "changepw@example.com" })
        .await.unwrap().unwrap();
    let jwt = make_jwt(&user.id.unwrap().to_hex(), "fighter");

    let req = test::TestRequest::patch()
        .uri("/api/v1/auth/change-password")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "currentPassword": "WrongCurrent!", "newPassword": "NewPass123!" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_change_password_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    register_and_activate(&app, &db, "success@example.com", "OldPass123!").await;
    let user = db.collection::<User>("users")
        .find_one(doc! { "email": "success@example.com" })
        .await.unwrap().unwrap();
    let jwt = make_jwt(&user.id.unwrap().to_hex(), "fighter");

    let req = test::TestRequest::patch()
        .uri("/api/v1/auth/change-password")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "currentPassword": "OldPass123!", "newPassword": "NewPass123!" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert_eq!(login_user(&app, "success@example.com", "OldPass123!").await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(login_user(&app, "success@example.com", "NewPass123!").await.status(), StatusCode::OK);

    teardown_db(&db).await;
}
