use actix_web::{test::TestRequest, HttpMessage};
use punchcraft::common::{errors::AppError, middleware::{require_admin, require_auth, Claims}};

fn request_with_claims(role: &str) -> actix_web::HttpRequest {
    let req = TestRequest::default().to_http_request();
    req.extensions_mut().insert(Claims {
        sub: "user123".to_string(),
        role: role.to_string(),
        exp: 9999999999,
    });
    req
}

fn request_without_claims() -> actix_web::HttpRequest {
    TestRequest::default().to_http_request()
}

// ── require_auth ──────────────────────────────────────────────────────────────

#[test]
fn require_auth_returns_claims_when_present() {
    let req = request_with_claims("fighter");
    let result = require_auth(&req);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().role, "fighter");
}

#[test]
fn require_auth_returns_unauthorized_when_missing() {
    let req = request_without_claims();
    let result = require_auth(&req);
    assert!(matches!(result, Err(AppError::Unauthorized)));
}

// ── require_admin ─────────────────────────────────────────────────────────────

#[test]
fn require_admin_passes_for_admin_role() {
    let req = request_with_claims("admin");
    assert!(require_admin(&req).is_ok());
}

#[test]
fn require_admin_passes_for_super_admin_role() {
    let req = request_with_claims("super_admin");
    assert!(require_admin(&req).is_ok());
}

#[test]
fn require_admin_forbidden_for_fighter_role() {
    let req = request_with_claims("fighter");
    assert!(matches!(require_admin(&req), Err(AppError::Forbidden)));
}

#[test]
fn require_admin_forbidden_for_gym_role() {
    let req = request_with_claims("gym");
    assert!(matches!(require_admin(&req), Err(AppError::Forbidden)));
}

#[test]
fn require_admin_unauthorized_when_no_claims() {
    let req = request_without_claims();
    assert!(matches!(require_admin(&req), Err(AppError::Unauthorized)));
}
