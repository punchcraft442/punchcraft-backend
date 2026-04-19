use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::auth::models::{User, UserRole, UserSocialLinks};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMeResponse {
    pub id: String,
    pub email: String,
    pub role: UserRole,
    pub account_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_photo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub social_links: Option<UserSocialLinks>,
}

impl UserMeResponse {
    pub fn from_user(u: &User) -> Self {
        let account_status = if u.is_suspended {
            "suspended"
        } else if u.is_active {
            "active"
        } else {
            "inactive"
        }
        .to_string();

        Self {
            id: u.id.map(|o| o.to_hex()).unwrap_or_default(),
            email: u.email.clone(),
            role: u.role.clone(),
            account_status,
            phone: u.phone.clone(),
            profile_photo: u.profile_photo.clone(),
            social_links: u.social_links.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeRequest {
    #[validate(length(min = 7, message = "phone must be at least 7 characters"))]
    pub phone: Option<String>,
    #[validate(url(message = "profilePhoto must be a valid URL"))]
    pub profile_photo: Option<String>,
    pub social_links: Option<UserSocialLinks>,
}
