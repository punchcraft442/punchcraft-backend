use actix_web::{web, HttpRequest, HttpResponse};
use mongodb::Database;
use validator::Validate;

use crate::common::{
    email::EmailService,
    errors::AppError,
    middleware::{extract_claims, require_auth},
    response,
};
use super::{models::*, service};

// ── FIGHTER ───────────────────────────────────────────────────────────────────

pub async fn create_fighter(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateFighterRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::enforce_role(&claims.role, "fighter")?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_fighter(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_fighter(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let caller = extract_claims(&req).map(|c| c.sub);
    let profile = service::get_fighter(db.get_ref(), &path.into_inner(), caller.as_deref()).await?;
    Ok(response::ok(profile))
}

pub async fn update_fighter(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateFighterRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_fighter(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_fighter(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_fighter(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.full_name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}

pub async fn list_fighters(
    db: web::Data<Database>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    let result = service::list_fighters(db.get_ref(), &params).await?;
    Ok(response::ok(result))
}

pub async fn add_fight_history(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<CreateFightHistoryRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::add_fight_history(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn delete_fight_history(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let (profile_id, fight_id) = path.into_inner();
    service::delete_fight_history(db.get_ref(), &profile_id, &claims.sub, &fight_id).await?;
    Ok(response::no_content())
}

// ── GYM ──────────────────────────────────────────────────────────────────────

pub async fn create_gym(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateGymRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::enforce_role(&claims.role, "gym")?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_gym(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_gym(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let caller = extract_claims(&req).map(|c| c.sub);
    let profile = service::get_gym(db.get_ref(), &path.into_inner(), caller.as_deref()).await?;
    Ok(response::ok(profile))
}

pub async fn update_gym(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateGymRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_gym(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_gym(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_gym(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}

pub async fn list_gyms(
    db: web::Data<Database>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    Ok(response::ok(service::list_gyms(db.get_ref(), &params).await?))
}

pub async fn gym_link_coach(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let (gym_id, coach_id) = path.into_inner();
    service::gym_link_coach(db.get_ref(), &gym_id, &claims.sub, &coach_id).await?;
    Ok(response::ok_message("Coach linked to gym"))
}

pub async fn gym_unlink_coach(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let (gym_id, coach_id) = path.into_inner();
    service::gym_unlink_coach(db.get_ref(), &gym_id, &claims.sub, &coach_id).await?;
    Ok(response::no_content())
}

pub async fn gym_link_fighter(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let (gym_id, fighter_id) = path.into_inner();
    service::gym_link_fighter(db.get_ref(), &gym_id, &claims.sub, &fighter_id).await?;
    Ok(response::ok_message("Fighter added to gym roster"))
}

pub async fn gym_unlink_fighter(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let (gym_id, fighter_id) = path.into_inner();
    service::gym_unlink_fighter(db.get_ref(), &gym_id, &claims.sub, &fighter_id).await?;
    Ok(response::no_content())
}

// ── COACH ─────────────────────────────────────────────────────────────────────

pub async fn create_coach(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateCoachRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::enforce_role(&claims.role, "coach")?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_coach(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_coach(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let caller = extract_claims(&req).map(|c| c.sub);
    let profile = service::get_coach(db.get_ref(), &path.into_inner(), caller.as_deref()).await?;
    Ok(response::ok(profile))
}

pub async fn update_coach(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateCoachRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_coach(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_coach(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_coach(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.full_name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}

pub async fn list_coaches(
    db: web::Data<Database>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    Ok(response::ok(service::list_coaches(db.get_ref(), &params).await?))
}

// ── OFFICIAL ──────────────────────────────────────────────────────────────────

pub async fn create_official(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateOfficialRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::enforce_role(&claims.role, "official")?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_official(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_official(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let caller = extract_claims(&req).map(|c| c.sub);
    let profile = service::get_official(db.get_ref(), &path.into_inner(), caller.as_deref()).await?;
    Ok(response::ok(profile))
}

pub async fn update_official(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateOfficialRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_official(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_official(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_official(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.full_name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}

pub async fn list_officials(
    db: web::Data<Database>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    Ok(response::ok(service::list_officials(db.get_ref(), &params).await?))
}

// ── PROMOTER ──────────────────────────────────────────────────────────────────

pub async fn create_promoter(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreatePromoterRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::enforce_role(&claims.role, "promoter")?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_promoter(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_promoter(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let caller = extract_claims(&req).map(|c| c.sub);
    let profile = service::get_promoter(db.get_ref(), &path.into_inner(), caller.as_deref()).await?;
    Ok(response::ok(profile))
}

pub async fn update_promoter(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdatePromoterRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_promoter(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_promoter(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_promoter(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.organization_name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}

pub async fn list_promoters(
    db: web::Data<Database>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    Ok(response::ok(service::list_promoters(db.get_ref(), &params).await?))
}

// ── MATCHMAKER ────────────────────────────────────────────────────────────────

pub async fn create_matchmaker(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateMatchmakerRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::enforce_role(&claims.role, "matchmaker")?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_matchmaker(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_matchmaker(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let caller = extract_claims(&req).map(|c| c.sub);
    let profile = service::get_matchmaker(db.get_ref(), &path.into_inner(), caller.as_deref()).await?;
    Ok(response::ok(profile))
}

pub async fn update_matchmaker(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateMatchmakerRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_matchmaker(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_matchmaker(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_matchmaker(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.full_name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}

pub async fn list_matchmakers(
    db: web::Data<Database>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    Ok(response::ok(service::list_matchmakers(db.get_ref(), &params).await?))
}

// ── FAN ───────────────────────────────────────────────────────────────────────

pub async fn create_fan(
    req: HttpRequest,
    db: web::Data<Database>,
    body: web::Json<CreateFanRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    service::enforce_role(&claims.role, "fan")?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::create_fan(db.get_ref(), &claims.sub, body.into_inner()).await?;
    Ok(response::created(profile))
}

pub async fn get_fan(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let caller = extract_claims(&req).map(|c| c.sub);
    let profile = service::get_fan(db.get_ref(), &path.into_inner(), caller.as_deref()).await?;
    Ok(response::ok(profile))
}

pub async fn update_fan(
    req: HttpRequest,
    db: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdateFanRequest>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let profile = service::update_fan(db.get_ref(), &path.into_inner(), &claims.sub, body.into_inner()).await?;
    Ok(response::ok(profile))
}

pub async fn submit_fan(
    req: HttpRequest,
    db: web::Data<Database>,
    email: web::Data<EmailService>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let claims = require_auth(&req)?;
    let profile = service::submit_fan(db.get_ref(), &path.into_inner(), &claims.sub).await?;
    email.send_profile_submitted(profile.display_name.clone(), profile.id.clone(), profile.role.clone());
    Ok(response::ok(profile))
}

pub async fn list_fans(
    db: web::Data<Database>,
    params: web::Query<PaginationParams>,
) -> Result<HttpResponse, AppError> {
    Ok(response::ok(service::list_fans(db.get_ref(), &params).await?))
}
