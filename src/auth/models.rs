use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Stored in the `users` collection.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_token_expires: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_token_expires: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token_expires: Option<DateTime<Utc>>,
    #[serde(default)]
    pub is_suspended: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(rename = "profilePhoto", skip_serializing_if = "Option::is_none")]
    pub profile_photo: Option<String>,
    #[serde(rename = "socialLinks", skip_serializing_if = "Option::is_none")]
    pub social_links: Option<UserSocialLinks>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserSocialLinks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instagram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiktok: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    SuperAdmin,
    Admin,
    Fighter,
    Gym,
    Coach,
    Official,
    Promoter,
    Matchmaker,
    Fan,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        write!(f, "{}", s)
    }
}

/// Request body for POST /auth/register
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub role: UserRole,
}

/// Request body for POST /auth/login
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

/// Request body for POST /auth/forgot-password
#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}

/// Request body for POST /auth/reset-password
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[serde(rename = "newPassword")]
    #[validate(length(min = 8))]
    pub new_password: String,
}

/// Request body for PATCH /auth/change-password
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[serde(rename = "currentPassword")]
    pub current_password: String,
    #[serde(rename = "newPassword")]
    #[validate(length(min = 8))]
    pub new_password: String,
}

/// Request body for POST /auth/refresh
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

/// Register response — returned on POST /auth/register.
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub email: String,
    pub role: UserRole,
    #[serde(rename = "accountStatus")]
    pub account_status: String,
}

/// Compact user shape embedded in login/refresh responses.
#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub id: String,
    pub email: String,
    pub role: UserRole,
    #[serde(rename = "accountStatus")]
    pub account_status: String,
}

/// Login response data — returned on POST /auth/login and POST /auth/refresh.
#[derive(Debug, Serialize)]
pub struct LoginData {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    pub user: UserSummary,
}

impl UserSummary {
    pub fn from_user(u: &User) -> Self {
        Self {
            id: u.id.map(|o| o.to_hex()).unwrap_or_default(),
            email: u.email.clone(),
            role: u.role.clone(),
            account_status: if u.is_active {
                "active".to_string()
            } else {
                "inactive".to_string()
            },
        }
    }
}
