use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/media")
            .route("/upload", web::post().to(handlers::upload_media))
            .route("/{id}", web::delete().to(handlers::delete_media)),
    );
}
