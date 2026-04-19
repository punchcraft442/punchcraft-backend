use actix_web::{http::StatusCode, test};
use serde_json::Value;

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};
use punchcraft::verification::service;

const USER_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
const ADMIN_ID: &str = "000000000000000000000000";
const FAKE_PROFILE_ID: &str = "cccccccccccccccccccccccc";

fn multipart_fields_only(boundary: &str, profile_id: &str, document_type: &str) -> String {
    format!(
        "--{b}\r\nContent-Disposition: form-data; name=\"profileId\"\r\n\r\n{p}\r\n\
         --{b}\r\nContent-Disposition: form-data; name=\"documentType\"\r\n\r\n{d}\r\n\
         --{b}--\r\n",
        b = boundary,
        p = profile_id,
        d = document_type
    )
}

// ── Submit document ───────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_document_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Send a proper multipart request so body extraction succeeds and auth check runs
    let boundary = "testboundary";
    let body = multipart_fields_only(boundary, FAKE_PROFILE_ID, "license");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .insert_header(("Content-Type", format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_document_missing_file_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Authenticated multipart with profileId and documentType but no file field
    let jwt = make_jwt(USER_A, "fighter");
    let boundary = "testboundary";
    let body = multipart_fields_only(boundary, FAKE_PROFILE_ID, "license");
    let req = test::TestRequest::post()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .insert_header(("Content-Type", format!("multipart/form-data; boundary={}", boundary)))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
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

    // Insert a document directly via service (no Cloudinary needed)
    service::submit_document(
        &db,
        FAKE_PROFILE_ID.to_string(),
        "boxing_license".to_string(),
        "https://res.cloudinary.com/test/raw/upload/v1/cert.pdf".to_string(),
    )
    .await
    .unwrap();

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

// ── Get single document (admin) ───────────────────────────────────────────────

#[tokio::test]
async fn get_document_requires_admin() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents/000000000000000000000001")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn get_document_unknown_id_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents/000000000000000000000001")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn get_document_returns_document() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let doc = service::submit_document(
        &db,
        FAKE_PROFILE_ID.to_string(),
        "passport".to_string(),
        "https://res.cloudinary.com/test/raw/upload/v1/id.jpg".to_string(),
    )
    .await
    .unwrap();
    let doc_id = doc.id.unwrap().to_hex();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/verification/documents/{}", doc_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["documentType"], "passport");
    assert_eq!(body["data"]["reviewStatus"], "pending");
    assert!(body["data"]["fileUrl"].as_str().unwrap().contains("cloudinary"));
    teardown_db(&db).await;
}

// ── List all documents (admin) ────────────────────────────────────────────────

#[tokio::test]
async fn list_all_requires_admin() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_all_returns_all_documents() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    service::submit_document(&db, FAKE_PROFILE_ID.to_string(), "passport".to_string(), "https://example.com/a.jpg".to_string()).await.unwrap();
    service::submit_document(&db, FAKE_PROFILE_ID.to_string(), "boxing_license".to_string(), "https://example.com/b.jpg".to_string()).await.unwrap();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_all_status_filter_works() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    service::submit_document(&db, FAKE_PROFILE_ID.to_string(), "passport".to_string(), "https://example.com/a.jpg".to_string()).await.unwrap();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents?status=approved")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_all_invalid_status_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::get()
        .uri("/api/v1/verification/documents?status=garbage")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
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
        .set_json(serde_json::json!({ "status": "approved" }))
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
        .set_json(serde_json::json!({ "status": "approved" }))
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

    let doc = service::submit_document(
        &db,
        FAKE_PROFILE_ID.to_string(),
        "passport".to_string(),
        "https://res.cloudinary.com/test/raw/upload/v1/id.jpg".to_string(),
    )
    .await
    .unwrap();
    let doc_id = doc.id.unwrap().to_hex();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/verification/documents/{}/review", doc_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(serde_json::json!({ "status": "approved", "adminNote": "Looks good" }))
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

    let doc = service::submit_document(
        &db,
        FAKE_PROFILE_ID.to_string(),
        "drivers_license".to_string(),
        "https://res.cloudinary.com/test/raw/upload/v1/blurry.jpg".to_string(),
    )
    .await
    .unwrap();
    let doc_id = doc.id.unwrap().to_hex();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/verification/documents/{}/review", doc_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(serde_json::json!({ "status": "rejected", "adminNote": "Image is unreadable" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"]["reviewStatus"], "rejected");
    teardown_db(&db).await;
}
