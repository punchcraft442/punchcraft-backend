use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, to_bson},
    Database,
};

use crate::common::errors::AppError;
use super::models::*;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn col<T: Send + Sync>(db: &Database, name: &str) -> mongodb::Collection<T> {
    db.collection::<T>(name)
}

fn bson_err(e: mongodb::bson::ser::Error) -> AppError {
    AppError::BadRequest(e.to_string())
}

// ── profiles collection ───────────────────────────────────────────────────────

pub fn col_profiles(db: &Database) -> mongodb::Collection<Profile> {
    db.collection("profiles")
}

pub async fn insert_profile(db: &Database, p: &Profile) -> Result<ObjectId, AppError> {
    let r = col::<Profile>(db, "profiles").insert_one(p).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_profile_by_id(db: &Database, id: ObjectId) -> Result<Option<Profile>, AppError> {
    Ok(col::<Profile>(db, "profiles").find_one(doc! { "_id": id }).await?)
}

pub async fn find_profile_by_user_and_role(
    db: &Database,
    user_id: ObjectId,
    role: &str,
) -> Result<Option<Profile>, AppError> {
    Ok(col::<Profile>(db, "profiles")
        .find_one(doc! { "userId": user_id, "role": role })
        .await?)
}

pub async fn update_profile(
    db: &Database,
    id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<Profile>(db, "profiles")
        .update_one(doc! { "_id": id }, doc! { "$set": update })
        .await?;
    Ok(())
}

pub async fn list_profiles(
    db: &Database,
    role: Option<&str>,
    params: &PaginationParams,
) -> Result<(Vec<Profile>, u64), AppError> {
    let mut filter = doc! { "searchable": true };
    if let Some(r) = role {
        filter.insert("role", r);
    }
    if let Some(kw) = &params.keyword {
        filter.insert("displayName", doc! { "$regex": kw, "$options": "i" });
    }
    if let Some(r) = &params.region {
        filter.insert("location.region", r.as_str());
    }
    if let Some(c) = &params.city {
        filter.insert("location.city", c.as_str());
    }
    if let Some(t) = &params.verification_tier {
        filter.insert("verificationTier", t.as_str());
    }
    if let Some(wc) = &params.weight_class {
        filter.insert("weightClass", wc.as_str());
    }

    let c = col::<Profile>(db, "profiles");
    let total = c.count_documents(filter.clone()).await?;
    let opts = mongodb::options::FindOptions::builder()
        .skip(params.skip())
        .limit(params.limit() as i64)
        .build();
    let items: Vec<Profile> = c.find(filter).with_options(opts).await?.try_collect().await?;
    Ok((items, total))
}

// ── fighterDetails ────────────────────────────────────────────────────────────

pub async fn insert_fighter(db: &Database, d: &FighterDetails) -> Result<ObjectId, AppError> {
    let r = col::<FighterDetails>(db, "fighterDetails").insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_fighter(db: &Database, profile_id: ObjectId) -> Result<Option<FighterDetails>, AppError> {
    Ok(col::<FighterDetails>(db, "fighterDetails")
        .find_one(doc! { "profileId": profile_id })
        .await?)
}

pub async fn update_fighter(
    db: &Database,
    profile_id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<FighterDetails>(db, "fighterDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$set": update })
        .await?;
    Ok(())
}

pub async fn push_fight_history(
    db: &Database,
    profile_id: ObjectId,
    entry: &FightHistoryEntry,
) -> Result<(), AppError> {
    let bson = to_bson(entry).map_err(bson_err)?;
    col::<FighterDetails>(db, "fighterDetails")
        .update_one(
            doc! { "profileId": profile_id },
            doc! { "$push": { "fightHistory": bson } },
        )
        .await?;
    Ok(())
}

pub async fn pull_fight_history(
    db: &Database,
    profile_id: ObjectId,
    fight_id: &str,
) -> Result<(), AppError> {
    col::<FighterDetails>(db, "fighterDetails")
        .update_one(
            doc! { "profileId": profile_id },
            doc! { "$pull": { "fightHistory": { "id": fight_id } } },
        )
        .await?;
    Ok(())
}

// ── gymDetails ────────────────────────────────────────────────────────────────

pub async fn insert_gym(db: &Database, d: &GymDetails) -> Result<ObjectId, AppError> {
    let r = col::<GymDetails>(db, "gymDetails").insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_gym(db: &Database, profile_id: ObjectId) -> Result<Option<GymDetails>, AppError> {
    Ok(col::<GymDetails>(db, "gymDetails")
        .find_one(doc! { "profileId": profile_id })
        .await?)
}

pub async fn update_gym(
    db: &Database,
    profile_id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<GymDetails>(db, "gymDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$set": update })
        .await?;
    Ok(())
}

pub async fn gym_add_coach(db: &Database, profile_id: ObjectId, coach_id: &str) -> Result<(), AppError> {
    col::<GymDetails>(db, "gymDetails")
        .update_one(
            doc! { "profileId": profile_id },
            doc! { "$addToSet": { "linkedCoachIds": coach_id } },
        )
        .await?;
    Ok(())
}

pub async fn gym_remove_coach(db: &Database, profile_id: ObjectId, coach_id: &str) -> Result<(), AppError> {
    col::<GymDetails>(db, "gymDetails")
        .update_one(
            doc! { "profileId": profile_id },
            doc! { "$pull": { "linkedCoachIds": coach_id } },
        )
        .await?;
    Ok(())
}

pub async fn gym_add_fighter(db: &Database, profile_id: ObjectId, fighter_id: &str) -> Result<(), AppError> {
    col::<GymDetails>(db, "gymDetails")
        .update_one(
            doc! { "profileId": profile_id },
            doc! { "$addToSet": { "rosterFighterIds": fighter_id } },
        )
        .await?;
    Ok(())
}

pub async fn gym_remove_fighter(db: &Database, profile_id: ObjectId, fighter_id: &str) -> Result<(), AppError> {
    col::<GymDetails>(db, "gymDetails")
        .update_one(
            doc! { "profileId": profile_id },
            doc! { "$pull": { "rosterFighterIds": fighter_id } },
        )
        .await?;
    Ok(())
}

// ── coachDetails ──────────────────────────────────────────────────────────────

pub async fn insert_coach(db: &Database, d: &CoachDetails) -> Result<ObjectId, AppError> {
    let r = col::<CoachDetails>(db, "coachDetails").insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_coach(db: &Database, profile_id: ObjectId) -> Result<Option<CoachDetails>, AppError> {
    Ok(col::<CoachDetails>(db, "coachDetails")
        .find_one(doc! { "profileId": profile_id })
        .await?)
}

pub async fn update_coach(
    db: &Database,
    profile_id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<CoachDetails>(db, "coachDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$set": update })
        .await?;
    Ok(())
}

pub async fn coach_add_gym(db: &Database, profile_id: ObjectId, gym_id: &str) -> Result<(), AppError> {
    col::<CoachDetails>(db, "coachDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$addToSet": { "linkedGymIds": gym_id } })
        .await?;
    Ok(())
}

pub async fn coach_remove_gym(db: &Database, profile_id: ObjectId, gym_id: &str) -> Result<(), AppError> {
    col::<CoachDetails>(db, "coachDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$pull": { "linkedGymIds": gym_id } })
        .await?;
    Ok(())
}

pub async fn coach_add_fighter(db: &Database, profile_id: ObjectId, fighter_id: &str) -> Result<(), AppError> {
    col::<CoachDetails>(db, "coachDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$addToSet": { "associatedFighterIds": fighter_id } })
        .await?;
    Ok(())
}

pub async fn coach_remove_fighter(db: &Database, profile_id: ObjectId, fighter_id: &str) -> Result<(), AppError> {
    col::<CoachDetails>(db, "coachDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$pull": { "associatedFighterIds": fighter_id } })
        .await?;
    Ok(())
}

// ── officialDetails ───────────────────────────────────────────────────────────

pub async fn insert_official(db: &Database, d: &OfficialDetails) -> Result<ObjectId, AppError> {
    let r = col::<OfficialDetails>(db, "officialDetails").insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_official(db: &Database, profile_id: ObjectId) -> Result<Option<OfficialDetails>, AppError> {
    Ok(col::<OfficialDetails>(db, "officialDetails")
        .find_one(doc! { "profileId": profile_id })
        .await?)
}

pub async fn update_official(
    db: &Database,
    profile_id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<OfficialDetails>(db, "officialDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$set": update })
        .await?;
    Ok(())
}

// ── promoterDetails ───────────────────────────────────────────────────────────

pub async fn insert_promoter(db: &Database, d: &PromoterDetails) -> Result<ObjectId, AppError> {
    let r = col::<PromoterDetails>(db, "promoterDetails").insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_promoter(db: &Database, profile_id: ObjectId) -> Result<Option<PromoterDetails>, AppError> {
    Ok(col::<PromoterDetails>(db, "promoterDetails")
        .find_one(doc! { "profileId": profile_id })
        .await?)
}

pub async fn update_promoter(
    db: &Database,
    profile_id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<PromoterDetails>(db, "promoterDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$set": update })
        .await?;
    Ok(())
}

// ── matchmakerDetails ─────────────────────────────────────────────────────────

pub async fn insert_matchmaker(db: &Database, d: &MatchmakerDetails) -> Result<ObjectId, AppError> {
    let r = col::<MatchmakerDetails>(db, "matchmakerDetails").insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_matchmaker(db: &Database, profile_id: ObjectId) -> Result<Option<MatchmakerDetails>, AppError> {
    Ok(col::<MatchmakerDetails>(db, "matchmakerDetails")
        .find_one(doc! { "profileId": profile_id })
        .await?)
}

pub async fn update_matchmaker(
    db: &Database,
    profile_id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<MatchmakerDetails>(db, "matchmakerDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$set": update })
        .await?;
    Ok(())
}

// ── fanDetails ────────────────────────────────────────────────────────────────

pub async fn insert_fan(db: &Database, d: &FanDetails) -> Result<ObjectId, AppError> {
    let r = col::<FanDetails>(db, "fanDetails").insert_one(d).await?;
    Ok(r.inserted_id.as_object_id().unwrap())
}

pub async fn find_fan(db: &Database, profile_id: ObjectId) -> Result<Option<FanDetails>, AppError> {
    Ok(col::<FanDetails>(db, "fanDetails")
        .find_one(doc! { "profileId": profile_id })
        .await?)
}

pub async fn update_fan(
    db: &Database,
    profile_id: ObjectId,
    update: mongodb::bson::Document,
) -> Result<(), AppError> {
    col::<FanDetails>(db, "fanDetails")
        .update_one(doc! { "profileId": profile_id }, doc! { "$set": update })
        .await?;
    Ok(())
}
