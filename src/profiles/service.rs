use chrono::Utc;
use mongodb::{
    bson::{doc, oid::ObjectId, to_bson},
    Database,
};
use uuid::Uuid;

use crate::common::errors::AppError;
use super::{models::*, repository};

// ── Shared helpers ────────────────────────────────────────────────────────────

fn parse_oid(s: &str) -> Result<ObjectId, AppError> {
    ObjectId::parse_str(s).map_err(|_| AppError::BadRequest("Invalid id".into()))
}

fn bson_err(e: mongodb::bson::ser::Error) -> AppError {
    AppError::BadRequest(e.to_string())
}

fn now_str() -> String {
    Utc::now().to_rfc3339()
}

fn new_profile(user_id: ObjectId, role: &str, display_name: &str, bio: Option<String>, location: Option<Location>, contact_details: Option<ContactDetails>, social_links: Option<SocialLinks>, weight_class: Option<String>) -> Profile {
    let now = Utc::now();
    Profile {
        id: None,
        user_id,
        role: role.to_string(),
        display_name: display_name.to_string(),
        bio,
        profile_image: None,
        cover_image: None,
        location,
        contact_details,
        social_links,
        status: ProfileStatus::Draft,
        visibility: ProfileVisibility::Private,
        verification_tier: VerificationTier::Unverified,
        searchable: false,
        has_verified_document: false,
        weight_class,
        rejection_reason: None,
        created_at: now,
        updated_at: now,
    }
}

/// Enforce that the caller's JWT role matches the expected role.
/// super_admin bypasses this check — they can create any profile type.
pub fn enforce_role(claims_role: &str, expected: &str) -> Result<(), AppError> {
    if claims_role == "super_admin" || claims_role == expected {
        return Ok(());
    }
    Err(AppError::ForbiddenMsg(format!(
        "Your account role '{}' cannot create a {} profile",
        claims_role, expected
    )))
}

/// Check that the profile exists, belongs to the caller, and is editable.
async fn editable_profile(db: &Database, profile_id: ObjectId, user_id: ObjectId) -> Result<Profile, AppError> {
    let profile = repository::find_profile_by_id(db, profile_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if profile.user_id != user_id {
        return Err(AppError::Forbidden);
    }
    if profile.status == ProfileStatus::Approved {
        return Err(AppError::BadRequest(
            "Approved profiles cannot be edited. Submit again after making changes.".into(),
        ));
    }
    Ok(profile)
}

/// Visibility rule: owner sees all; everyone else only sees approved+public+has_verified_document.
fn check_visibility(profile: &Profile, caller: Option<&str>) -> Result<(), AppError> {
    let is_owner = caller
        .and_then(|uid| ObjectId::parse_str(uid).ok())
        .map(|uid| uid == profile.user_id)
        .unwrap_or(false);
    if is_owner {
        return Ok(());
    }
    if profile.status != ProfileStatus::Approved
        || profile.visibility != ProfileVisibility::Public
        || !profile.has_verified_document
    {
        return Err(AppError::NotFound);
    }
    Ok(())
}

fn set_nullable<T: serde::Serialize>(
    d: &mut mongodb::bson::Document,
    key: &str,
    field: &Option<Option<T>>,
) -> Result<(), AppError> {
    match field {
        Some(Some(v)) => { d.insert(key, to_bson(v).map_err(bson_err)?); }
        Some(None)    => { d.insert(key, mongodb::bson::Bson::Null); }
        None          => {}
    }
    Ok(())
}

fn base_profile_update_doc(
    display_name: Option<&str>,
    bio: &Option<Option<String>>,
    location: &Option<Option<Location>>,
    contact_details: &Option<Option<ContactDetails>>,
    social_links: &Option<Option<SocialLinks>>,
    weight_class: Option<&str>,
) -> Result<mongodb::bson::Document, AppError> {
    let mut d = doc! { "updatedAt": now_str() };
    if let Some(name) = display_name {
        d.insert("displayName", name);
    }
    set_nullable(&mut d, "bio", bio)?;
    set_nullable(&mut d, "location", location)?;
    set_nullable(&mut d, "contactDetails", contact_details)?;
    set_nullable(&mut d, "socialLinks", social_links)?;
    if let Some(wc) = weight_class {
        d.insert("weightClass", wc);
    }
    Ok(d)
}

fn paginated<T: serde::Serialize>(items: Vec<T>, total: u64, params: &PaginationParams) -> PaginatedResponse<T> {
    let limit = params.limit() as u64;
    let total_pages = (total + limit - 1) / limit;
    PaginatedResponse {
        items,
        pagination: PaginationMeta {
            page: params.page(),
            limit: params.limit(),
            total_items: total,
            total_pages,
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FIGHTER
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_fighter(
    db: &Database,
    user_id: &str,
    req: CreateFighterRequest,
) -> Result<FighterProfileResponse, AppError> {
    let uid = parse_oid(user_id)?;
    if repository::find_profile_by_user_and_role(db, uid, "fighter").await?.is_some() {
        return Err(AppError::Conflict("Fighter profile already exists".into()));
    }
    // Validate linked gym/coach exist with correct roles
    if let Some(gym_id) = &req.linked_gym_id {
        let gid = parse_oid(gym_id)?;
        let g = repository::find_profile_by_id(db, gid).await?.ok_or(AppError::NotFound)?;
        if g.role != "gym" { return Err(AppError::BadRequest("linkedGymId does not refer to a gym profile".into())); }
    }
    let coach_pid_for_sync = if let Some(coach_id) = &req.linked_coach_id {
        let cid = parse_oid(coach_id)?;
        let c = repository::find_profile_by_id(db, cid).await?.ok_or(AppError::NotFound)?;
        if c.role != "coach" { return Err(AppError::BadRequest("linkedCoachId does not refer to a coach profile".into())); }
        Some(cid)
    } else { None };

    let profile = new_profile(
        uid, "fighter", &req.full_name, req.bio, req.location.clone(),
        req.contact_details.clone(), req.social_links.clone(),
        Some(req.weight_class.clone()),
    );
    let pid = repository::insert_profile(db, &profile).await?;
    let pid_str = pid.to_hex();
    let details = FighterDetails {
        id: None,
        profile_id: pid,
        full_name: req.full_name,
        ring_name: req.ring_name,
        nationality: req.nationality,
        weight_class: Some(req.weight_class),
        stance: req.stance,
        height_cm: req.height_cm,
        reach_cm: req.reach_cm,
        record: req.record,
        titles: req.titles.unwrap_or_default(),
        linked_gym_id: req.linked_gym_id,
        linked_coach_id: req.linked_coach_id,
        fight_history: vec![],
    };
    repository::insert_fighter(db, &details).await?;
    if let Some(cpid) = coach_pid_for_sync {
        repository::coach_add_fighter(db, cpid, &pid_str).await?;
    }
    let mut saved = profile;
    saved.id = Some(pid);
    Ok(FighterProfileResponse::from_parts(saved, details))
}

pub async fn get_fighter(
    db: &Database,
    profile_id: &str,
    caller: Option<&str>,
) -> Result<FighterProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.role != "fighter" {
        return Err(AppError::NotFound);
    }
    check_visibility(&profile, caller)?;
    let details = repository::find_fighter(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(FighterProfileResponse::from_parts(profile, details))
}

pub async fn update_fighter(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdateFighterRequest,
) -> Result<FighterProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = editable_profile(db, pid, uid).await?;
    if profile.role != "fighter" {
        return Err(AppError::NotFound);
    }

    // Fetch current fighter details once if gym or coach is changing (needed for roster sync)
    let current_details = if req.linked_gym_id.is_some() || req.linked_coach_id.is_some() {
        Some(repository::find_fighter(db, pid).await?.ok_or(AppError::NotFound)?)
    } else {
        None
    };

    let gym_sync = if let Some(new_gym_id) = &req.linked_gym_id {
        let gid = parse_oid(new_gym_id)?;
        let g = repository::find_profile_by_id(db, gid).await?.ok_or(AppError::NotFound)?;
        if g.role != "gym" { return Err(AppError::BadRequest("linkedGymId does not refer to a gym profile".into())); }
        let old_gym_id = current_details.as_ref().and_then(|d| d.linked_gym_id.clone());
        Some((gid, new_gym_id.clone(), old_gym_id))
    } else { None };

    let coach_sync = if let Some(new_coach_id) = &req.linked_coach_id {
        let ncid = parse_oid(new_coach_id)?;
        let c = repository::find_profile_by_id(db, ncid).await?.ok_or(AppError::NotFound)?;
        if c.role != "coach" { return Err(AppError::BadRequest("linkedCoachId does not refer to a coach profile".into())); }
        let old_coach_id = current_details.as_ref().and_then(|d| d.linked_coach_id.clone());
        Some((ncid, old_coach_id))
    } else { None };

    let weight_class_ref = req.weight_class.as_deref();
    let display_name = req.full_name.as_deref();
    let p_update = base_profile_update_doc(
        display_name, &req.bio, &req.location, &req.contact_details, &req.social_links,
        weight_class_ref,
    )?;
    repository::update_profile(db, pid, p_update).await?;

    let mut d_update = doc! { };
    if let Some(v) = &req.full_name       { d_update.insert("fullName", v.as_str()); }
    if let Some(v) = &req.ring_name       { d_update.insert("ringName", v.as_str()); }
    if let Some(v) = &req.nationality     { d_update.insert("nationality", v.as_str()); }
    if let Some(v) = &req.weight_class    { d_update.insert("weightClass", v.as_str()); }
    if let Some(v) = &req.stance          { d_update.insert("stance", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = req.height_cm        { d_update.insert("heightCm", v); }
    if let Some(v) = req.reach_cm         { d_update.insert("reachCm", v); }
    if let Some(v) = &req.record          { d_update.insert("record", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.titles          { d_update.insert("titles", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.linked_gym_id   { d_update.insert("linkedGymId", v.as_str()); }
    if let Some(v) = &req.linked_coach_id { d_update.insert("linkedCoachId", v.as_str()); }
    if !d_update.is_empty() {
        repository::update_fighter(db, pid, d_update).await?;
    }

    let pid_str = pid.to_hex();

    // Sync gym rosterFighterIds when fighter changes their gym
    if let Some((new_gid, new_gym_id_str, old_gym_id)) = gym_sync {
        if let Some(old_id) = old_gym_id {
            if old_id != new_gym_id_str {
                if let Ok(old_gid) = ObjectId::parse_str(&old_id) {
                    repository::gym_remove_fighter(db, old_gid, &pid_str).await?;
                }
            }
        }
        repository::gym_add_fighter(db, new_gid, &pid_str).await?;
    }

    // Sync coach associatedFighterIds after fighter details are saved
    if let Some((new_cpid, old_coach_id_str)) = coach_sync {
        if let Some(old_id) = old_coach_id_str {
            if old_id != new_cpid.to_hex() {
                if let Ok(old_cpid) = ObjectId::parse_str(&old_id) {
                    repository::coach_remove_fighter(db, old_cpid, &pid_str).await?;
                }
            }
        }
        repository::coach_add_fighter(db, new_cpid, &pid_str).await?;
    }

    let updated_profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let updated_details = repository::find_fighter(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(FighterProfileResponse::from_parts(updated_profile, updated_details))
}

pub async fn submit_fighter(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<FighterProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "fighter" { return Err(AppError::NotFound); }
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "submitted",
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_fighter(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(FighterProfileResponse::from_parts(updated, details))
}

pub async fn list_fighters(
    db: &Database,
    params: &PaginationParams,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let (profiles, total) = repository::list_profiles(db, Some("fighter"), params).await?;
    Ok(paginated(profiles.into_iter().map(ProfileSummary::from).collect(), total, params))
}

pub async fn add_fight_history(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: CreateFightHistoryRequest,
) -> Result<FighterProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "fighter" { return Err(AppError::NotFound); }

    let entry = FightHistoryEntry {
        id: Uuid::new_v4().to_string(),
        opponent_name: req.opponent_name,
        event_name: req.event_name,
        event_date: req.event_date,
        result: req.result,
        method: req.method,
        round: req.round,
    };
    repository::push_fight_history(db, pid, &entry).await?;
    repository::update_profile(db, pid, doc! { "updatedAt": now_str() }).await?;

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_fighter(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(FighterProfileResponse::from_parts(updated, details))
}

pub async fn delete_fight_history(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    fight_id: &str,
) -> Result<(), AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "fighter" { return Err(AppError::NotFound); }

    repository::pull_fight_history(db, pid, fight_id).await?;
    repository::update_profile(db, pid, doc! { "updatedAt": now_str() }).await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// GYM
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_gym(
    db: &Database,
    user_id: &str,
    req: CreateGymRequest,
) -> Result<GymProfileResponse, AppError> {
    let uid = parse_oid(user_id)?;
    if repository::find_profile_by_user_and_role(db, uid, "gym").await?.is_some() {
        return Err(AppError::Conflict("Gym profile already exists".into()));
    }
    let profile = new_profile(uid, "gym", &req.name, req.bio, req.location.clone(), req.contact_details.clone(), req.social_links.clone(), None);
    let pid = repository::insert_profile(db, &profile).await?;
    let details = GymDetails {
        id: None,
        profile_id: pid,
        name: req.name,
        address: req.address,
        services: req.services.unwrap_or_default(),
        facilities: req.facilities.unwrap_or_default(),
        linked_coach_ids: vec![],
        roster_fighter_ids: vec![],
    };
    repository::insert_gym(db, &details).await?;
    let mut saved = profile;
    saved.id = Some(pid);
    Ok(GymProfileResponse::from_parts(saved, details))
}

pub async fn get_gym(
    db: &Database,
    profile_id: &str,
    caller: Option<&str>,
) -> Result<GymProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.role != "gym" { return Err(AppError::NotFound); }
    check_visibility(&profile, caller)?;
    let details = repository::find_gym(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(GymProfileResponse::from_parts(profile, details))
}

pub async fn update_gym(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdateGymRequest,
) -> Result<GymProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = editable_profile(db, pid, uid).await?;
    if profile.role != "gym" { return Err(AppError::NotFound); }

    let p_update = base_profile_update_doc(req.name.as_deref(), &req.bio, &req.location, &req.contact_details, &req.social_links, None)?;
    repository::update_profile(db, pid, p_update).await?;

    let mut d_update = doc! {};
    if let Some(v) = &req.name      { d_update.insert("name", v.as_str()); }
    if let Some(v) = &req.address   { d_update.insert("address", v.as_str()); }
    if let Some(v) = &req.services  { d_update.insert("services", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.facilities { d_update.insert("facilities", to_bson(v).map_err(bson_err)?); }
    if !d_update.is_empty() {
        repository::update_gym(db, pid, d_update).await?;
    }

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_gym(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(GymProfileResponse::from_parts(updated, details))
}

pub async fn submit_gym(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<GymProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "gym" { return Err(AppError::NotFound); }
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "submitted",
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_gym(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(GymProfileResponse::from_parts(updated, details))
}

pub async fn list_gyms(
    db: &Database,
    params: &PaginationParams,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let (profiles, total) = repository::list_profiles(db, Some("gym"), params).await?;
    Ok(paginated(profiles.into_iter().map(ProfileSummary::from).collect(), total, params))
}

pub async fn gym_link_coach(
    db: &Database,
    gym_profile_id: &str,
    user_id: &str,
    coach_profile_id: &str,
) -> Result<(), AppError> {
    let pid = parse_oid(gym_profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "gym" { return Err(AppError::NotFound); }

    let coach_pid = parse_oid(coach_profile_id)?;
    let coach = repository::find_profile_by_id(db, coach_pid).await?.ok_or(AppError::NotFound)?;
    if coach.role != "coach" {
        return Err(AppError::BadRequest("Linked profile is not a coach".into()));
    }

    repository::gym_add_coach(db, pid, coach_profile_id).await?;
    repository::coach_add_gym(db, coach_pid, gym_profile_id).await
}

pub async fn gym_unlink_coach(
    db: &Database,
    gym_profile_id: &str,
    user_id: &str,
    coach_profile_id: &str,
) -> Result<(), AppError> {
    let pid = parse_oid(gym_profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "gym" { return Err(AppError::NotFound); }
    repository::gym_remove_coach(db, pid, coach_profile_id).await?;
    if let Ok(coach_pid) = ObjectId::parse_str(coach_profile_id) {
        repository::coach_remove_gym(db, coach_pid, gym_profile_id).await?;
    }
    Ok(())
}

pub async fn gym_link_fighter(
    db: &Database,
    gym_profile_id: &str,
    user_id: &str,
    fighter_profile_id: &str,
) -> Result<(), AppError> {
    let pid = parse_oid(gym_profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "gym" { return Err(AppError::NotFound); }

    let fighter_pid = parse_oid(fighter_profile_id)?;
    let fighter = repository::find_profile_by_id(db, fighter_pid).await?.ok_or(AppError::NotFound)?;
    if fighter.role != "fighter" {
        return Err(AppError::BadRequest("Linked profile is not a fighter".into()));
    }

    let fighter_details = repository::find_fighter(db, fighter_pid).await?.ok_or(AppError::NotFound)?;
    if let Some(existing_gym) = &fighter_details.linked_gym_id {
        if existing_gym != gym_profile_id {
            return Err(AppError::Conflict(
                "Fighter already belongs to another gym. The fighter must update their profile to change gyms.".into(),
            ));
        }
        // Already linked to this gym — nothing to do
        return Ok(());
    }

    repository::gym_add_fighter(db, pid, fighter_profile_id).await?;
    repository::update_fighter(db, fighter_pid, doc! { "linkedGymId": gym_profile_id }).await
}

pub async fn gym_unlink_fighter(
    db: &Database,
    gym_profile_id: &str,
    user_id: &str,
    fighter_profile_id: &str,
) -> Result<(), AppError> {
    let pid = parse_oid(gym_profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "gym" { return Err(AppError::NotFound); }
    repository::gym_remove_fighter(db, pid, fighter_profile_id).await?;
    // Clear fighter's linkedGymId if it points to this gym
    if let Ok(fighter_pid) = ObjectId::parse_str(fighter_profile_id) {
        if let Some(fd) = repository::find_fighter(db, fighter_pid).await? {
            if fd.linked_gym_id.as_deref() == Some(gym_profile_id) {
                repository::update_fighter(db, fighter_pid, doc! { "linkedGymId": mongodb::bson::Bson::Null }).await?;
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// COACH
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_coach(
    db: &Database,
    user_id: &str,
    req: CreateCoachRequest,
) -> Result<CoachProfileResponse, AppError> {
    let uid = parse_oid(user_id)?;
    if repository::find_profile_by_user_and_role(db, uid, "coach").await?.is_some() {
        return Err(AppError::Conflict("Coach profile already exists".into()));
    }
    let profile = new_profile(uid, "coach", &req.full_name, req.bio, req.location.clone(), req.contact_details.clone(), req.social_links.clone(), None);
    let pid = repository::insert_profile(db, &profile).await?;
    let details = CoachDetails {
        id: None,
        profile_id: pid,
        full_name: req.full_name,
        experience_summary: req.experience_summary,
        specialties: req.specialties.unwrap_or_default(),
        linked_gym_ids: req.linked_gym_ids.unwrap_or_default(),
        associated_fighter_ids: vec![],
        certifications: vec![],
    };
    repository::insert_coach(db, &details).await?;
    let mut saved = profile;
    saved.id = Some(pid);
    Ok(CoachProfileResponse::from_parts(saved, details))
}

pub async fn get_coach(
    db: &Database,
    profile_id: &str,
    caller: Option<&str>,
) -> Result<CoachProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.role != "coach" { return Err(AppError::NotFound); }
    check_visibility(&profile, caller)?;
    let details = repository::find_coach(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(CoachProfileResponse::from_parts(profile, details))
}

pub async fn update_coach(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdateCoachRequest,
) -> Result<CoachProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = editable_profile(db, pid, uid).await?;
    if profile.role != "coach" { return Err(AppError::NotFound); }

    let p_update = base_profile_update_doc(req.full_name.as_deref(), &req.bio, &req.location, &req.contact_details, &req.social_links, None)?;
    repository::update_profile(db, pid, p_update).await?;

    let mut d_update = doc! {};
    if let Some(v) = &req.full_name            { d_update.insert("fullName", v.as_str()); }
    if let Some(v) = &req.experience_summary   { d_update.insert("experienceSummary", v.as_str()); }
    if let Some(v) = &req.specialties          { d_update.insert("specialties", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.linked_gym_ids       { d_update.insert("linkedGymIds", to_bson(v).map_err(bson_err)?); }
    if !d_update.is_empty() {
        repository::update_coach(db, pid, d_update).await?;
    }

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_coach(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(CoachProfileResponse::from_parts(updated, details))
}

pub async fn submit_coach(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<CoachProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "coach" { return Err(AppError::NotFound); }
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "submitted",
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_coach(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(CoachProfileResponse::from_parts(updated, details))
}

pub async fn list_coaches(
    db: &Database,
    params: &PaginationParams,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let (profiles, total) = repository::list_profiles(db, Some("coach"), params).await?;
    Ok(paginated(profiles.into_iter().map(ProfileSummary::from).collect(), total, params))
}

pub async fn add_coach_certification(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    data: Vec<u8>,
    filename: String,
    label: Option<String>,
) -> Result<CoachProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "coach" { return Err(AppError::NotFound); }

    let client = crate::media::cloudinary::CloudinaryClient::from_env()?;
    let folder = format!("punchcraft/profiles/{}/certifications", pid.to_hex());
    let resp = client.upload_auto(data, filename, &folder).await?;

    let entry = DocumentEntry {
        id: Uuid::new_v4().to_string(),
        label,
        file_url: resp.secure_url,
        uploaded_at: now_str(),
    };
    repository::push_coach_certification(db, pid, &entry).await?;
    repository::update_profile(db, pid, doc! { "updatedAt": now_str() }).await?;

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_coach(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(CoachProfileResponse::from_parts(updated, details))
}

// ─────────────────────────────────────────────────────────────────────────────
// OFFICIAL
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_official(
    db: &Database,
    user_id: &str,
    req: CreateOfficialRequest,
) -> Result<OfficialProfileResponse, AppError> {
    let uid = parse_oid(user_id)?;
    if repository::find_profile_by_user_and_role(db, uid, "official").await?.is_some() {
        return Err(AppError::Conflict("Official profile already exists".into()));
    }
    let profile = new_profile(uid, "official", &req.full_name, req.bio, req.location.clone(), req.contact_details.clone(), req.social_links.clone(), None);
    let pid = repository::insert_profile(db, &profile).await?;
    let details = OfficialDetails {
        id: None,
        profile_id: pid,
        full_name: req.full_name,
        official_type: req.official_type,
        experience_years: req.experience_years,
        events_worked: req.events_worked.unwrap_or_default(),
        licensing_details: req.licensing_details,
        coverage_area: req.coverage_area.unwrap_or_default(),
        availability: req.availability,
        credentials: vec![],
    };
    repository::insert_official(db, &details).await?;
    let mut saved = profile;
    saved.id = Some(pid);
    Ok(OfficialProfileResponse::from_parts(saved, details))
}

pub async fn get_official(
    db: &Database,
    profile_id: &str,
    caller: Option<&str>,
) -> Result<OfficialProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.role != "official" { return Err(AppError::NotFound); }
    check_visibility(&profile, caller)?;
    let details = repository::find_official(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(OfficialProfileResponse::from_parts(profile, details))
}

pub async fn update_official(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdateOfficialRequest,
) -> Result<OfficialProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = editable_profile(db, pid, uid).await?;
    if profile.role != "official" { return Err(AppError::NotFound); }

    let p_update = base_profile_update_doc(req.full_name.as_deref(), &req.bio, &req.location, &req.contact_details, &req.social_links, None)?;
    repository::update_profile(db, pid, p_update).await?;

    let mut d_update = doc! {};
    if let Some(v) = &req.full_name           { d_update.insert("fullName", v.as_str()); }
    if let Some(v) = &req.official_type       { d_update.insert("officialType", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = req.experience_years     { d_update.insert("experienceYears", v); }
    if let Some(v) = &req.events_worked       { d_update.insert("eventsWorked", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.licensing_details   { d_update.insert("licensingDetails", v.as_str()); }
    if let Some(v) = &req.coverage_area       { d_update.insert("coverageArea", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.availability        { d_update.insert("availability", v.as_str()); }
    if !d_update.is_empty() {
        repository::update_official(db, pid, d_update).await?;
    }

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_official(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(OfficialProfileResponse::from_parts(updated, details))
}

pub async fn submit_official(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<OfficialProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "official" { return Err(AppError::NotFound); }
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "submitted",
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_official(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(OfficialProfileResponse::from_parts(updated, details))
}

pub async fn list_officials(
    db: &Database,
    params: &PaginationParams,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let (profiles, total) = repository::list_profiles(db, Some("official"), params).await?;
    Ok(paginated(profiles.into_iter().map(ProfileSummary::from).collect(), total, params))
}

pub async fn add_official_credential(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    data: Vec<u8>,
    filename: String,
    label: Option<String>,
) -> Result<OfficialProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "official" { return Err(AppError::NotFound); }

    let client = crate::media::cloudinary::CloudinaryClient::from_env()?;
    let folder = format!("punchcraft/profiles/{}/credentials", pid.to_hex());
    let resp = client.upload_auto(data, filename, &folder).await?;

    let entry = DocumentEntry {
        id: Uuid::new_v4().to_string(),
        label,
        file_url: resp.secure_url,
        uploaded_at: now_str(),
    };
    repository::push_official_credential(db, pid, &entry).await?;
    repository::update_profile(db, pid, doc! { "updatedAt": now_str() }).await?;

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_official(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(OfficialProfileResponse::from_parts(updated, details))
}

// ─────────────────────────────────────────────────────────────────────────────
// PROMOTER
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_promoter(
    db: &Database,
    user_id: &str,
    req: CreatePromoterRequest,
) -> Result<PromoterProfileResponse, AppError> {
    let uid = parse_oid(user_id)?;
    if repository::find_profile_by_user_and_role(db, uid, "promoter").await?.is_some() {
        return Err(AppError::Conflict("Promoter profile already exists".into()));
    }
    let profile = new_profile(uid, "promoter", &req.organization_name, req.bio, req.location.clone(), req.contact_details.clone(), req.social_links.clone(), None);
    let pid = repository::insert_profile(db, &profile).await?;
    let details = PromoterDetails {
        id: None,
        profile_id: pid,
        organization_name: req.organization_name,
        coverage_areas: req.coverage_areas.unwrap_or_default(),
        past_events: req.past_events.unwrap_or_default(),
        references: req.references.unwrap_or_default(),
    };
    repository::insert_promoter(db, &details).await?;
    let mut saved = profile;
    saved.id = Some(pid);
    Ok(PromoterProfileResponse::from_parts(saved, details))
}

pub async fn get_promoter(
    db: &Database,
    profile_id: &str,
    caller: Option<&str>,
) -> Result<PromoterProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.role != "promoter" { return Err(AppError::NotFound); }
    check_visibility(&profile, caller)?;
    let details = repository::find_promoter(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(PromoterProfileResponse::from_parts(profile, details))
}

pub async fn update_promoter(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdatePromoterRequest,
) -> Result<PromoterProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = editable_profile(db, pid, uid).await?;
    if profile.role != "promoter" { return Err(AppError::NotFound); }

    let p_update = base_profile_update_doc(req.organization_name.as_deref(), &req.bio, &req.location, &req.contact_details, &req.social_links, None)?;
    repository::update_profile(db, pid, p_update).await?;

    let mut d_update = doc! {};
    if let Some(v) = &req.organization_name { d_update.insert("organizationName", v.as_str()); }
    if let Some(v) = &req.coverage_areas    { d_update.insert("coverageAreas", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.past_events       { d_update.insert("pastEvents", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.references        { d_update.insert("references", to_bson(v).map_err(bson_err)?); }
    if !d_update.is_empty() {
        repository::update_promoter(db, pid, d_update).await?;
    }

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_promoter(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(PromoterProfileResponse::from_parts(updated, details))
}

pub async fn submit_promoter(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<PromoterProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "promoter" { return Err(AppError::NotFound); }
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "submitted",
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_promoter(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(PromoterProfileResponse::from_parts(updated, details))
}

pub async fn list_promoters(
    db: &Database,
    params: &PaginationParams,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let (profiles, total) = repository::list_profiles(db, Some("promoter"), params).await?;
    Ok(paginated(profiles.into_iter().map(ProfileSummary::from).collect(), total, params))
}

// ─────────────────────────────────────────────────────────────────────────────
// MATCHMAKER
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_matchmaker(
    db: &Database,
    user_id: &str,
    req: CreateMatchmakerRequest,
) -> Result<MatchmakerProfileResponse, AppError> {
    let uid = parse_oid(user_id)?;
    if repository::find_profile_by_user_and_role(db, uid, "matchmaker").await?.is_some() {
        return Err(AppError::Conflict("Matchmaker profile already exists".into()));
    }
    let profile = new_profile(uid, "matchmaker", &req.full_name, req.bio, req.location.clone(), req.contact_details.clone(), req.social_links.clone(), None);
    let pid = repository::insert_profile(db, &profile).await?;
    let details = MatchmakerDetails {
        id: None,
        profile_id: pid,
        full_name: req.full_name,
        regions_served: req.regions_served.unwrap_or_default(),
        weight_classes_focus: req.weight_classes_focus.unwrap_or_default(),
        experience_summary: req.experience_summary,
        past_matchups: req.past_matchups.unwrap_or_default(),
    };
    repository::insert_matchmaker(db, &details).await?;
    let mut saved = profile;
    saved.id = Some(pid);
    Ok(MatchmakerProfileResponse::from_parts(saved, details))
}

pub async fn get_matchmaker(
    db: &Database,
    profile_id: &str,
    caller: Option<&str>,
) -> Result<MatchmakerProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.role != "matchmaker" { return Err(AppError::NotFound); }
    check_visibility(&profile, caller)?;
    let details = repository::find_matchmaker(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(MatchmakerProfileResponse::from_parts(profile, details))
}

pub async fn update_matchmaker(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdateMatchmakerRequest,
) -> Result<MatchmakerProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = editable_profile(db, pid, uid).await?;
    if profile.role != "matchmaker" { return Err(AppError::NotFound); }

    let p_update = base_profile_update_doc(req.full_name.as_deref(), &req.bio, &req.location, &req.contact_details, &req.social_links, None)?;
    repository::update_profile(db, pid, p_update).await?;

    let mut d_update = doc! {};
    if let Some(v) = &req.full_name            { d_update.insert("fullName", v.as_str()); }
    if let Some(v) = &req.regions_served       { d_update.insert("regionsServed", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.weight_classes_focus { d_update.insert("weightClassesFocus", to_bson(v).map_err(bson_err)?); }
    if let Some(v) = &req.experience_summary   { d_update.insert("experienceSummary", v.as_str()); }
    if let Some(v) = &req.past_matchups        { d_update.insert("pastMatchups", to_bson(v).map_err(bson_err)?); }
    if !d_update.is_empty() {
        repository::update_matchmaker(db, pid, d_update).await?;
    }

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_matchmaker(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(MatchmakerProfileResponse::from_parts(updated, details))
}

pub async fn submit_matchmaker(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<MatchmakerProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "matchmaker" { return Err(AppError::NotFound); }
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "submitted",
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_matchmaker(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(MatchmakerProfileResponse::from_parts(updated, details))
}

pub async fn list_matchmakers(
    db: &Database,
    params: &PaginationParams,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let (profiles, total) = repository::list_profiles(db, Some("matchmaker"), params).await?;
    Ok(paginated(profiles.into_iter().map(ProfileSummary::from).collect(), total, params))
}

// ─────────────────────────────────────────────────────────────────────────────
// FAN
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_fan(
    db: &Database,
    user_id: &str,
    req: CreateFanRequest,
) -> Result<FanProfileResponse, AppError> {
    let uid = parse_oid(user_id)?;
    if repository::find_profile_by_user_and_role(db, uid, "fan").await?.is_some() {
        return Err(AppError::Conflict("Fan profile already exists".into()));
    }
    let profile = new_profile(uid, "fan", &req.display_name, req.bio, req.location.clone(), None, req.social_links.clone(), None);
    let pid = repository::insert_profile(db, &profile).await?;
    let details = FanDetails {
        id: None,
        profile_id: pid,
        display_name: req.display_name,
        favourite_weight_class: req.favourite_weight_class,
    };
    repository::insert_fan(db, &details).await?;
    let mut saved = profile;
    saved.id = Some(pid);
    Ok(FanProfileResponse::from_parts(saved, details))
}

pub async fn get_fan(
    db: &Database,
    profile_id: &str,
    caller: Option<&str>,
) -> Result<FanProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.role != "fan" { return Err(AppError::NotFound); }
    check_visibility(&profile, caller)?;
    let details = repository::find_fan(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(FanProfileResponse::from_parts(profile, details))
}

pub async fn update_fan(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    req: UpdateFanRequest,
) -> Result<FanProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = editable_profile(db, pid, uid).await?;
    if profile.role != "fan" { return Err(AppError::NotFound); }

    let p_update = base_profile_update_doc(req.display_name.as_deref(), &req.bio, &req.location, &None, &req.social_links, None)?;
    repository::update_profile(db, pid, p_update).await?;

    let mut d_update = doc! {};
    if let Some(v) = &req.display_name            { d_update.insert("displayName", v.as_str()); }
    if let Some(v) = &req.favourite_weight_class   { d_update.insert("favouriteWeightClass", v.as_str()); }
    if !d_update.is_empty() {
        repository::update_fan(db, pid, d_update).await?;
    }

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_fan(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(FanProfileResponse::from_parts(updated, details))
}

pub async fn submit_fan(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<FanProfileResponse, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid { return Err(AppError::Forbidden); }
    if profile.role != "fan" { return Err(AppError::NotFound); }
    if profile.status != ProfileStatus::Draft && profile.status != ProfileStatus::Rejected {
        return Err(AppError::BadRequest("Only draft or rejected profiles can be submitted".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "submitted",
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    let details = repository::find_fan(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(FanProfileResponse::from_parts(updated, details))
}

pub async fn list_fans(
    db: &Database,
    params: &PaginationParams,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let (profiles, total) = repository::list_profiles(db, Some("fan"), params).await?;
    Ok(paginated(profiles.into_iter().map(ProfileSummary::from).collect(), total, params))
}

// ─────────────────────────────────────────────────────────────────────────────
// Revision request
// ─────────────────────────────────────────────────────────────────────────────

/// Owner requests a revision on an approved profile — reverts to draft/private/not-searchable.
pub async fn request_revision(
    db: &Database,
    profile_id: &str,
    user_id: &str,
) -> Result<ProfileSummary, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid {
        return Err(AppError::Forbidden);
    }
    if profile.status != ProfileStatus::Approved {
        return Err(AppError::BadRequest(
            "Only approved profiles can request a revision".into(),
        ));
    }
    repository::update_profile(db, pid, doc! {
        "status": "draft",
        "visibility": "private",
        "searchable": false,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(updated))
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin helpers (used by admin module)
// ─────────────────────────────────────────────────────────────────────────────

/// Approve a profile: set status=approved, visibility=public, searchable=true.
pub async fn admin_approve(db: &Database, profile_id: &str) -> Result<ProfileSummary, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.status != ProfileStatus::Submitted {
        return Err(AppError::BadRequest("Only submitted profiles can be approved".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "approved",
        "visibility": "public",
        "searchable": true,
        "rejectionReason": mongodb::bson::Bson::Null,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(updated))
}

/// Reject a profile: set status=rejected, store reason.
pub async fn admin_reject(db: &Database, profile_id: &str, reason: &str) -> Result<ProfileSummary, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.status != ProfileStatus::Submitted {
        return Err(AppError::BadRequest("Only submitted profiles can be rejected".into()));
    }
    if reason.trim().is_empty() {
        return Err(AppError::BadRequest("Rejection reason is required".into()));
    }
    repository::update_profile(db, pid, doc! {
        "status": "rejected",
        "rejectionReason": reason,
        "updatedAt": now_str(),
    }).await?;
    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(updated))
}

/// Fetch a profile summary by ID — used by admin for any role.
pub async fn admin_get_profile(db: &Database, profile_id: &str) -> Result<ProfileSummary, AppError> {
    let pid = parse_oid(profile_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(profile))
}

/// Upload a profile image to Cloudinary and update profileImageUrl on the profile.
/// Caller must own the profile.
pub async fn update_profile_image(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    data: Vec<u8>,
    filename: String,
) -> Result<ProfileSummary, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid {
        return Err(AppError::Forbidden);
    }

    let client = crate::media::cloudinary::CloudinaryClient::from_env()?;
    let folder = format!("punchcraft/profiles/{}/avatar", pid.to_hex());
    let resp = client.upload(data, filename, &folder).await?;

    repository::update_profile(db, pid, doc! {
        "profileImageUrl": &resp.secure_url,
        "updatedAt": now_str(),
    }).await?;

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(updated))
}

/// Upload a cover image to Cloudinary and update coverImageUrl on the profile.
/// Caller must own the profile.
pub async fn update_cover_image(
    db: &Database,
    profile_id: &str,
    user_id: &str,
    data: Vec<u8>,
    filename: String,
) -> Result<ProfileSummary, AppError> {
    let pid = parse_oid(profile_id)?;
    let uid = parse_oid(user_id)?;
    let profile = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    if profile.user_id != uid {
        return Err(AppError::Forbidden);
    }

    let client = crate::media::cloudinary::CloudinaryClient::from_env()?;
    let folder = format!("punchcraft/profiles/{}/cover", pid.to_hex());
    let resp = client.upload(data, filename, &folder).await?;

    repository::update_profile(db, pid, doc! {
        "coverImageUrl": &resp.secure_url,
        "updatedAt": now_str(),
    }).await?;

    let updated = repository::find_profile_by_id(db, pid).await?.ok_or(AppError::NotFound)?;
    Ok(ProfileSummary::from(updated))
}
