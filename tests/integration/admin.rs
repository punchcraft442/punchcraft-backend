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

/// Registers a user, creates and submits a profile, returns (profile_id, user_jwt).
async fn setup_submitted_profile<S, B>(app: &S, db: &mongodb::Database, email: &str) -> (String, String)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let jwt = register_and_get_jwt(app, db, email, "fighter").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Test Fighter", "weightClass": "Heavyweight" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let profile_id = body["data"]["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/fighters/{}/submit", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    test::call_service(app, req).await;

    (profile_id, jwt)
}

// ── Approve ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_approve_requires_admin_role() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, user_jwt) =
        setup_submitted_profile(&app, &db, "fighter1@example.com").await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/approve", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_approve_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/profiles/000000000000000000000000/approve")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_approve_profile_sets_approved_and_public() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, _) =
        setup_submitted_profile(&app, &db, "toapprove@example.com").await;

    let admin_jwt = make_jwt("admin_user_id", "admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/approve", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "approved");
    assert_eq!(body["data"]["visibility"], "public");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_cannot_approve_draft_profile() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = register_and_get_jwt(&app, &db, "draft@example.com", "fighter").await;

    // Create but do NOT submit
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Draft Fighter", "weightClass": "Heavyweight" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let profile_id = body["data"]["id"].as_str().unwrap();

    let admin_jwt = make_jwt("admin_id", "admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/approve", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

// ── Reject ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_reject_requires_admin_role() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, user_jwt) =
        setup_submitted_profile(&app, &db, "toreject@example.com").await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/reject", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({ "reason": "Missing info" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_reject_without_reason_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, _) =
        setup_submitted_profile(&app, &db, "noreason@example.com").await;

    let admin_jwt = make_jwt("admin_id", "admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/reject", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "reason": "" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_reject_profile_sets_rejected_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, _) =
        setup_submitted_profile(&app, &db, "rejected@example.com").await;

    let admin_jwt = make_jwt("admin_id", "admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/reject", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "reason": "Profile information is incomplete." }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "rejected");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_rejected_profile_can_be_resubmitted() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, user_jwt) =
        setup_submitted_profile(&app, &db, "resubmit@example.com").await;

    let admin_jwt = make_jwt("admin_id", "admin");

    // Reject it
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/reject", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "reason": "Needs more info." }))
        .to_request();
    test::call_service(&app, req).await;

    // User resubmits — must succeed
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/fighters/{}/submit", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "submitted");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_approved_profile_cannot_be_edited_directly() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, user_jwt) =
        setup_submitted_profile(&app, &db, "approved_edit@example.com").await;

    let admin_jwt = make_jwt("admin_id", "admin");

    // Approve it
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/approve", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    test::call_service(&app, req).await;

    // User attempts to edit — must be rejected
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/fighters/{}", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", user_jwt)))
        .set_json(json!({ "bio": "Trying to edit after approval" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    teardown_db(&db).await;
}

// ── Audit logs ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_audit_logs_requires_admin() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let fighter_jwt = register_and_get_jwt(&app, &db, "fighter_audit@example.com", "fighter").await;

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs")
        .insert_header(("Authorization", format!("Bearer {}", fighter_jwt)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_audit_logs_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_audit_log_recorded_after_approve() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, _) = setup_submitted_profile(&app, &db, "audit_approve@example.com").await;
    let admin_jwt = make_jwt("audit_admin_id", "admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/approve", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert!(!items.is_empty());

    let entry = items.iter().find(|i| i["action"] == "approve_profile").unwrap();
    assert_eq!(entry["targetId"], profile_id);
    assert_eq!(entry["targetType"], "profile");
    assert_eq!(entry["adminId"], "audit_admin_id");

    teardown_db(&db).await;
}

#[tokio::test]
async fn test_audit_logs_filter_by_action() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (profile_id, _) = setup_submitted_profile(&app, &db, "audit_filter@example.com").await;
    let admin_jwt = make_jwt("filter_admin_id", "admin");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/approve", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?action=approve_profile")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert!(items.iter().all(|i| i["action"] == "approve_profile"));

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?action=reject_profile&adminId=filter_admin_id")
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 0);

    teardown_db(&db).await;
}
