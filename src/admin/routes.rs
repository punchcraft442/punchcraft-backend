use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .route("/profiles/{id}/approve", web::post().to(handlers::approve_profile))
            .route("/profiles/{id}/reject", web::post().to(handlers::reject_profile)),
    );
}
