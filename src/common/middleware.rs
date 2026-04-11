use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // user_id
    pub role: String,
    pub exp: usize,
}

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            let token = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            if let Some(token) = token {
                let secret = std::env::var("JWT_SECRET").unwrap_or_default();
                let key = DecodingKey::from_secret(secret.as_bytes());
                let validation = Validation::new(Algorithm::HS256);

                if let Ok(token_data) = decode::<Claims>(token, &key, &validation) {
                    req.extensions_mut().insert(token_data.claims);
                }
            }

            service.call(req).await
        })
    }
}

/// Extract claims from request — returns None if unauthenticated.
pub fn extract_claims(req: &actix_web::HttpRequest) -> Option<Claims> {
    req.extensions().get::<Claims>().cloned()
}

/// Require authentication — returns Err(Unauthorized) if missing.
pub fn require_auth(req: &actix_web::HttpRequest) -> Result<Claims, crate::common::errors::AppError> {
    extract_claims(req).ok_or(crate::common::errors::AppError::Unauthorized)
}

/// Require admin or super_admin role.
pub fn require_admin(req: &actix_web::HttpRequest) -> Result<Claims, crate::common::errors::AppError> {
    let claims = require_auth(req)?;
    if claims.role == "admin" || claims.role == "super_admin" {
        Ok(claims)
    } else {
        Err(crate::common::errors::AppError::Forbidden)
    }
}
