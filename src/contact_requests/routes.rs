use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/contact-requests")
            .route("", web::post().to(handlers::create))
            .route("", web::get().to(handlers::list))
            .route("/{id}", web::patch().to(handlers::update_status)),
    );
}
