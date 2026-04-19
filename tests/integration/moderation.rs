use actix_web::{http::StatusCode, test};
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};

const USER_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
const ADMIN_ID: &str = "000000000000000000000000";
const FAKE_PROFILE_ID: &str = "cccccccccccccccccccccccc";

// ── Create report ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_report_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/reports")
        .set_json(json!({ "profileId": FAKE_PROFILE_ID, "reason": "Spam content" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_report_missing_reason_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/reports")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "profileId": FAKE_PROFILE_ID, "reason": "ab" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_report_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/reports")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "profileId": FAKE_PROFILE_ID, "reason": "This profile contains offensive content" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["status"], "open");
    assert!(body["data"]["_id"]["$oid"].is_string());
    teardown_db(&db).await;
}

// ── Admin list reports ────────────────────────────────────────────────────────

#[tokio::test]
async fn list_reports_requires_admin() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/reports/admin")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_reports_admin_returns_created_report() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Create a report
    let user_jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/reports")
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({ "profileId": FAKE_PROFILE_ID, "reason": "Inappropriate profile content" }))
        .to_request();
    test::call_service(&app, req).await;

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::get()
        .uri("/api/v1/reports/admin")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    let items = body["data"].as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["status"], "open");
    teardown_db(&db).await;
}

// ── Admin decide report ───────────────────────────────────────────────────────

#[tokio::test]
async fn decide_report_requires_admin() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/reports/admin/000000000000000000000001/decision")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "status": "reviewed", "adminNote": "Handled" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn decide_report_unknown_id_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri("/api/v1/reports/admin/000000000000000000000001/decision")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "status": "reviewed" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn decide_report_sets_reviewed_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Create a report
    let user_jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/reports")
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({ "profileId": FAKE_PROFILE_ID, "reason": "Violates community guidelines" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let report_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/reports/admin/{}/decision", report_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "status": "reviewed", "adminNote": "Confirmed and actioned" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "reviewed");
    assert_eq!(body["data"]["adminNote"], "Confirmed and actioned");
    teardown_db(&db).await;
}

#[tokio::test]
async fn decide_report_can_dismiss() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let user_jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/reports")
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({ "profileId": FAKE_PROFILE_ID, "reason": "Seems suspicious to me" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let report_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    let admin_jwt = make_jwt(ADMIN_ID, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/reports/admin/{}/decision", report_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "status": "dismissed" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"]["status"], "dismissed");
    teardown_db(&db).await;
}
