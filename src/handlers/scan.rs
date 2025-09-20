use actix_web::{web, HttpRequest, HttpResponse, Result};
use std::sync::{Arc, Mutex};
use actix_web::error::ErrorBadRequest;
use actix_web::http::StatusCode;
use sqlx::{SqlitePool};
use uuid::Uuid;

use crate::handlers::{json_success, json_error, internal_error};
use crate::models::{ScanJob, ScanRequest, ScanJobStatus};
use crate::services::sane::SaneService;


/// GET /api/scanners - List all available scanners
pub async fn list_scanners() -> Result<HttpResponse> {
    let sane_service = SaneService::new();

    if !sane_service.is_available().await {
        return json_error("SANE service is not available".to_string());
    }

    match sane_service.get_scanners().await {
        Ok(scanners) => json_success(scanners),
        Err(e) => internal_error(format!("Failed to get scanners: {}", e)),
    }
}

/// POST /api/scan - Start a scan job
pub async fn start_scan(req: web::Json<ScanRequest>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let sane_service = SaneService::new();

    if !sane_service.is_available().await {
        return json_error("SANE service is not available".to_string());
    }

    let scanner_name = req.scanner.clone()
        .ok_or_else(|| json_error("Scanner must be specified".to_string())).map_err(|_| ErrorBadRequest("Validation error".to_string()))?;

    // Create scan job
    let scan_job = ScanJob::new(scanner_name, req.into_inner());
    let job_id = scan_job.id;

    // Store job
    match scan_job.save_to_db(&pool).await.map_err(|e| ErrorBadRequest(e.to_string())) {
        Ok(added_rows) => {
            log::info!("Successfully added {added_rows}row.");

            tokio::spawn(async move {
                if let Err(e) = execute_scan_job(scan_job.id, &pool).await {
                    log::error!("Scan job {} failed: {}", job_id, e);
                }
            });
            
            json_success(serde_json::json!({
                "job_id": job_id,
                "status": "queued"
            }))
        }
        Err(e) => {
            json_error(e.to_string())
        }
    }
}

/// GET /api/scan/jobs - List all scan jobs
pub async fn list_scan_jobs(pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    //change to all in the future
    match ScanJob::get_recent(10, pool.as_ref()).await {
        Ok(jobs) => json_success(jobs),
        Err(e) => internal_error(format!("Failed to get recent jobs: {}", e)),
    }
}

/// GET /api/scan/jobs/{job_id} - Get specific scan job
pub async fn get_scan_job(path: web::Path<Uuid>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    match ScanJob::find_by_uuid(job_id, pool.as_ref()).await {
        Ok(job) => json_success(job),
        Err(e) => internal_error(format!("Failed to find job: {}", e)),
    }
}

/// GET /api/scan/download/{job_id} - Download scanned file
pub async fn download_scan(path: web::Path<Uuid>, req: HttpRequest, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    let job = match ScanJob::find_by_uuid(job_id, pool.as_ref()).await {
        Ok(job) => job,
        Err(e) => return json_error(e.to_string()),
    };

    match job {
        Some(job) => {
            if let ScanJobStatus::Completed = job.status {
                if let Some(file_path) = job.get_file_path() {
                    match actix_files::NamedFile::open(&file_path) {
                        Ok(file) => Ok(file.into_response(&req)),
                        Err(_) => json_error("Scan file not found".to_string()),
                    }
                } else {
                    json_error("No output file available".to_string())
                }
            } else {
                json_error("Scan not completed".to_string())
            }
        },
        None => json_error("Scan job not found".to_string()),
    }
}

/// Background task to execute scan job
async fn execute_scan_job(job_id: Uuid, pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let sane_service = SaneService::new();

    // Get job from storage
    let mut job = match ScanJob::find_by_uuid(job_id, pool).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            log::warn!("Scan job {} not found in storage", job_id);
            return Err(sqlx::Error::RowNotFound);
        },
        Err(e) => { return Err(e.into()) }
    };

    // Update status to scanning
    job.set_status(ScanJobStatus::Scanning);
    job.update_in_db(pool).await.map_err(|e| e.to_string()).unwrap();

    // Execute the scan
    match sane_service.start_scan(&job).await {
        Ok(output_path) => {
            if let Ok(metadata) = std::fs::metadata(&output_path) {
                job.file_size = Some(metadata.len());
            }
            job.set_status(ScanJobStatus::Completed);
            job.update_in_db(pool).await?;

            log::info!("Scan job {} completed successfully", job_id);
        },
        Err(e) => {
            job.set_error(e.clone());
            job.update_in_db(pool).await?;

            log::error!("Scan job {} failed: {}", job_id, e);
        }
    }
    Ok(())
}