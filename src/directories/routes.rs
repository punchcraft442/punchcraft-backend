use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/directories")
            .route("", web::get().to(handlers::list)),
    );
}
