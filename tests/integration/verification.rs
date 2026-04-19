use actix_web::{http::StatusCode, test};
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};

const USER_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
const ADMIN_ID: &str = "000000000000000000000000";
const FAKE_PROFILE_ID: &str = "cccccccccccccccccccccccc";

// ── Submit document ───────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_document_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .set_json(json!({
            "profileId": FAKE_PROFILE_ID,
            "fileUrl": "https://example.com/doc.pdf",
            "documentType": "license"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_document_invalid_url_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "profileId": FAKE_PROFILE_ID,
            "fileUrl": "not-a-url",
            "documentType": "license"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_document_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "profileId": FAKE_PROFILE_ID,
            "fileUrl": "https://res.cloudinary.com/test/image/upload/v1/doc.pdf",
            "documentType": "government_id"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["reviewStatus"], "pending");
    assert_eq!(body["data"]["documentType"], "government_id");
    assert!(body["data"]["_id"]["$oid"].is_string());
    teardown_db(&db).await;
}

// ── List pending (admin) ──────────────────────────────────────────────────────

#[tokio::test]
async fn list_pending_requires_admin() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents/pending")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_pending_admin_sees_submitted_document() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let user_jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({
            "profileId": FAKE_PROFILE_ID,
            "fileUrl": "https://res.cloudinary.com/test/image/upload/v1/cert.pdf",
            "documentType": "boxing_license"
        }))
        .to_request();
    test::call_service(&app, req).await;

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents/pending")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    let items = body["data"].as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["reviewStatus"], "pending");
    teardown_db(&db).await;
}

// ── Review document ───────────────────────────────────────────────────────────

#[tokio::test]
async fn review_document_requires_admin() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents/000000000000000000000001/review")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "status": "approved" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn review_document_unknown_id_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents/000000000000000000000001/review")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "status": "approved" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn review_document_approve_sets_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let user_jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({
            "profileId": FAKE_PROFILE_ID,
            "fileUrl": "https://res.cloudinary.com/test/image/upload/v1/id.jpg",
            "documentType": "passport"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let doc_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/verification/documents/{}/review", doc_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "status": "approved", "adminNote": "Looks good" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["reviewStatus"], "approved");
    assert_eq!(body["data"]["adminNote"], "Looks good");
    teardown_db(&db).await;
}

#[tokio::test]
async fn review_document_reject_sets_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let user_jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({
            "profileId": FAKE_PROFILE_ID,
            "fileUrl": "https://res.cloudinary.com/test/image/upload/v1/blurry.jpg",
            "documentType": "drivers_license"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let doc_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/verification/documents/{}/review", doc_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "status": "rejected", "adminNote": "Image is unreadable" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"]["reviewStatus"], "rejected");
    teardown_db(&db).await;
}
