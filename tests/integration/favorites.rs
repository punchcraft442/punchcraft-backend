use actix_web::{http::StatusCode, test};
use serde_json::{json, Value};

use crate::build_app;
use crate::common::{init, make_jwt, setup_db, teardown_db};

const USER_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
const USER_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbb";
const PROFILE_X: &str = "cccccccccccccccccccccccc";
const PROFILE_Y: &str = "dddddddddddddddddddddddd";

// ── Add favorite ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn add_favorite_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::post()
        .uri("/api/v1/favorites")
        .set_json(json!({ "profileId": PROFILE_X }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn add_favorite_empty_profile_id_returns_400() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "profileId": "" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    teardown_db(&db).await;
}

#[tokio::test]
async fn add_favorite_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "profileId": PROFILE_X }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body["data"]["_id"]["$oid"].is_string());
    assert!(body["data"]["profileId"]["$oid"].is_string());
    teardown_db(&db).await;
}

#[tokio::test]
async fn add_favorite_duplicate_returns_409() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let payload = json!({ "profileId": PROFILE_X });

    // First add
    test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/favorites")
            .insert_header(("Authorization", format!("Bearer {}", jwt)))
            .set_json(payload.clone())
            .to_request(),
    )
    .await;

    // Second add — same user, same profile
    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/favorites")
            .insert_header(("Authorization", format!("Bearer {}", jwt)))
            .set_json(payload)
            .to_request(),
    )
    .await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    teardown_db(&db).await;
}

// ── List favorites ────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_favorites_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::get()
        .uri("/api/v1/favorites")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_favorites_empty_for_new_user() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::get()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_favorites_shows_added_items() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");

    // Add two favorites
    for pid in [PROFILE_X, PROFILE_Y] {
        test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/v1/favorites")
                .insert_header(("Authorization", format!("Bearer {}", jwt)))
                .set_json(json!({ "profileId": pid }))
                .to_request(),
        )
        .await;
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    teardown_db(&db).await;
}

#[tokio::test]
async fn list_favorites_isolated_per_user() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // USER_A adds a favorite
    let jwt_a = make_jwt(USER_A, "fighter");
    test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/favorites")
            .insert_header(("Authorization", format!("Bearer {}", jwt_a)))
            .set_json(json!({ "profileId": PROFILE_X }))
            .to_request(),
    )
    .await;

    // USER_B should see 0
    let jwt_b = make_jwt(USER_B, "gym");
    let req = test::TestRequest::get()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt_b)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    teardown_db(&db).await;
}

// ── Remove favorite ───────────────────────────────────────────────────────────

#[tokio::test]
async fn remove_favorite_requires_auth() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let req = test::TestRequest::delete()
        .uri("/api/v1/favorites/000000000000000000000001")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    teardown_db(&db).await;
}

#[tokio::test]
async fn remove_favorite_unknown_id_returns_404() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::delete()
        .uri("/api/v1/favorites/000000000000000000000001")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    teardown_db(&db).await;
}

#[tokio::test]
async fn remove_favorite_success() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    let jwt = make_jwt(USER_A, "fighter");

    // Add
    let req = test::TestRequest::post()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .set_json(json!({ "profileId": PROFILE_X }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let fav_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    // Remove
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/favorites/{}", fav_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Confirm list is empty
    let req = test::TestRequest::get()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);

    teardown_db(&db).await;
}

#[tokio::test]
async fn remove_favorite_wrong_user_returns_403() {
    init();
    let db = setup_db().await;
    let app = build_app!(db);

    // USER_A adds a favorite
    let jwt_a = make_jwt(USER_A, "fighter");
    let req = test::TestRequest::post()
        .uri("/api/v1/favorites")
        .insert_header(("Authorization", format!("Bearer {}", jwt_a)))
        .set_json(json!({ "profileId": PROFILE_X }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    let fav_id = body["data"]["_id"]["$oid"].as_str().unwrap();

    // USER_B tries to delete USER_A's favorite — must be forbidden
    let jwt_b = make_jwt(USER_B, "gym");
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/favorites/{}", fav_id))
        .insert_header(("Authorization", format!("Bearer {}", jwt_b)))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    teardown_db(&db).await;
}
