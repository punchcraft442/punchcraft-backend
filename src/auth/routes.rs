use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(handlers::register))
            .route("/verify-email", web::get().to(handlers::verify_email))
            .route("/login", web::post().to(handlers::login))
            .route("/refresh", web::post().to(handlers::refresh_token))
            .route("/logout", web::post().to(handlers::logout))
            .route("/forgot-password", web::post().to(handlers::forgot_password))
            .route("/reset-password", web::post().to(handlers::reset_password))
            .route("/change-password", web::patch().to(handlers::change_password)),
    );
}
