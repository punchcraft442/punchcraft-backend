use actix_web::{body::MessageBody, dev::{Service, ServiceResponse}, http::StatusCode, test};
use actix_http::Request;
use mongodb::bson::doc;
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};
use punchcraft::auth::models::User;

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn register_and_get_jwt<S, B>(app: &S, db: &mongodb::Database, email: &str, role: &str) -> String
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": email, "password": "StrongPass123!", "role": role }))
        .to_request();
    test::call_service(app, req).await;

    let user = db
        .collection::<User>("users")
        .find_one(doc! { "email": email })
        .await
        .unwrap()
        .unwrap();

    make_jwt(&user.id.unwrap().to_hex(), role)
}

async fn create_profile<S, B>(app: &S, jwt: &str, display_name: &str) -> Value
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "display_name": display_name, "role": "fighter" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    test::read_body_json(resp).await
}

// ── Create Profile ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_profile_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles")
        .set_json(json!({ "display_name": "Test Fighter", "role": "fighter" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_create_profile_success_starts_as_draft_and_private() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = register_and_get_jwt(&app, &db, "fighter@example.com", "fighter").await;
    let body = create_profile(&app, &jwt, "Kwame Mensah").await;

    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["status"], "draft");
    assert_eq!(body["data"]["visibility"], "private");
    assert_eq!(body["data"]["verification_tier"], "unverified");
    assert_eq!(body["data"]["display_name"], "Kwame Mensah");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_create_profile_duplicate_returns_409() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = register_and_get_jwt(&app, &db, "dup@example.com", "fighter").await;
    create_profile(&app, &jwt, "First Profile").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "display_name": "Second Profile", "role": "fighter" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    teardown_db(&db).await;
}

// ── Get Profile ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_profile_does_not_require_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = register_and_get_jwt(&app, &db, "pub@example.com", "fighter").await;
    let body = create_profile(&app, &jwt, "Public Fighter").await;
    let profile_id = body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/{}", profile_id))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_get_unknown_profile_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/000000000000000000000000")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    teardown_db(&db).await;
}

// ── Update Profile ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_profile_requires_ownership() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let owner_jwt = register_and_get_jwt(&app, &db, "owner@example.com", "fighter").await;
    let body = create_profile(&app, &owner_jwt, "Owner").await;
    let profile_id = body["data"]["id"].as_str().unwrap();

    // Different user tries to update
    let other_jwt = register_and_get_jwt(&app, &db, "other@example.com", "fighter").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/{}", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", other_jwt)))
        .set_json(json!({ "bio": "Hijacked bio" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    teardown_db(&db).await;
}

// ── Submit Profile ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_submit_profile_changes_status_to_submitted() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = register_and_get_jwt(&app, &db, "submit@example.com", "fighter").await;
    let body = create_profile(&app, &jwt, "Submitting Fighter").await;
    let profile_id = body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/{}/submit", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "submitted");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_submit_already_submitted_profile_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = register_and_get_jwt(&app, &db, "double@example.com", "fighter").await;
    let body = create_profile(&app, &jwt, "Double Submit").await;
    let profile_id = body["data"]["id"].as_str().unwrap();

    // Submit once
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/{}/submit", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    test::call_service(&app, req).await;

    // Submit again — must be rejected
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/{}/submit", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_submit_requires_ownership() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let owner_jwt = register_and_get_jwt(&app, &db, "own2@example.com", "fighter").await;
    let body = create_profile(&app, &owner_jwt, "Owned Profile").await;
    let profile_id = body["data"]["id"].as_str().unwrap();

    let other_jwt = register_and_get_jwt(&app, &db, "other2@example.com", "fighter").await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/{}/submit", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", other_jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    teardown_db(&db).await;
}
