pub mod print;
pub mod scan;
pub mod system;
pub mod events;

use actix_web::{HttpResponse, Result};
use crate::models::ApiResponse;

/// Helper function to create JSON success responses
pub fn json_success<T: serde::Serialize>(data: T) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ApiResponse::success(data)))
}

/// Helper function to create JSON error responses
pub fn json_error(message: String) -> Result<HttpResponse> {
    Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(message)))
}

/// Helper function to create internal server error responses
pub fn internal_error(message: String) -> Result<HttpResponse> {
    Ok(HttpResponse::InternalServerError().json(ApiResponse::<()>::error(message)))
}