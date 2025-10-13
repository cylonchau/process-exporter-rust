pub mod register;
pub mod metrics;

pub use register::{register_process, unregister_process, list_processes};
pub use metrics::get_metrics;

use actix_web::{HttpResponse, Responder};

pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy"
    }))
}