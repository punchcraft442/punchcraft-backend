use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/notifications")
            .route("", web::get().to(handlers::list_notifications))
            .route("/{id}/read", web::patch().to(handlers::mark_read)),
    );
}
