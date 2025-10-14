use actix_web::{web, HttpRequest, HttpResponse, Result};
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError};
use sqlx::{SqlitePool};
use uuid::Uuid;

use crate::handlers::{json_success, json_error, internal_error};
use crate::models::{ScanJob, ScanRequest, ScanJobStatus, ScanJobQueue, add_to_scan_queue, notify_scan_queue};
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
pub async fn start_scan(req: web::Json<ScanRequest>, pool: web::Data<SqlitePool>, s_queue: web::Data<ScanJobQueue>) -> Result<HttpResponse> {
    let sane_service = SaneService::new();

    if !sane_service.is_available().await {
        return json_error("SANE service is not available".to_string());
    }

    let scanner_name = req.scanner.clone()
        .ok_or_else(|| ErrorBadRequest("Scanner must be specified"))?;

    // Create scan job
    let scan_job = ScanJob::new(scanner_name, req.into_inner());
    let job_id = scan_job.id;

    // Store job in database
    scan_job.save_to_db(&pool)
        .await
        .map_err(|e| ErrorBadRequest(e.to_string()))?;

    log::info!("Scan job {} saved to database", job_id);
    
    add_to_scan_queue(&s_queue, scan_job)
        .await
        .map_err(|e| ErrorInternalServerError(e.to_string()))?;

    log::warn!("Notify the queue after adding job");

    tokio::spawn(async move {
        if let Err(e) = notify_scan_queue(&s_queue, &pool).await {
            log::error!("Failed to notify scan queue: {}", e);
        };
    });

    json_success(serde_json::json!({
        "job_id": job_id,
        "status": "queued"
    }))
}

/// GET /api/scan/jobs - List all scan jobs
pub async fn list_scan_jobs(pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    //change to all in the future
    match ScanJob::get_all(pool.as_ref()).await {
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


/// DELETE /api/scan/jobs/{job_id} - Delete specific scan job form database
pub async fn delete_scan_job_record(path: web::Path<Uuid>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    match ScanJob::remove_by_uuid(job_id, pool.as_ref()).await {
        Ok(_) => json_success(format!("Successfully removed job {}", job_id)),
        Err(e) => internal_error(format!("Failed to find job: {}", e)),
    }
}

/// DELETE /api/scan/remove/{job_id} - Delete specific scan file
pub async fn delete_scan_file(path: web::Path<Uuid>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    if let Some(scan_job) = ScanJob::find_by_uuid(job_id, pool.as_ref())
        .await
        .map_err(|e| ErrorInternalServerError(e.to_string()))? {

        let filename = match scan_job.output_filename {
            Some(filename) => filename,
            None => return Err(ErrorInternalServerError("Validation error".to_string())),
        };

        delete_scan(filename.clone(), &pool)
            .await
            .map_err(|e| ErrorInternalServerError(e.to_string()))?;

        return json_success(format!("Successfully removed scan {}", filename));
    };

    internal_error(format!("Could not find scan job {job_id}"))
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
                    match actix_files::NamedFile::open_async(&file_path).await {
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




async fn delete_scan(filename: String, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::remove_file(format!("scans/{}", filename)) {
        Ok(_) => {
            ScanJob::update_file_available_by_filename(filename.clone(), false, pool).await
                .map_err(|e| format!("Failed to delete scan: {}", e))?;
        },
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    log::warn!("File {} does not exist", filename);
                },
                _ => return Err(Box::new(e)),
            }
        }
    }

    Ok(())
}