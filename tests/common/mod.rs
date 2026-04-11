use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use mongodb::Database;
use std::sync::OnceLock;
use uuid::Uuid;

use punchcraft::common::{email::EmailService, middleware::Claims};

static INIT: OnceLock<()> = OnceLock::new();

/// Loads .env once and sets a stable JWT secret for all tests.
pub fn init() {
    INIT.get_or_init(|| {
        dotenvy::dotenv().ok();
        // Override with a known test secret so generated tokens are verifiable.
        unsafe { std::env::set_var("JWT_SECRET", test_jwt_secret()) };
    });
}

pub fn test_jwt_secret() -> &'static str {
    "punchcraft_test_secret"
}

/// Creates a fresh uniquely-named test database. Drop it after the test.
pub async fn setup_db() -> Database {
    init();
    let uri = std::env::var("MONGODB_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let uuid = Uuid::new_v4().to_string().replace("-", "");
    let db_name = format!("pct_{}", &uuid[..28]); // max 38 chars on Atlas
    let client = mongodb::Client::with_uri_str(&uri).await.unwrap();
    client.database(&db_name)
}

/// Drops the test database.
pub async fn teardown_db(db: &Database) {
    db.drop().await.ok();
}

/// A no-op EmailService suitable for tests (API calls will fail silently).
pub fn test_email_service() -> EmailService {
    EmailService::new(
        "test_key".to_string(),
        "Test <test@resend.dev>".to_string(),
        "admin@example.com".to_string(),
    )
}

/// Builds a JWT signed with the test secret.
pub fn make_jwt(user_id: &str, role: &str) -> String {
    let claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp: (Utc::now() + chrono::Duration::days(1)).timestamp() as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(test_jwt_secret().as_bytes()),
    )
    .unwrap()
}

/// Builds the full actix App wired with a given DB and the test EmailService.
#[macro_export]
macro_rules! build_app {
    ($db:expr) => {{
        use actix_web::{web, App};
        use punchcraft::{
            admin, auth, directories, media, moderation, notifications, profiles, verification,
        };

        let db_data = web::Data::new($db.clone());
        let email_data = web::Data::new(crate::common::test_email_service());

        actix_web::test::init_service(
            App::new()
                .wrap(punchcraft::common::middleware::AuthMiddleware)
                .app_data(db_data)
                .app_data(email_data)
                .service(
                    web::scope("/api/v1")
                        .configure(auth::routes::configure)
                        .configure(profiles::routes::configure)
                        .configure(directories::routes::configure)
                        .configure(verification::routes::configure)
                        .configure(media::routes::configure)
                        .configure(moderation::routes::configure)
                        .configure(notifications::routes::configure)
                        .configure(admin::routes::configure),
                ),
        )
        .await
    }};
}
