use actix_web::{http::StatusCode, test};
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};

const SENDER: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
const RECIPIENT_USER: &str = "bbbbbbbbbbbbbbbbbbbbbbbb";
const FAKE_PROFILE_ID: &str = "cccccccccccccccccccccccc";

// ── Create ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_contact_request_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/contact-requests")
        .set_json(json!({ "recipientProfileId": FAKE_PROFILE_ID, "message": "Hi there" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_contact_request_empty_message_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(SENDER, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/contact-requests")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "recipientProfileId": FAKE_PROFILE_ID, "message": "" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_contact_request_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(SENDER, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/contact-requests")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "recipientProfileId": FAKE_PROFILE_ID,
            "message": "I would like to connect with you"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["status"], "pending");
    assert!(body["data"]["_id"]["$oid"].is_string());
    teardown_db(&db).await;
}

// ── List ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_contact_requests_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/contact-requests")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_contact_requests_returns_own_only() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // SENDER sends a contact request
    let jwt_sender = make_jwt(SENDER, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/contact-requests")
        .insert_header(("Authorization", format!("Bearer {}", jwt_sender)))
        .set_json(json!({
            "recipientProfileId": FAKE_PROFILE_ID,
            "message": "Reaching out to you"
        }))
        .to_request();
    test::call_service(&app, req).await;

    // SENDER should see it
    let req = test::TestRequest::get()
        .uri("/api/v1/contact-requests")
        .insert_header(("Authorization", format!("Bearer {}", jwt_sender)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);

    // RECIPIENT_USER should NOT see SENDER's request
    let jwt_recipient = make_jwt(RECIPIENT_USER, "gym");
    let req = test::TestRequest::get()
        .uri("/api/v1/contact-requests")
        .insert_header(("Authorization", format!("Bearer {}", jwt_recipient)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);

    teardown_db(&db).await;
}

// ── Update status ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_contact_request_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::patch()
        .uri("/api/v1/contact-requests/000000000000000000000001")
        .set_json(json!({ "status": "accepted" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_contact_request_unknown_id_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(SENDER, "fighter");
    let req = test::TestRequest::patch()
        .uri("/api/v1/contact-requests/000000000000000000000001")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "status": "accepted" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_contact_request_accepted() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(SENDER, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/contact-requests")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "recipientProfileId": FAKE_PROFILE_ID,
            "message": "Would love to train together"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let cr_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/contact-requests/{}", cr_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "status": "accepted" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "accepted");
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_contact_request_declined() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(SENDER, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/contact-requests")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "recipientProfileId": FAKE_PROFILE_ID,
            "message": "Want to spar sometime"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let cr_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/contact-requests/{}", cr_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "status": "declined" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"]["status"], "declined");
    teardown_db(&db).await;
}
