use actix_web::web;
use super::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            // ── Approval queue ─────────────────────────────────────────────
            .route("/profiles/queue", web::get().to(handlers::get_approval_queue))
            .route("/profiles/{id}/approve", web::post().to(handlers::approve_profile))
            .route("/profiles/{id}/reject", web::post().to(handlers::reject_profile))
            .route("/profiles/{id}/verify", web::patch().to(handlers::set_verification_tier))
            // ── User management (admin + super_admin) ──────────────────────
            .route("/users", web::get().to(handlers::list_users))
            .route("/users/{id}", web::get().to(handlers::get_user))
            .route("/users/{id}/suspend", web::post().to(handlers::suspend_user))
            .route("/users/{id}/activate", web::post().to(handlers::activate_user))
            // ── Super admin only ───────────────────────────────────────────
            .route("/users", web::post().to(handlers::create_user_direct))
            .route("/users/{id}/ban", web::delete().to(handlers::ban_user))
            .route("/users/{id}/role", web::patch().to(handlers::change_user_role)),
    );
}
