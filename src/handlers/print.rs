use actix_web::{web, HttpResponse, Result};
use actix_multipart::Multipart;
use futures_util::{TryFutureExt, TryStreamExt};
use std::collections::HashMap;
use uuid::Uuid;
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError};
use sqlx::SqlitePool;
use crate::handlers::{json_success, json_error, internal_error};
use crate::models::{PrintJob, PrintRequest, PrintJobStatus, PrintPageSize, AppState, add_to_job_queue, Job, notify_scan_queue, JobQueue};
use crate::services::cups::CupsService;



/// GET /api/printers - List all available printers
pub async fn list_printers(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let printers = app_state.get_printers().await;

    json_success(printers)
}

/// POST /api/print - Submit a print job
pub async fn submit_print_job(mut payload: Multipart, pool: web::Data<SqlitePool>, job_queue: web::Data<JobQueue>, app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let cups_service = CupsService::new();

    if !cups_service.is_available().await {
        return json_error("CUPS service is not available".to_string());
    }

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut form_data: HashMap<String, String> = HashMap::new();
    
    while let Some(mut field) = payload.try_next().await.map_err(|e| {
        log::error!("Error reading multipart field: {}", e);
    }).unwrap_or(None) {
        let content_disposition = field.content_disposition().unwrap();
        let field_name = content_disposition.get_name().map(|s| s.to_string());
        let file_name = content_disposition.get_filename().map(|s| s.to_string());

        if let Some(field_name) = field_name {
            if field_name == "file" {
                if let Some(file_name) = file_name {
                    filename = Some(file_name);
                }

                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| {
                    log::error!("Error reading file chunk: {}", e);
                }).unwrap_or(None) {
                    bytes.extend_from_slice(&chunk);
                }
                file_data = Some(bytes);
            } else {
                let mut field_data = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| {
                    log::error!("Error reading form field: {}", e);
                }).unwrap_or(None) {
                    field_data.extend_from_slice(&chunk);
                }

                if let Ok(value) = String::from_utf8(field_data) {
                    form_data.insert(field_name, value);
                }
            }
        }
    }

    let file_data = match file_data {
        Some(data) => data,
        None => {
            log::warn!("No file provided in print request");
            return json_error("No file provided".to_string());
        }
    };

    let filename = match filename {
        Some(name) => name,
        None => {
            log::warn!("No filename provided in print request");
            return json_error("No filename provided".to_string());
        }
    };
    
    let print_request = PrintRequest {
        printer: form_data.get("printer").cloned(),
        copies: form_data.get("copies")
            .and_then(|s| s.parse().ok()),
        pages: form_data.get("pages").cloned(),
        duplex: Some(form_data.get("duplex")
            .map(|s| s == "true" || s == "on")
            .unwrap_or(false)),
        color: Some(form_data.get("color")
            .map(|s| s == "true" || s == "on")
            .unwrap_or(true)),
        page_size: form_data.get("page_size")
            .cloned().map(|s| { PrintPageSize::from(s)} )
    };



    let available_printers = app_state.get_printers().await;

    if available_printers.is_empty() {
        return json_error("No printers available".to_string());
    }

    // Determine printer to use with validation
    let printer_name = if let Some(requested_printer) = print_request.printer.clone() {
        if requested_printer.is_empty() {
            available_printers.iter()
                .find(|p| p.is_default)
                .map(|p| p.name.clone())
                .or_else(|| {
                    available_printers.first().map(|p| p.name.clone())
                })
                .unwrap_or_else(|| {
                    log::warn!("No default printer found, using 'default'");
                    "default".to_string()
                })
        } else {
            if available_printers.iter().any(|p| p.name == requested_printer) {
                requested_printer
            } else {
                let printer_names: Vec<String> = available_printers.iter().map(|p| p.name.clone()).collect();
                log::warn!("Requested printer '{}' not found. Available printers: {:?}",
                    requested_printer,
                    printer_names
                );
                return json_error(format!(
                    "Printer '{}' not found. Available printers: {}",
                    requested_printer,
                    printer_names.join(", ")
                ));
            }
        }
    } else {
        available_printers.iter()
            .find(|p| p.is_default)
            .map(|p| p.name.clone())
            .or_else(|| {
                available_printers.first().map(|p| p.name.clone())
            })
            .unwrap_or_else(|| {
                log::warn!("No printers found, using 'default'");
                "default".to_string()
            })
    };
    log::info!("Using printer: {}", printer_name);
    let (vendor, model) = app_state.get_printers().await
        .iter()
        .find(|&x| {x.name == printer_name})
        .map(|x| (x.vendor.clone(), x.model.clone()))
        .ok_or_else(|| {
            log::error!("Printer '{}' not found in all printers list", printer_name);
            ErrorBadRequest("Printer not found".to_string())
        })?;

    let print_job = PrintJob::new(filename.clone(), printer_name, vendor, model, print_request);
    let job_id = print_job.id;
    let printer = print_job.printer.clone();

    let file_path = format!("uploads/{}", filename);
    if let Err(e) = std::fs::write(&file_path, &file_data) {
        log::error!("Failed to save uploaded file: {}", e);
        return internal_error("Failed to save uploaded file".to_string());
    };

    print_job.save_to_db(&pool).await.map_err(|e| {
        log::error!("Failed to save print job: {}", e);
        ErrorInternalServerError(e.to_string())
    })?;

    add_to_job_queue(&job_queue, Job::Print(print_job))
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&file_path);
            ErrorInternalServerError(e.to_string())
        })?;

    tokio::spawn(async move {
        if let Err(e) = notify_scan_queue(&job_queue, &pool).await {
            let _ = std::fs::remove_file(&file_path);
            log::error!("Failed to notify scan queue: {}", e);
        };
    });

    json_success(serde_json::json!({
        "job_id": job_id,
        "status": "queued",
        "printer": printer
    }))
}

/// GET /api/print/jobs - List all print jobs
pub async fn list_print_jobs(pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let pool = pool.as_ref();

    let jobs = PrintJob::get_all(pool).map_err(|e| {
        log::error!("Failed to get print jobs: {}", e);
        ErrorInternalServerError(e.to_string())
    }).await?;

    json_success(jobs)
}

/// GET /api/print/jobs/{job_id} - Get specific print job
pub async fn get_print_job(path: web::Path<Uuid>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let uuid = path.into_inner();
    let pool = pool.as_ref();

    match PrintJob::find_by_uuid(uuid, pool).await {
        Ok(job) => json_success(job.clone()),
        Err(e) => internal_error(format!("Print job not found. {e}")),
    }
}

/// POST /api/print/jobs/{job_id} - Cancel print job
pub async fn cancel_print_job(path: web::Path<Uuid>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    let pool = pool.as_ref();

    let cups_service = CupsService::new();

    if let Some(mut job) = PrintJob::find_by_uuid(job_id, pool).await.map_err(|e| {ErrorInternalServerError(e.to_string())})? {
        if let Some(cups_job_id) = job.cups_job_id {
            match cups_service.cancel_job(&job.printer, cups_job_id).await {
                Ok(_) => {
                    job.set_status(PrintJobStatus::Cancelled);
                    job.update_in_db(pool).await.map_err(|e| {ErrorInternalServerError(e.to_string())})?;
                    json_success(serde_json::json!({"message": "Print job cancelled"}))
                },
                Err(e) => {
                    log::warn!("Failed to cancel CUPS job {}: {}", cups_job_id, e);
                    job.set_status(PrintJobStatus::Cancelled);
                    job.update_in_db(pool).await.map_err(|e| {ErrorInternalServerError(e.to_string())})?;
                    json_success(serde_json::json!({"message": "Print job marked as cancelled"}))
                }
            }
        } else {
            job.set_status(PrintJobStatus::Cancelled);
            job.update_in_db(pool).await.map_err(|e| {ErrorInternalServerError(e.to_string())})?;
            json_success(serde_json::json!({"message": "Print job cancelled"}))
        }
    } else {
        json_error("Print job not found".to_string())
    }
}

/// DELETE /api/print/jobs/{job_id} - Delete specific print job form database
pub async fn delete_print_job_record(path: web::Path<Uuid>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    match PrintJob::remove_by_uuid(job_id, pool.as_ref()).await {
        Ok(_) => { 
            log::info!("Removed Print Job record for {}", job_id);
            json_success(format!("Successfully removed job {}", job_id))
        },
        Err(e) => { 
            log::error!("Failed to remove Print Job record for {}: {}", job_id, e);
            internal_error(format!("Failed to find job: {}", e)) 
        },
    }
}

