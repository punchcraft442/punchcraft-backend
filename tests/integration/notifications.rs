use actix_web::{http::StatusCode, test};
use chrono::Utc;
use serde_json::Value;

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};

const USER_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
const USER_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbb";

// ── Auth guard ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_notifications_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

// ── Empty list ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_notifications_returns_empty_array_for_new_user() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    teardown_db(&db).await;
}

// ── Mark read ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn mark_read_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::patch()
        .uri("/api/v1/notifications/000000000000000000000001/read")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn mark_read_unknown_id_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::patch()
        .uri("/api/v1/notifications/000000000000000000000001/read")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

// ── Isolation — users see only their own ─────────────────────────────────────

#[tokio::test]
async fn users_see_only_their_own_notifications() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Seed a notification for USER_A directly in the DB
    use mongodb::bson::{doc, oid::ObjectId};
    use chrono::Utc;
    let col = db.collection::<mongodb::bson::Document>("notifications");
    let oid_a = ObjectId::parse_str(USER_A).unwrap();
    col.insert_one(doc! {
        "userId": oid_a,
        "title": "Test",
        "message": "Hello A",
        "isRead": false,
        "createdAt": Utc::now().to_rfc3339(),
    })
    .await
    .unwrap();

    // USER_B must get 0
    let jwt_b = make_jwt(USER_B, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header(("Authorization", format!("Bearer {}", jwt_b)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);

    // USER_A must get 1
    let jwt_a = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header(("Authorization", format!("Bearer {}", jwt_a)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);

    teardown_db(&db).await;
}
