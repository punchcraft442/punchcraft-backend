use chrono::Utc;
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::auth::models::{User, UserRole};
use crate::common::errors::AppError;
use crate::profiles::{models::ProfileSummary, repository as profile_repo, service as profile_service};

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVerificationTierRequest {
    pub tier: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserDirectRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRoleRequest {
    pub role: UserRole,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAdminView {
    pub id: String,
    pub email: String,
    pub role: UserRole,
    pub is_active: bool,
    pub is_suspended: bool,
    pub created_at: chrono::DateTime<Utc>,
}

impl UserAdminView {
    pub fn from(u: User) -> Self {
        Self {
            id: u.id.map(|o| o.to_hex()).unwrap_or_default(),
            email: u.email,
            role: u.role,
            is_active: u.is_active,
            is_suspended: u.is_suspended,
            created_at: u.created_at,
        }
    }
}

// ── Profile approval ──────────────────────────────────────────────────────────

pub async fn approve_profile(db: &Database, profile_id: &str) -> Result<ProfileSummary, AppError> {
    profile_service::admin_approve(db, profile_id).await
}

pub async fn reject_profile(
    db: &Database,
    profile_id: &str,
    req: RejectRequest,
) -> Result<ProfileSummary, AppError> {
    profile_service::admin_reject(db, profile_id, &req.reason).await
}

pub async fn get_approval_queue(
    db: &Database,
    page: u32,
    limit: u32,
) -> Result<(Vec<ProfileSummary>, u64), AppError> {
    let filter = doc! { "status": "submitted" };
    let total = profile_repo::col_profiles(db).count_documents(filter.clone()).await?;
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "updatedAt": 1 })
        .skip(((page.saturating_sub(1)) * limit) as u64)
        .limit(limit as i64)
        .build();
    let profiles: Vec<crate::profiles::models::Profile> = profile_repo::col_profiles(db)
        .find(filter)
        .with_options(opts)
        .await?
        .try_collect()
        .await?;
    Ok((profiles.into_iter().map(ProfileSummary::from).collect(), total))
}

pub async fn set_verification_tier(
    db: &Database,
    profile_id: &str,
    req: SetVerificationTierRequest,
) -> Result<ProfileSummary, AppError> {
    let valid_tiers = ["unverified", "tier2_verified", "tier1_managed_verified"];
    if !valid_tiers.contains(&req.tier.as_str()) {
        return Err(AppError::BadRequest(format!(
            "tier must be one of: {}",
            valid_tiers.join(", ")
        )));
    }
    let pid = ObjectId::parse_str(profile_id).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    profile_repo::col_profiles(db)
        .update_one(
            doc! { "_id": pid },
            doc! { "$set": { "verificationTier": &req.tier, "updatedAt": Utc::now().to_rfc3339() } },
        )
        .await?;
    let updated = profile_repo::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(updated))
}

pub async fn set_visibility(
    db: &Database,
    profile_id: &str,
    visibility: &str,
) -> Result<ProfileSummary, AppError> {
    if visibility != "public" && visibility != "private" {
        return Err(AppError::BadRequest("visibility must be 'public' or 'private'".into()));
    }
    let pid = ObjectId::parse_str(profile_id).map_err(|_| AppError::BadRequest("Invalid id".into()))?;
    let profile = profile_repo::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;

    use crate::profiles::models::ProfileStatus;
    if profile.status != ProfileStatus::Approved {
        return Err(AppError::BadRequest("Only approved profiles can have their visibility changed".into()));
    }

    let searchable = visibility == "public" && profile.has_verified_document;
    profile_repo::update_profile(db, pid, doc! {
        "visibility": visibility,
        "searchable": searchable,
        "updatedAt": Utc::now().to_rfc3339(),
    }).await?;

    let updated = profile_repo::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(updated))
}

// ── User management ───────────────────────────────────────────────────────────

fn users(db: &Database) -> mongodb::Collection<User> {
    db.collection("users")
}

pub async fn list_users(db: &Database, page: u32, limit: u32) -> Result<(Vec<UserAdminView>, u64), AppError> {
    let total = users(db).count_documents(doc! {}).await?;
    let opts = mongodb::options::FindOptions::builder()
        .sort(doc! { "createdAt": -1 })
        .skip(((page.saturating_sub(1)) * limit) as u64)
        .limit(limit as i64)
        .build();
    let list: Vec<User> = users(db).find(doc! {}).with_options(opts).await?.try_collect().await?;
    Ok((list.into_iter().map(UserAdminView::from).collect(), total))
}

pub async fn get_user(db: &Database, user_id: &str) -> Result<UserAdminView, AppError> {
    let oid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid userId".into()))?;
    let user = users(db).find_one(doc! { "_id": oid }).await?.ok_or(AppError::NotFound)?;
    Ok(UserAdminView::from(user))
}

pub async fn suspend_user(
    db: &Database,
    user_id: &str,
    caller_role: &str,
) -> Result<UserAdminView, AppError> {
    let oid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid userId".into()))?;
    let target = users(db).find_one(doc! { "_id": oid }).await?.ok_or(AppError::NotFound)?;

    // regular admin cannot suspend other admins or super_admins
    if caller_role == "admin" {
        let target_role = target.role.to_string();
        if target_role == "admin" || target_role == "super_admin" {
            return Err(AppError::Forbidden);
        }
    }

    users(db)
        .update_one(
            doc! { "_id": oid },
            doc! { "$set": { "is_suspended": true, "updated_at": Utc::now().to_rfc3339() } },
        )
        .await?;

    let updated = users(db).find_one(doc! { "_id": oid }).await?.ok_or(AppError::NotFound)?;
    Ok(UserAdminView::from(updated))
}

pub async fn activate_user(db: &Database, user_id: &str) -> Result<UserAdminView, AppError> {
    let oid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid userId".into()))?;
    users(db)
        .update_one(
            doc! { "_id": oid },
            doc! { "$set": { "is_suspended": false, "is_active": true, "updated_at": Utc::now().to_rfc3339() } },
        )
        .await?;
    let updated = users(db).find_one(doc! { "_id": oid }).await?.ok_or(AppError::NotFound)?;
    Ok(UserAdminView::from(updated))
}

// ── Super admin only ──────────────────────────────────────────────────────────

pub async fn create_user_direct(
    db: &Database,
    req: CreateUserDirectRequest,
) -> Result<UserAdminView, AppError> {
    if users(db).find_one(doc! { "email": &req.email }).await?.is_some() {
        return Err(AppError::Conflict("Email already in use".into()));
    }

    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let now = Utc::now();
    let user = User {
        id: None,
        email: req.email,
        password_hash,
        role: req.role,
        is_active: true,
        is_suspended: false,
        activation_token: None,
        activation_token_expires: None,
        reset_token: None,
        reset_token_expires: None,
        refresh_token: None,
        refresh_token_expires: None,
        phone: None,
        profile_photo: None,
        social_links: None,
        created_at: now,
        updated_at: now,
    };

    let r = users(db).insert_one(&user).await?;
    let id = r.inserted_id.as_object_id().unwrap();
    let mut user = user;
    user.id = Some(id);
    Ok(UserAdminView::from(user))
}

pub async fn ban_user(db: &Database, user_id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid userId".into()))?;
    let r = users(db).delete_one(doc! { "_id": oid }).await?;
    if r.deleted_count == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn change_user_role(
    db: &Database,
    user_id: &str,
    req: ChangeRoleRequest,
) -> Result<UserAdminView, AppError> {
    let oid = ObjectId::parse_str(user_id).map_err(|_| AppError::BadRequest("Invalid userId".into()))?;
    let role_str = req.role.to_string();
    let r = users(db)
        .update_one(
            doc! { "_id": oid },
            doc! { "$set": { "role": &role_str, "updated_at": Utc::now().to_rfc3339() } },
        )
        .await?;
    if r.matched_count == 0 {
        return Err(AppError::NotFound);
    }
    let updated = users(db).find_one(doc! { "_id": oid }).await?.ok_or(AppError::NotFound)?;
    Ok(UserAdminView::from(updated))
}
