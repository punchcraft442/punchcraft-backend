use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use mongodb::{bson::{doc, oid::ObjectId}, Database};
use uuid::Uuid;

use crate::common::{errors::AppError, middleware::Claims};
use super::models::{
    ChangePasswordRequest, ForgotPasswordRequest, LoginData, LoginRequest,
    RefreshTokenRequest, RegisterRequest, RegisterResponse, ResetPasswordRequest,
    User, UserSummary,
};

fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "change_me".to_string())
}

fn make_access_token(user_id: &str, role: &str) -> Result<String, AppError> {
    let exp = (Utc::now() + chrono::Duration::minutes(15)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

/// Returns (RegisterResponse, activation_token) — caller fires the activation email.
pub async fn register(
    db: &Database,
    req: RegisterRequest,
) -> Result<(RegisterResponse, String), AppError> {
    let users = db.collection::<User>("users");

    if users
        .find_one(doc! { "email": &req.email })
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Email already in use".to_string()));
    }

    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let activation_token = Uuid::new_v4().to_string();
    let activation_expires = Utc::now() + chrono::Duration::days(7);
    let now = Utc::now();

    let user = User {
        id: None,
        email: req.email.clone(),
        password_hash,
        role: req.role,
        is_active: false,
        activation_token: Some(activation_token.clone()),
        activation_token_expires: Some(activation_expires),
        reset_token: None,
        reset_token_expires: None,
        refresh_token: None,
        refresh_token_expires: None,
        created_at: now,
        updated_at: now,
    };

    let result = users.insert_one(&user).await?;
    let inserted_id = result.inserted_id.as_object_id().unwrap();

    let response = RegisterResponse {
        user_id: inserted_id.to_hex(),
        email: user.email,
        role: user.role,
        account_status: "inactive".to_string(),
    };

    Ok((response, activation_token))
}

/// Activates the account linked to the given activation token.
pub async fn verify_email(db: &Database, token: &str) -> Result<(), AppError> {
    let users = db.collection::<User>("users");

    let user = users
        .find_one(doc! { "activation_token": token })
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired activation link".into()))?;

    if let Some(expires) = user.activation_token_expires {
        if Utc::now() > expires {
            return Err(AppError::BadRequest("Activation link has expired".into()));
        }
    }

    users
        .update_one(
            doc! { "_id": user.id },
            doc! {
                "$set": { "is_active": true, "updated_at": Utc::now().to_rfc3339() },
                "$unset": { "activation_token": "", "activation_token_expires": "" }
            },
        )
        .await?;

    Ok(())
}

/// Validates credentials and returns access + refresh tokens.
pub async fn login(db: &Database, req: LoginRequest) -> Result<LoginData, AppError> {
    let users = db.collection::<User>("users");

    let user = users
        .find_one(doc! { "email": &req.email })
        .await?
        .ok_or(AppError::Unauthorized)?;

    let valid = bcrypt::verify(&req.password, &user.password_hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    if !user.is_active {
        return Err(AppError::ForbiddenMsg(
            "Account not yet activated. Please check your email for the activation link.".into(),
        ));
    }

    let user_id = user.id.unwrap().to_hex();
    let role = user.role.to_string();

    let access_token = make_access_token(&user_id, &role)?;

    let refresh_token = Uuid::new_v4().to_string();
    let refresh_expires = Utc::now() + chrono::Duration::days(30);

    users
        .update_one(
            doc! { "_id": user.id },
            doc! { "$set": {
                "refresh_token": &refresh_token,
                "refresh_token_expires": refresh_expires.to_rfc3339(),
                "updated_at": Utc::now().to_rfc3339(),
            }},
        )
        .await?;

    Ok(LoginData {
        access_token,
        refresh_token,
        user: UserSummary::from_user(&user),
    })
}

/// Issues a new access token given a valid refresh token.
pub async fn refresh_access_token(
    db: &Database,
    req: RefreshTokenRequest,
) -> Result<String, AppError> {
    let users = db.collection::<User>("users");

    let user = users
        .find_one(doc! { "refresh_token": &req.refresh_token })
        .await?
        .ok_or(AppError::Unauthorized)?;

    let expires = user
        .refresh_token_expires
        .ok_or(AppError::Unauthorized)?;

    if Utc::now() > expires {
        return Err(AppError::Unauthorized);
    }

    let user_id = user.id.unwrap().to_hex();
    make_access_token(&user_id, &user.role.to_string())
}

/// Clears the stored refresh token, effectively logging the user out.
pub async fn logout(db: &Database, user_id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid user id".into()))?;

    db.collection::<User>("users")
        .update_one(
            doc! { "_id": oid },
            doc! { "$unset": { "refresh_token": "", "refresh_token_expires": "" } },
        )
        .await?;

    Ok(())
}

/// Generates a reset token, stores it on the user document, and returns (email, token).
/// Returns None if the email is not found — caller should still return 200 to prevent enumeration.
pub async fn forgot_password(
    db: &Database,
    req: ForgotPasswordRequest,
) -> Result<Option<(String, String)>, AppError> {
    let users = db.collection::<User>("users");
    let user = match users.find_one(doc! { "email": &req.email }).await? {
        Some(u) => u,
        None => return Ok(None),
    };

    let token = Uuid::new_v4().to_string();
    let expires = Utc::now() + chrono::Duration::hours(1);

    users
        .update_one(
            doc! { "_id": user.id },
            doc! { "$set": {
                "reset_token": &token,
                "reset_token_expires": expires.to_rfc3339(),
                "updated_at": Utc::now().to_rfc3339(),
            }},
        )
        .await?;

    Ok(Some((user.email, token)))
}

/// Validates the reset token and updates the password.
pub async fn reset_password(db: &Database, req: ResetPasswordRequest) -> Result<(), AppError> {
    let users = db.collection::<User>("users");
    let user = users
        .find_one(doc! { "reset_token": &req.token })
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".into()))?;

    let expires = user
        .reset_token_expires
        .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".into()))?;

    if Utc::now() > expires {
        return Err(AppError::BadRequest("Reset token has expired".into()));
    }

    let password_hash = bcrypt::hash(&req.new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    users
        .update_one(
            doc! { "_id": user.id },
            doc! { "$set": {
                "password_hash": password_hash,
                "updated_at": Utc::now().to_rfc3339(),
            }, "$unset": {
                "reset_token": "",
                "reset_token_expires": "",
            }},
        )
        .await?;

    Ok(())
}

/// Verifies the current password then sets the new one.
pub async fn change_password(
    db: &Database,
    user_id: &str,
    req: ChangePasswordRequest,
) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid user id".into()))?;
    let users = db.collection::<User>("users");
    let user = users
        .find_one(doc! { "_id": oid })
        .await?
        .ok_or(AppError::NotFound)?;

    let valid = bcrypt::verify(&req.current_password, &user.password_hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    if !valid {
        return Err(AppError::BadRequest("Current password is incorrect".into()));
    }

    let password_hash = bcrypt::hash(&req.new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    users
        .update_one(
            doc! { "_id": oid },
            doc! { "$set": {
                "password_hash": password_hash,
                "updated_at": Utc::now().to_rfc3339(),
            }},
        )
        .await?;

    Ok(())
}

/// Looks up a user's email by their ObjectId string. Used by admin handlers to send emails.
pub async fn find_user_email(db: &Database, user_id: &str) -> Option<String> {
    let oid = ObjectId::parse_str(user_id).ok()?;
    db.collection::<User>("users")
        .find_one(doc! { "_id": oid })
        .await
        .ok()
        .flatten()
        .map(|u| u.email)
}
