use punchcraft::{admin, auth, common, contact_requests, directories, docs, favorites, media, moderation, notifications, profiles, verification};

use actix_cors::Cors;
use actix_web::{http, web, App, HttpServer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Fail fast if Cloudinary credentials are missing
    for key in &["CLOUDINARY_CLOUD_NAME", "CLOUDINARY_API_KEY", "CLOUDINARY_API_SECRET"] {
        std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in .env"));
    }

    let mongo_uri = std::env::var("MONGODB_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());

    let db_name = std::env::var("DB_NAME")
        .unwrap_or_else(|_| "punchcraft".to_string());

    let db = common::db::connect(&mongo_uri, &db_name).await?;
    let db_data = web::Data::new(db);

    let email_svc = common::email::EmailService::new(
        std::env::var("RESEND_API_KEY").expect("RESEND_API_KEY must be set"),
        std::env::var("EMAIL_FROM").unwrap_or_else(|_| "PunchCraft <noreply@thepunchcraft.com>".to_string()),
        std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@thepunchcraft.com".to_string()),
    );
    let email_data = web::Data::new(email_svc);

    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    tracing::info!("Starting PunchCraft API on {}", bind_addr);

    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let frontend_url = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    HttpServer::new(move || {
        let cors = if app_env == "production" {
            Cors::default()
                .allowed_origin(&frontend_url)
                .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE", "OPTIONS"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::CONTENT_TYPE])
                .max_age(3600)
        } else {
            Cors::default()
                .allowed_origin_fn(|_, _| true)
                .allowed_methods(vec!["GET", "POST", "PATCH", "DELETE", "OPTIONS"])
                .allowed_headers(vec![http::header::AUTHORIZATION, http::header::CONTENT_TYPE])
                .max_age(3600)
        };

        App::new()
            .wrap(cors)
            .wrap(common::middleware::AuthMiddleware)
            .app_data(db_data.clone())
            .app_data(email_data.clone())
            .configure(docs::configure)
            .service(
                web::scope("/api/v1")
                    .configure(auth::routes::configure)
                    .configure(profiles::routes::configure)
                    .configure(directories::routes::configure)
                    .configure(verification::routes::configure)
                    .configure(media::routes::configure)
                    .configure(moderation::routes::configure)
                    .configure(notifications::routes::configure)
                    .configure(contact_requests::routes::configure)
                    .configure(favorites::routes::configure)
                    .configure(admin::routes::configure)
            )
    })
    .bind(&bind_addr)?
    .run()
    .await?;

    Ok(())
}
