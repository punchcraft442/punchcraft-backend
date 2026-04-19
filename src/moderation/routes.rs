use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reports")
            .route("", web::post().to(handlers::create_report))
            .route("/admin", web::get().to(handlers::list_reports))
            .route("/admin/{id}/decision", web::post().to(handlers::decide_report)),
    );
}
