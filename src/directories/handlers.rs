use actix_web::{web, HttpResponse};
use mongodb::Database;

use crate::common::{errors::AppError, response};
use super::service::{self, DirectoryQuery};

pub async fn list(
    db: web::Data<Database>,
    query: web::Query<DirectoryQuery>,
) -> Result<HttpResponse, AppError> {
    let results = service::list(db.get_ref(), query.into_inner()).await?;
    Ok(response::ok(results))
}
