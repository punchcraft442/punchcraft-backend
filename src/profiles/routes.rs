use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/profiles")
            .route("", web::post().to(handlers::create_profile))
            .route("/{id}", web::get().to(handlers::get_profile))
            .route("/{id}", web::patch().to(handlers::update_profile))
            .route("/{id}/submit", web::post().to(handlers::submit_profile)),
    );
}
