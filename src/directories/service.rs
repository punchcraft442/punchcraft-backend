use mongodb::Database;
use serde::Deserialize;

use crate::common::errors::AppError;
use crate::profiles::{models::{PaginationParams, PaginatedResponse, ProfileSummary}, repository};

/// Query parameters for GET /directories
#[derive(Debug, Deserialize)]
pub struct DirectoryQuery {
    pub role: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub verification_tier: Option<String>,
    pub keyword: Option<String>,
    pub weight_class: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

/// Returns only approved + searchable profiles across all roles (or filtered by role).
pub async fn list(
    db: &Database,
    query: DirectoryQuery,
) -> Result<PaginatedResponse<ProfileSummary>, AppError> {
    let params = PaginationParams {
        page: query.page,
        limit: query.limit,
        keyword: query.keyword,
        region: query.region,
        city: query.city,
        verification_tier: query.verification_tier,
        weight_class: query.weight_class,
        sort: None,
    };

    let role = query.role.as_deref();
    let (profiles, total) = repository::list_profiles(db, role, &params).await?;

    let limit = params.limit() as u64;
    let total_pages = if limit == 0 { 0 } else { (total + limit - 1) / limit };

    use crate::profiles::models::PaginationMeta;
    Ok(PaginatedResponse {
        items: profiles.into_iter().map(ProfileSummary::from).collect(),
        pagination: PaginationMeta {
            page: params.page(),
            limit: params.limit(),
            total_items: total,
            total_pages,
        },
    })
}
