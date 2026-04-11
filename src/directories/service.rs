use mongodb::{bson::doc, Database};
use futures_util::TryStreamExt;
use serde::Deserialize;

use crate::common::errors::AppError;
use crate::profiles::models::{Profile, ProfileResponse};

/// Query parameters for GET /directories
#[derive(Debug, Deserialize)]
pub struct DirectoryQuery {
    pub role: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub verification_tier: Option<String>,
}

/// Returns only approved + public + searchable profiles.
pub async fn list(db: &Database, query: DirectoryQuery) -> Result<Vec<ProfileResponse>, AppError> {
    let mut filter = doc! {
        "status": "approved",
        "visibility": "public",
        "searchable": true,
    };

    if let Some(role) = query.role {
        filter.insert("role", role);
    }
    if let Some(region) = query.region {
        filter.insert("location.region", region);
    }
    if let Some(city) = query.city {
        filter.insert("location.city", city);
    }
    if let Some(tier) = query.verification_tier {
        filter.insert("verification_tier", tier);
    }

    let col = db.collection::<Profile>("profiles");
    let cursor = col.find(filter).await?;
    let profiles: Vec<Profile> = cursor.try_collect().await?;

    Ok(profiles.into_iter().map(ProfileResponse::from).collect())
}
