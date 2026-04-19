use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/profiles")
            // ── Fighters ──────────────────────────────────────────────────────
            .service(
                web::scope("/fighters")
                    .route("", web::post().to(handlers::create_fighter))
                    .route("", web::get().to(handlers::list_fighters))
                    .route("/{id}", web::get().to(handlers::get_fighter))
                    .route("/{id}", web::patch().to(handlers::update_fighter))
                    .route("/{id}/submit", web::post().to(handlers::submit_fighter))
                    .route("/{id}/request-revision", web::post().to(handlers::request_revision))
                    .route("/{id}/profile-image", web::post().to(handlers::upload_profile_image))
                    .route("/{id}/cover-image", web::post().to(handlers::upload_cover_image))
                    .route("/{id}/fight-history", web::post().to(handlers::add_fight_history))
                    .route("/{id}/fight-history/{fight_id}", web::delete().to(handlers::delete_fight_history)),
            )
            // ── Gyms ──────────────────────────────────────────────────────────
            .service(
                web::scope("/gyms")
                    .route("", web::post().to(handlers::create_gym))
                    .route("", web::get().to(handlers::list_gyms))
                    .route("/{id}", web::get().to(handlers::get_gym))
                    .route("/{id}", web::patch().to(handlers::update_gym))
                    .route("/{id}/submit", web::post().to(handlers::submit_gym))
                    .route("/{id}/request-revision", web::post().to(handlers::request_revision))
                    .route("/{id}/profile-image", web::post().to(handlers::upload_profile_image))
                    .route("/{id}/cover-image", web::post().to(handlers::upload_cover_image))
                    .route("/{id}/coaches/{coach_id}", web::post().to(handlers::gym_link_coach))
                    .route("/{id}/coaches/{coach_id}", web::delete().to(handlers::gym_unlink_coach))
                    .route("/{id}/fighters/{fighter_id}", web::post().to(handlers::gym_link_fighter))
                    .route("/{id}/fighters/{fighter_id}", web::delete().to(handlers::gym_unlink_fighter)),
            )
            // ── Coaches ───────────────────────────────────────────────────────
            .service(
                web::scope("/coaches")
                    .route("", web::post().to(handlers::create_coach))
                    .route("", web::get().to(handlers::list_coaches))
                    .route("/{id}", web::get().to(handlers::get_coach))
                    .route("/{id}", web::patch().to(handlers::update_coach))
                    .route("/{id}/submit", web::post().to(handlers::submit_coach))
                    .route("/{id}/request-revision", web::post().to(handlers::request_revision))
                    .route("/{id}/profile-image", web::post().to(handlers::upload_profile_image))
                    .route("/{id}/cover-image", web::post().to(handlers::upload_cover_image)),
            )
            // ── Officials ─────────────────────────────────────────────────────
            .service(
                web::scope("/officials")
                    .route("", web::post().to(handlers::create_official))
                    .route("", web::get().to(handlers::list_officials))
                    .route("/{id}", web::get().to(handlers::get_official))
                    .route("/{id}", web::patch().to(handlers::update_official))
                    .route("/{id}/submit", web::post().to(handlers::submit_official))
                    .route("/{id}/request-revision", web::post().to(handlers::request_revision))
                    .route("/{id}/profile-image", web::post().to(handlers::upload_profile_image))
                    .route("/{id}/cover-image", web::post().to(handlers::upload_cover_image)),
            )
            // ── Promoters ─────────────────────────────────────────────────────
            .service(
                web::scope("/promoters")
                    .route("", web::post().to(handlers::create_promoter))
                    .route("", web::get().to(handlers::list_promoters))
                    .route("/{id}", web::get().to(handlers::get_promoter))
                    .route("/{id}", web::patch().to(handlers::update_promoter))
                    .route("/{id}/submit", web::post().to(handlers::submit_promoter))
                    .route("/{id}/request-revision", web::post().to(handlers::request_revision))
                    .route("/{id}/profile-image", web::post().to(handlers::upload_profile_image))
                    .route("/{id}/cover-image", web::post().to(handlers::upload_cover_image)),
            )
            // ── Matchmakers ───────────────────────────────────────────────────
            .service(
                web::scope("/matchmakers")
                    .route("", web::post().to(handlers::create_matchmaker))
                    .route("", web::get().to(handlers::list_matchmakers))
                    .route("/{id}", web::get().to(handlers::get_matchmaker))
                    .route("/{id}", web::patch().to(handlers::update_matchmaker))
                    .route("/{id}/submit", web::post().to(handlers::submit_matchmaker))
                    .route("/{id}/request-revision", web::post().to(handlers::request_revision))
                    .route("/{id}/profile-image", web::post().to(handlers::upload_profile_image))
                    .route("/{id}/cover-image", web::post().to(handlers::upload_cover_image)),
            )
            // ── Fans ──────────────────────────────────────────────────────────
            .service(
                web::scope("/fans")
                    .route("", web::post().to(handlers::create_fan))
                    .route("", web::get().to(handlers::list_fans))
                    .route("/{id}", web::get().to(handlers::get_fan))
                    .route("/{id}", web::patch().to(handlers::update_fan))
                    .route("/{id}/submit", web::post().to(handlers::submit_fan))
                    .route("/{id}/request-revision", web::post().to(handlers::request_revision))
                    .route("/{id}/profile-image", web::post().to(handlers::upload_profile_image))
                    .route("/{id}/cover-image", web::post().to(handlers::upload_cover_image)),
            ),
    );
}
