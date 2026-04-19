use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/favorites")
            .route("", web::post().to(handlers::add_favorite))
            .route("", web::get().to(handlers::list_favorites))
            .route("/{id}", web::delete().to(handlers::remove_favorite)),
    );
}
