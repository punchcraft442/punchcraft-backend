use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/verification")
            .route("/documents", web::post().to(handlers::submit_document))
            .route("/documents/pending", web::get().to(handlers::list_pending))
            .route("/documents/{id}/review", web::post().to(handlers::review_document)),
    );
}
