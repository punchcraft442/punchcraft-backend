use actix_web::{body::MessageBody, dev::{Service, ServiceResponse}, http::StatusCode, test};
use actix_http::Request;
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};

// ── Pre-defined user IDs (valid 24-char hex ObjectIds) ────────────────────────

const FIGHTER_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
const FIGHTER_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbb";
const GYM_A: &str = "cccccccccccccccccccccccc";
const GYM_B: &str = "dddddddddddddddddddddddd";
const COACH_A: &str = "eeeeeeeeeeeeeeeeeeeeeeee";
const OFFICIAL_A: &str = "ffffffffffffffffffffffff";
const PROMOTER_A: &str = "111111111111111111111111";
const MATCHMAKER_A: &str = "222222222222222222222222";
const FAN_A: &str = "333333333333333333333333";
const ADMIN_USER: &str = "000000000000000000000000";

// ── HTTP helpers ──────────────────────────────────────────────────────────────

async fn create_fighter<S, B>(app: &S, user_id: &str) -> (StatusCode, Value)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let jwt = make_jwt(user_id, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Test Fighter", "weightClass": "Heavyweight" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    (status, test::read_body_json(resp).await)
}

async fn create_gym<S, B>(app: &S, user_id: &str) -> (StatusCode, Value)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let jwt = make_jwt(user_id, "gym");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/gyms")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "name": "Test Gym" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    (status, test::read_body_json(resp).await)
}

async fn submit_fighter<S, B>(app: &S, user_id: &str, profile_id: &str) -> StatusCode
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let jwt = make_jwt(user_id, "fighter");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/fighters/{}/submit", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    test::call_service(app, req).await.status()
}

async fn admin_approve<S, B>(app: &S, profile_id: &str)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let jwt = make_jwt(ADMIN_USER, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/approve", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    test::call_service(app, req).await;
}

// ═════════════════════════════════════════════════════════════════════════════
// FIGHTER
// ═════════════════════════════════════════════════════════════════════════════

// ── Create ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_fighter_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .set_json(json!({ "fullName": "Test Fighter", "weightClass": "Heavyweight" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_fighter_wrong_role_returns_403() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // JWT claims role = "gym", but endpoint expects "fighter"
    let jwt = make_jwt(GYM_A, "gym");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Sneaky Gym", "weightClass": "Heavyweight" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_fighter_success_returns_201_draft_private() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (status, body) = create_fighter(&app, FIGHTER_A).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["status"], "draft");
    assert_eq!(body["data"]["visibility"], "private");
    assert_eq!(body["data"]["verificationTier"], "unverified");
    assert_eq!(body["data"]["fullName"], "Test Fighter");
    assert_eq!(body["data"]["weightClass"], "Heavyweight");
    assert_eq!(body["data"]["role"], "fighter");
    assert!(body["data"]["id"].is_string());
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_fighter_duplicate_returns_409() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    create_fighter(&app, FIGHTER_A).await;
    let (status, _) = create_fighter(&app, FIGHTER_A).await;

    assert_eq!(status, StatusCode::CONFLICT);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_fighter_missing_required_field_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FIGHTER_A, "fighter");
    // weightClass is required — omit it
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "No Weight Class" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

// ── Get / Visibility ──────────────────────────────────────────────────────────

#[tokio::test]
async fn get_fighter_draft_not_visible_to_unauthenticated() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    // No JWT — draft is invisible
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/fighters/{}", id))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn get_fighter_owner_can_see_own_draft() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/fighters/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    teardown_db(&db).await;
}

#[tokio::test]
async fn get_fighter_approved_profile_visible_to_public() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    submit_fighter(&app, FIGHTER_A, id).await;
    admin_approve(&app, id).await;

    // No JWT — approved + public is visible
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/fighters/{}", id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"]["status"], "approved");
    assert_eq!(body["data"]["visibility"], "public");
    teardown_db(&db).await;
}

#[tokio::test]
async fn get_fighter_unknown_id_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/fighters/000000000000000000000001")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

// ── Update ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_fighter_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/fighters/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "bio": "Updated bio", "weightClass": "Cruiserweight" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["bio"], "Updated bio");
    assert_eq!(body["data"]["weightClass"], "Cruiserweight");
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_fighter_requires_ownership() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    // FIGHTER_B tries to update FIGHTER_A's profile
    let jwt = make_jwt(FIGHTER_B, "fighter");
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/fighters/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "bio": "Hijacked bio" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_approved_fighter_is_rejected() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    submit_fighter(&app, FIGHTER_A, id).await;
    admin_approve(&app, id).await;

    // Owner tries to edit an approved profile
    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/fighters/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "bio": "Sneaky edit" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

// ── Submit ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_fighter_changes_status_to_submitted() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let status = submit_fighter(&app, FIGHTER_A, id).await;
    assert_eq!(status, StatusCode::OK);

    // Fetch and verify status
    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/fighters/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "submitted");
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_fighter_already_submitted_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    submit_fighter(&app, FIGHTER_A, id).await;
    let status = submit_fighter(&app, FIGHTER_A, id).await; // second submit

    assert_eq!(status, StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_fighter_requires_ownership() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let status = submit_fighter(&app, FIGHTER_B, id).await; // wrong user

    assert_eq!(status, StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn rejected_fighter_can_be_resubmitted() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    submit_fighter(&app, FIGHTER_A, id).await;

    // Admin rejects
    let admin_jwt = make_jwt(ADMIN_USER, "admin");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/profiles/{}/reject", id))
        .insert_header(("Authorization", format!("Bearer {}", admin_jwt)))
        .set_json(json!({ "reason": "Incomplete details." }))
        .to_request();
    test::call_service(&app, req).await;

    // User resubmits after rejection
    let status = submit_fighter(&app, FIGHTER_A, id).await;
    assert_eq!(status, StatusCode::OK);
    teardown_db(&db).await;
}

// ── Fight history ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn add_fight_history_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/fighters/{}/fight-history", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "opponentName": "Rival Boxer",
            "eventName": "Championship Night",
            "eventDate": "2024-06-15",
            "result": "win",
            "method": "KO",
            "round": 3
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    let history = body["data"]["fightHistory"].as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["opponentName"], "Rival Boxer");
    assert_eq!(history[0]["result"], "win");
    assert!(history[0]["id"].is_string()); // UUID assigned
    teardown_db(&db).await;
}

#[tokio::test]
async fn add_fight_history_requires_ownership() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(FIGHTER_B, "fighter"); // wrong user
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/fighters/{}/fight-history", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "opponentName": "Ghost",
            "eventName": "Fake Night",
            "eventDate": "2024-01-01",
            "result": "win"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn delete_fight_history_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let profile_id = body["data"]["id"].as_str().unwrap();

    // Add an entry
    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/fighters/{}/fight-history", profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "opponentName": "To Be Deleted",
            "eventName": "Event",
            "eventDate": "2024-03-01",
            "result": "loss"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    let fight_id = body["data"]["fightHistory"][0]["id"].as_str().unwrap();

    // Delete the entry
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/profiles/fighters/{}/fight-history/{}", profile_id, fight_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    teardown_db(&db).await;
}

// ── List / Directory ──────────────────────────────────────────────────────────

#[tokio::test]
async fn list_fighters_returns_empty_when_none_approved() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Create + submit but not approve — should not appear in list
    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id = body["data"]["id"].as_str().unwrap();
    submit_fighter(&app, FIGHTER_A, id).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/fighters")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_fighters_shows_only_approved_and_searchable() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // Fighter A: approve → appears
    let (_, body) = create_fighter(&app, FIGHTER_A).await;
    let id_a = body["data"]["id"].as_str().unwrap().to_string();
    submit_fighter(&app, FIGHTER_A, &id_a).await;
    admin_approve(&app, &id_a).await;

    // Fighter B: draft only → does not appear
    create_fighter(&app, FIGHTER_B).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/fighters")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], id_a.as_str());
    assert_eq!(body["data"]["pagination"]["totalItems"], 1);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_fighters_pagination_meta_is_present() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/fighters?page=1&limit=10")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["pagination"]["page"].is_number());
    assert!(body["data"]["pagination"]["limit"].is_number());
    assert!(body["data"]["pagination"]["totalItems"].is_number());
    assert!(body["data"]["pagination"]["totalPages"].is_number());
    teardown_db(&db).await;
}

// ═════════════════════════════════════════════════════════════════════════════
// GYM
// ═════════════════════════════════════════════════════════════════════════════

// ── Create ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_gym_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/gyms")
        .set_json(json!({ "name": "Test Gym" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_gym_wrong_role_returns_403() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FIGHTER_A, "fighter"); // fighter cannot create gym profile
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/gyms")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "name": "Sneaky Gym" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_gym_success_returns_201() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (status, body) = create_gym(&app, GYM_A).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["role"], "gym");
    assert_eq!(body["data"]["status"], "draft");
    assert_eq!(body["data"]["visibility"], "private");
    assert_eq!(body["data"]["name"], "Test Gym");
    assert!(body["data"]["linkedCoachIds"].as_array().unwrap().is_empty());
    assert!(body["data"]["rosterFighterIds"].as_array().unwrap().is_empty());
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_gym_duplicate_returns_409() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    create_gym(&app, GYM_A).await;
    let (status, _) = create_gym(&app, GYM_A).await;

    assert_eq!(status, StatusCode::CONFLICT);
    teardown_db(&db).await;
}

// ── Update ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_gym_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(GYM_A, "gym");
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/gyms/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "name": "Updated Gym Name",
            "address": "123 Boxing Lane",
            "services": ["boxing", "sparring"]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["name"], "Updated Gym Name");
    assert_eq!(body["data"]["address"], "123 Boxing Lane");
    teardown_db(&db).await;
}

#[tokio::test]
async fn update_gym_requires_ownership() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(GYM_B, "gym"); // different gym user
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/gyms/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "name": "Stolen Gym" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

// ── Submit ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_gym_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(GYM_A, "gym");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/submit", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "submitted");
    teardown_db(&db).await;
}

// ── Link / Unlink coaches ─────────────────────────────────────────────────────

#[tokio::test]
async fn gym_link_coach_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let gym_id = body["data"]["id"].as_str().unwrap();

    // Create a real coach profile so the service can validate it exists
    let coach_jwt = make_jwt(COACH_A, "coach");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", coach_jwt)))
        .set_json(json!({ "fullName": "Test Coach", "experienceSummary": "5 years", "specialties": ["boxing"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let coach_body: Value = test::read_body_json(resp).await;
    let coach_profile_id = coach_body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(GYM_A, "gym");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/coaches/{}", gym_id, coach_profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    teardown_db(&db).await;
}

#[tokio::test]
async fn gym_link_coach_requires_ownership() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let gym_id = body["data"]["id"].as_str().unwrap();

    let jwt = make_jwt(GYM_B, "gym"); // wrong owner
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/coaches/eeeeeeeeeeeeeeeeeeeeeeee", gym_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn gym_unlink_coach_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let gym_id = body["data"]["id"].as_str().unwrap();
    let jwt = make_jwt(GYM_A, "gym");
    let coach_id = "eeeeeeeeeeeeeeeeeeeeeeee";

    // Link first
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/coaches/{}", gym_id, coach_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    test::call_service(&app, req).await;

    // Then unlink
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/profiles/gyms/{}/coaches/{}", gym_id, coach_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    teardown_db(&db).await;
}

#[tokio::test]
async fn gym_link_fighter_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let gym_id = body["data"]["id"].as_str().unwrap();
    let jwt = make_jwt(GYM_A, "gym");

    // Create a real fighter profile so the service can validate it exists
    let (_, fighter_body) = create_fighter(&app, FIGHTER_A).await;
    let fighter_profile_id = fighter_body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/fighters/{}", gym_id, fighter_profile_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    teardown_db(&db).await;
}

#[tokio::test]
async fn gym_unlink_fighter_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let (_, body) = create_gym(&app, GYM_A).await;
    let gym_id = body["data"]["id"].as_str().unwrap();
    let jwt = make_jwt(GYM_A, "gym");
    let fighter_id = "aaaaaaaaaaaaaaaaaaaaaaaa";

    // Link
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/fighters/{}", gym_id, fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    test::call_service(&app, req).await;

    // Unlink
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/profiles/gyms/{}/fighters/{}", gym_id, fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    teardown_db(&db).await;
}

// ═════════════════════════════════════════════════════════════════════════════
// COACH
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_coach_requires_coach_role() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FIGHTER_A, "fighter"); // wrong role
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Not A Coach" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_coach_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(COACH_A, "coach");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "fullName": "Mike Trainer",
            "experienceSummary": "10 years coaching",
            "specialties": ["boxing", "footwork"]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["fullName"], "Mike Trainer");
    assert_eq!(body["data"]["role"], "coach");
    assert_eq!(body["data"]["status"], "draft");
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_coach_changes_status_to_submitted() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(COACH_A, "coach");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Submit Coach" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    let id = body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/coaches/{}/submit", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "submitted");
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_coach_duplicate_returns_409() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(COACH_A, "coach");
    let body = json!({ "fullName": "Dup Coach" });
    for _ in 0..2 {
        test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/v1/profiles/coaches")
                .insert_header(("Authorization", format!("Bearer {}", jwt)))
                .set_json(body.clone())
                .to_request(),
        )
        .await;
    }
    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/profiles/coaches")
            .insert_header(("Authorization", format!("Bearer {}", jwt)))
            .set_json(body)
            .to_request(),
    )
    .await;

    // The second call already returned 409; we just confirm it's still 409
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    teardown_db(&db).await;
}

// ═════════════════════════════════════════════════════════════════════════════
// OFFICIAL
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_official_requires_official_role() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/officials")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Not Official", "officialType": ["referee"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_official_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(OFFICIAL_A, "official");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/officials")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "fullName": "James Referee",
            "officialType": ["referee"],
            "experienceYears": 5
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["fullName"], "James Referee");
    assert_eq!(body["data"]["role"], "official");
    assert_eq!(body["data"]["status"], "draft");
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_official_changes_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(OFFICIAL_A, "official");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/officials")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Joe Judge", "officialType": ["judge"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    let id = body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/officials/{}/submit", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "submitted");
    teardown_db(&db).await;
}

// ═════════════════════════════════════════════════════════════════════════════
// PROMOTER
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_promoter_requires_promoter_role() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/promoters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "organizationName": "Not Promoter" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_promoter_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(PROMOTER_A, "promoter");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/promoters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "organizationName": "Top Rank Boxing",
            "coverageAreas": ["US", "UK"]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["organizationName"], "Top Rank Boxing");
    assert_eq!(body["data"]["role"], "promoter");
    assert_eq!(body["data"]["status"], "draft");
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_promoter_changes_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(PROMOTER_A, "promoter");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/promoters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "organizationName": "Big Event Co" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    let id = body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/promoters/{}/submit", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "submitted");
    teardown_db(&db).await;
}

// ═════════════════════════════════════════════════════════════════════════════
// MATCHMAKER
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_matchmaker_requires_matchmaker_role() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/matchmakers")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Not A Matchmaker" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_matchmaker_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(MATCHMAKER_A, "matchmaker");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/matchmakers")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "fullName": "Bob Matchmaker",
            "regionsServed": ["Northeast US"],
            "weightClassesFocus": ["Heavyweight", "Cruiserweight"]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["fullName"], "Bob Matchmaker");
    assert_eq!(body["data"]["role"], "matchmaker");
    assert_eq!(body["data"]["status"], "draft");
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_matchmaker_changes_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(MATCHMAKER_A, "matchmaker");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/matchmakers")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "Submit Maker" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    let id = body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/matchmakers/{}/submit", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "submitted");
    teardown_db(&db).await;
}

// ═════════════════════════════════════════════════════════════════════════════
// FAN
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_fan_requires_fan_role() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FIGHTER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fans")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "displayName": "Not A Fan" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_fan_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FAN_A, "fan");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fans")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({
            "displayName": "BoxingFan99",
            "favouriteWeightClass": "Heavyweight"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["displayName"], "BoxingFan99");
    assert_eq!(body["data"]["role"], "fan");
    assert_eq!(body["data"]["status"], "draft");
    teardown_db(&db).await;
}

#[tokio::test]
async fn submit_fan_changes_status() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FAN_A, "fan");
    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/fans")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "displayName": "SubmitFan" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    let id = body["data"]["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/fans/{}/submit", id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "submitted");
    teardown_db(&db).await;
}

#[tokio::test]
async fn create_fan_duplicate_returns_409() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(FAN_A, "fan");
    let body = json!({ "displayName": "DupFan" });

    // First create
    test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/profiles/fans")
            .insert_header(("Authorization", format!("Bearer {}", jwt)))
            .set_json(body.clone())
            .to_request(),
    )
    .await;

    // Second create — must conflict
    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/profiles/fans")
            .insert_header(("Authorization", format!("Bearer {}", jwt)))
            .set_json(body)
            .to_request(),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    teardown_db(&db).await;
}

// =============================================================================
// BIDIRECTIONAL LINK TESTS
// =============================================================================

#[tokio::test]
async fn gym_link_coach_updates_coach_linked_gym_ids() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let (_, gym_body) = create_gym(&app, GYM_A).await;
    let gym_id = gym_body["data"]["id"].as_str().unwrap();
    let coach_jwt = make_jwt(COACH_A, "coach");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", coach_jwt)))
        .set_json(json!({ "fullName": "Coach Bidir" }))
        .to_request()).await;
    let coach_body: Value = test::read_body_json(resp).await;
    let coach_id = coach_body["data"]["id"].as_str().unwrap();
    let gym_jwt = make_jwt(GYM_A, "gym");
    test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/coaches/{}", gym_id, coach_id))
        .insert_header(("Authorization", format!("Bearer {}", gym_jwt)))
        .to_request()).await;
    let resp = test::call_service(&app, test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/coaches/{}", coach_id))
        .insert_header(("Authorization", format!("Bearer {}", coach_jwt)))
        .to_request()).await;
    let body: Value = test::read_body_json(resp).await;
    let linked_gyms = body["data"]["linkedGymIds"].as_array().unwrap();
    assert!(linked_gyms.iter().any(|v| v == gym_id));
    teardown_db(&db).await;
}

#[tokio::test]
async fn gym_unlink_coach_removes_from_coach_linked_gym_ids() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let (_, gym_body) = create_gym(&app, GYM_A).await;
    let gym_id = gym_body["data"]["id"].as_str().unwrap();
    let gym_jwt = make_jwt(GYM_A, "gym");
    let coach_jwt = make_jwt(COACH_A, "coach");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", coach_jwt)))
        .set_json(json!({ "fullName": "UnlinkCoach" }))
        .to_request()).await;
    let coach_id = test::read_body_json::<Value, _>(resp).await["data"]["id"].as_str().unwrap().to_string();
    test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/coaches/{}", gym_id, coach_id))
        .insert_header(("Authorization", format!("Bearer {}", gym_jwt)))
        .to_request()).await;
    test::call_service(&app, test::TestRequest::delete()
        .uri(&format!("/api/v1/profiles/gyms/{}/coaches/{}", gym_id, coach_id))
        .insert_header(("Authorization", format!("Bearer {}", gym_jwt)))
        .to_request()).await;
    let resp = test::call_service(&app, test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/coaches/{}", coach_id))
        .insert_header(("Authorization", format!("Bearer {}", coach_jwt)))
        .to_request()).await;
    let body: Value = test::read_body_json(resp).await;
    let linked_gyms = body["data"]["linkedGymIds"].as_array().unwrap();
    assert!(!linked_gyms.iter().any(|v| v == gym_id));
    teardown_db(&db).await;
}

#[tokio::test]
async fn gym_link_fighter_updates_fighter_linked_gym_id() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let (_, gym_body) = create_gym(&app, GYM_A).await;
    let gym_id = gym_body["data"]["id"].as_str().unwrap();
    let gym_jwt = make_jwt(GYM_A, "gym");
    let (_, fighter_body) = create_fighter(&app, FIGHTER_A).await;
    let fighter_id = fighter_body["data"]["id"].as_str().unwrap();
    test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/fighters/{}", gym_id, fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", gym_jwt)))
        .to_request()).await;
    let fighter_jwt = make_jwt(FIGHTER_A, "fighter");
    let resp = test::call_service(&app, test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/fighters/{}", fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", fighter_jwt)))
        .to_request()).await;
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["linkedGymId"], gym_id);
    teardown_db(&db).await;
}

#[tokio::test]
async fn gym_unlink_fighter_clears_fighter_linked_gym_id() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let (_, gym_body) = create_gym(&app, GYM_A).await;
    let gym_id = gym_body["data"]["id"].as_str().unwrap();
    let gym_jwt = make_jwt(GYM_A, "gym");
    let (_, fighter_body) = create_fighter(&app, FIGHTER_A).await;
    let fighter_id = fighter_body["data"]["id"].as_str().unwrap();
    test::call_service(&app, test::TestRequest::post()
        .uri(&format!("/api/v1/profiles/gyms/{}/fighters/{}", gym_id, fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", gym_jwt)))
        .to_request()).await;
    test::call_service(&app, test::TestRequest::delete()
        .uri(&format!("/api/v1/profiles/gyms/{}/fighters/{}", gym_id, fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", gym_jwt)))
        .to_request()).await;
    let fighter_jwt = make_jwt(FIGHTER_A, "fighter");
    let resp = test::call_service(&app, test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/fighters/{}", fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", fighter_jwt)))
        .to_request()).await;
    let body: Value = test::read_body_json(resp).await;
    assert!(body["data"]["linkedGymId"].is_null());
    teardown_db(&db).await;
}

#[tokio::test]
async fn fighter_self_link_coach_updates_coach_associated_fighters() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let coach_jwt = make_jwt(COACH_A, "coach");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", coach_jwt)))
        .set_json(json!({ "fullName": "LinkCoach" }))
        .to_request()).await;
    let coach_id = test::read_body_json::<Value, _>(resp).await["data"]["id"].as_str().unwrap().to_string();
    let fighter_jwt = make_jwt(FIGHTER_A, "fighter");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", fighter_jwt)))
        .set_json(json!({ "fullName": "FighterSelfLink", "weightClass": "Heavyweight", "linkedCoachId": coach_id }))
        .to_request()).await;
    let fighter_id = test::read_body_json::<Value, _>(resp).await["data"]["id"].as_str().unwrap().to_string();
    let resp = test::call_service(&app, test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/coaches/{}", coach_id))
        .insert_header(("Authorization", format!("Bearer {}", coach_jwt)))
        .to_request()).await;
    let body: Value = test::read_body_json(resp).await;
    let assoc = body["data"]["associatedFighterIds"].as_array().unwrap();
    assert!(assoc.iter().any(|v| v == fighter_id.as_str()));
    teardown_db(&db).await;
}

#[tokio::test]
async fn fighter_update_coach_syncs_associated_fighters() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let coach_a_jwt = make_jwt(COACH_A, "coach");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", coach_a_jwt)))
        .set_json(json!({ "fullName": "CoachA" }))
        .to_request()).await;
    let coach_a_id = test::read_body_json::<Value, _>(resp).await["data"]["id"].as_str().unwrap().to_string();
    let coach_b_jwt = make_jwt(FIGHTER_B, "coach");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/coaches")
        .insert_header(("Authorization", format!("Bearer {}", coach_b_jwt)))
        .set_json(json!({ "fullName": "CoachB" }))
        .to_request()).await;
    let coach_b_id = test::read_body_json::<Value, _>(resp).await["data"]["id"].as_str().unwrap().to_string();
    let fighter_jwt = make_jwt(FIGHTER_A, "fighter");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", fighter_jwt)))
        .set_json(json!({ "fullName": "SwitchFighter", "weightClass": "Lightweight", "linkedCoachId": coach_a_id }))
        .to_request()).await;
    let fighter_id = test::read_body_json::<Value, _>(resp).await["data"]["id"].as_str().unwrap().to_string();
    let resp = test::call_service(&app, test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/fighters/{}", fighter_id))
        .insert_header(("Authorization", format!("Bearer {}", fighter_jwt)))
        .set_json(json!({ "linkedCoachId": coach_b_id }))
        .to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = test::call_service(&app, test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/coaches/{}", coach_a_id))
        .insert_header(("Authorization", format!("Bearer {}", coach_a_jwt)))
        .to_request()).await;
    let assoc_a = test::read_body_json::<Value, _>(resp).await["data"]["associatedFighterIds"]
        .as_array().unwrap().to_owned();
    assert!(!assoc_a.iter().any(|v| v == fighter_id.as_str()));
    let resp = test::call_service(&app, test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/coaches/{}", coach_b_id))
        .insert_header(("Authorization", format!("Bearer {}", coach_b_jwt)))
        .to_request()).await;
    let assoc_b = test::read_body_json::<Value, _>(resp).await["data"]["associatedFighterIds"]
        .as_array().unwrap().to_owned();
    assert!(assoc_b.iter().any(|v| v == fighter_id.as_str()));
    teardown_db(&db).await;
}

#[tokio::test]
async fn fighter_create_with_invalid_linked_gym_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let jwt = make_jwt(FIGHTER_A, "fighter");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "BadGymFighter", "weightClass": "Heavyweight", "linkedGymId": "aaaaaaaaaaaaaaaaaaaaaaaa" }))
        .to_request()).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn fighter_create_with_invalid_linked_coach_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);
    let jwt = make_jwt(FIGHTER_A, "fighter");
    let resp = test::call_service(&app, test::TestRequest::post()
        .uri("/api/v1/profiles/fighters")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "fullName": "BadCoachFighter", "weightClass": "Heavyweight", "linkedCoachId": "aaaaaaaaaaaaaaaaaaaaaaaa" }))
        .to_request()).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}
