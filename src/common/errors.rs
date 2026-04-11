use actix_web::HttpResponse;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found")]
    NotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("{0}")]
    ForbiddenMsg(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal error")]
    Internal(#[from] anyhow::Error),

    #[error("MongoDB error")]
    Mongo(#[from] mongodb::error::Error),
}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, message) = match self {
            AppError::NotFound => (actix_web::http::StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized => (actix_web::http::StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (actix_web::http::StatusCode::FORBIDDEN, self.to_string()),
            AppError::ForbiddenMsg(msg) => (actix_web::http::StatusCode::FORBIDDEN, msg.clone()),
            AppError::BadRequest(msg) => (actix_web::http::StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (actix_web::http::StatusCode::CONFLICT, msg.clone()),
            AppError::Internal(_) | AppError::Mongo(_) => {
                tracing::error!("Internal error: {:?}", self);
                (
                    actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        HttpResponse::build(status).json(json!({
            "success": false,
            "message": message
        }))
    }
}
