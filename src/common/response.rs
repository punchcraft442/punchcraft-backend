use actix_web::HttpResponse;
use serde::Serialize;
use serde_json::json;

pub fn ok<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "success": true, "data": data }))
}

pub fn created<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Created().json(json!({ "success": true, "data": data }))
}

pub fn no_content() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

pub fn ok_message(message: &str) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "success": true, "message": message }))
}
