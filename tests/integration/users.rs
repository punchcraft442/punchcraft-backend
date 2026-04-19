use actix_web::{http::StatusCode, test};
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};

const USER_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";

// ── GET /users/me ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_me_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/users/me")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn get_me_unknown_user_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/users/me")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn get_me_returns_user_data() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Register a real user so the document exists in DB
    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "me@example.com", "password": "StrongPass1!", "role": "fighter" }))
        .to_request();
    let register_resp = test::call_service(&app, register_req).await;
    let body: Value = test::read_body_json(register_resp).await;
    let user_id = body["data"]["userId"].as_str().unwrap().to_string();

    let jwt = make_jwt(&user_id, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/users/me")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["email"], "me@example.com");
    assert_eq!(body["data"]["role"], "fighter");
    assert!(body["data"]["accountStatus"].is_string());
    teardown_db(&db).await;
}

// ── PATCH /users/me ───────────────────────────────────────────────────────────

#[tokio::test]
async fn update_me_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .set_json(json!({ "phone": "+233500000000" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_me_invalid_url_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Register so user exists
    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "me2@example.com", "password": "StrongPass1!", "role": "fighter" }))
        .to_request();
    let resp = test::call_service(&app, register_req).await;
    let body: Value = test::read_body_json(resp).await;
    let user_id = body["data"]["userId"].as_str().unwrap().to_string();

    let jwt = make_jwt(&user_id, "fighter");
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "profilePhoto": "not-a-url" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_me_updates_fields() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "me3@example.com", "password": "StrongPass1!", "role": "fighter" }))
        .to_request();
    let resp = test::call_service(&app, register_req).await;
    let body: Value = test::read_body_json(resp).await;
    let user_id = body["data"]["userId"].as_str().unwrap().to_string();

    let jwt = make_jwt(&user_id, "fighter");
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "phone": "+233500000000",
            "profilePhoto": "https://cdn.example.com/photo.jpg",
            "socialLinks": {
                "instagram": "https://instagram.com/fighter",
                "youtube": "https://youtube.com/@fighter"
            }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["phone"], "+233500000000");
    assert_eq!(body["data"]["profilePhoto"], "https://cdn.example.com/photo.jpg");
    assert_eq!(body["data"]["socialLinks"]["instagram"], "https://instagram.com/fighter");
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_me_partial_update_works() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let register_req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(json!({ "email": "me4@example.com", "password": "StrongPass1!", "role": "coach" }))
        .to_request();
    let resp = test::call_service(&app, register_req).await;
    let body: Value = test::read_body_json(resp).await;
    let user_id = body["data"]["userId"].as_str().unwrap().to_string();

    let jwt = make_jwt(&user_id, "coach");
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "phone": "+44700000000" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["phone"], "+44700000000");
    // Other fields not set yet — should not appear
    assert!(body["data"]["profilePhoto"].is_null());
    teardown_db(&db).await;
}
