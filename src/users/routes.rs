use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .route("/me", web::get().to(handlers::get_me))
            .route("/me", web::patch().to(handlers::update_me)),
    );
}
