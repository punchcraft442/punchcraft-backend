use punchcraft::auth::models::{
    ChangePasswordRequest, ForgotPasswordRequest, LoginRequest, RegisterRequest,
    ResetPasswordRequest,
};
use validator::Validate;

// ── RegisterRequest ───────────────────────────────────────────────────────────

#[test]
fn register_valid_passes() {
    let req = RegisterRequest {
        email: "kwame@example.com".to_string(),
        password: "StrongPass1!".to_string(),
        role: punchcraft::auth::models::UserRole::Fighter,
    };
    assert!(req.validate().is_ok());
}

#[test]
fn register_invalid_email_fails() {
    let req = RegisterRequest {
        email: "not-an-email".to_string(),
        password: "StrongPass1!".to_string(),
        role: punchcraft::auth::models::UserRole::Fighter,
    };
    assert!(req.validate().is_err());
}

#[test]
fn register_short_password_fails() {
    let req = RegisterRequest {
        email: "kwame@example.com".to_string(),
        password: "short".to_string(),
        role: punchcraft::auth::models::UserRole::Fighter,
    };
    assert!(req.validate().is_err());
}

// ── LoginRequest ─────────────────────────────────────────────────────────────

#[test]
fn login_valid_passes() {
    let req = LoginRequest {
        email: "kwame@example.com".to_string(),
        password: "anypassword".to_string(),
    };
    assert!(req.validate().is_ok());
}

#[test]
fn login_invalid_email_fails() {
    let req = LoginRequest {
        email: "bad".to_string(),
        password: "anypassword".to_string(),
    };
    assert!(req.validate().is_err());
}

// ── ForgotPasswordRequest ─────────────────────────────────────────────────────

#[test]
fn forgot_password_valid_passes() {
    let req = ForgotPasswordRequest {
        email: "kwame@example.com".to_string(),
    };
    assert!(req.validate().is_ok());
}

#[test]
fn forgot_password_invalid_email_fails() {
    let req = ForgotPasswordRequest {
        email: "notvalid".to_string(),
    };
    assert!(req.validate().is_err());
}

// ── ResetPasswordRequest ──────────────────────────────────────────────────────

#[test]
fn reset_password_short_new_password_fails() {
    let req = ResetPasswordRequest {
        token: "some-uuid-token".to_string(),
        new_password: "short".to_string(), // Rust field name stays snake_case
    };
    assert!(req.validate().is_err());
}

#[test]
fn reset_password_valid_passes() {
    let req = ResetPasswordRequest {
        token: "some-uuid-token".to_string(),
        new_password: "NewStrongPass1!".to_string(),
    };
    assert!(req.validate().is_ok());
}

// ── ChangePasswordRequest ─────────────────────────────────────────────────────

#[test]
fn change_password_short_new_password_fails() {
    let req = ChangePasswordRequest {
        current_password: "OldPass123!".to_string(),
        new_password: "bad".to_string(),
    };
    assert!(req.validate().is_err());
}

#[test]
fn change_password_valid_passes() {
    let req = ChangePasswordRequest {
        current_password: "OldPass123!".to_string(),
        new_password: "NewPass123!".to_string(),
    };
    assert!(req.validate().is_ok());
}
