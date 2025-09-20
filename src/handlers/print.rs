use actix_web::{web, HttpResponse, Result};
use actix_multipart::Multipart;
use futures_util::TryStreamExt;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use std::io::Write;
use log::log;
use crate::handlers::{json_success, json_error, internal_error};
use crate::models::{PrintJob, PrintRequest, PrintJobStatus};
use crate::services::cups::CupsService;

// In a real application, you'd want to use a proper database
// For this example, we'll use in-memory storage
// TODO use a database to store job IDs
type JobStorage = Arc<Mutex<HashMap<Uuid, PrintJob>>>;

lazy_static::lazy_static! {
    static ref PRINT_JOBS: JobStorage = Arc::new(Mutex::new(HashMap::new()));
}

/// GET /api/printers - List all available printers
pub async fn list_printers() -> Result<HttpResponse> {
    let cups_service = CupsService::new();

    if !cups_service.is_available().await {
        return json_error("CUPS service is not available".to_string());
    }

    match cups_service.get_printers().await {
        Ok(printers) => json_success(printers),
        Err(e) => internal_error(format!("Failed to get printers: {}", e)),
    }
}

/// POST /api/print - Submit a print job
pub async fn submit_print_job(mut payload: Multipart) -> Result<HttpResponse> {
    let cups_service = CupsService::new();

    if !cups_service.is_available().await {
        return json_error("CUPS service is not available".to_string());
    }

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut form_data: HashMap<String, String> = HashMap::new();

    // Process multipart form data
    while let Some(mut field) = payload.try_next().await.map_err(|e| {
        log::error!("Error reading multipart field: {}", e);
    }).unwrap_or(None) {
        let content_disposition = field.content_disposition().unwrap();

        if let Some(field_name) = content_disposition.get_name() {
            if field_name == "file" {
                // Handle file upload
                if let Some(file_name) = content_disposition.get_filename() {
                    filename = Some(file_name.to_string());
                }

                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| {
                    log::error!("Error reading file chunk: {}", e);
                }).unwrap_or(None) {
                    bytes.extend_from_slice(&chunk);
                }
                file_data = Some(bytes);
            } else {
                // Handle form fields
                let mut field_data = Vec::new();
                while let Some(chunk) = field.try_next().await.map_err(|e| {
                    log::error!("Error reading form field: {}", e);
                }).unwrap_or(None) {
                    field_data.extend_from_slice(&chunk);
                }

                if let Ok(value) = String::from_utf8(field_data) {
                    // form_data.insert(field_name.to_string(), value); // Fixme
                    form_data.insert("Dont know what is it".to_string(), value);
                }
            }
        }
    }

    // Validate required data
    let file_data = file_data.ok_or_else(|| {
        log::warn!("No file provided in print request");
        "No file provided"
    }).map_err(|e| json_error(e.to_string())).expect("No file provided"); //Fixme

    let filename = filename.ok_or_else(|| {
        log::warn!("No filename provided in print request");
        "No filename provided"
    }).map_err(|e| json_error(e.to_string())).expect("No filename provided"); //Fixme

    // Parse form data into PrintRequest
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
    };

    // Determine printer to use
    let printer_name = if let Some(printer) = print_request.printer.clone() {
        if printer.is_empty() {
            // Get default printer
            match cups_service.get_printers().await {
                Ok(printers) => {
                    printers.into_iter()
                        .find(|p| p.is_default)
                        .map(|p| p.name)
                        .unwrap_or_else(|| "default".to_string())
                },
                Err(_) => "default".to_string(),
            }
        } else {
            printer
        }
    } else {
        "default".to_string()
    };

    // Create print job
    let mut print_job = PrintJob::new(filename.clone(), printer_name, print_request);

    // Save file to uploads directory
    let file_path = format!("uploads/{}", filename);
    if let Err(e) = std::fs::write(&file_path, &file_data) {
        log::error!("Failed to save uploaded file: {}", e);
        return internal_error("Failed to save uploaded file".to_string());
    }

    // Submit to CUPS
    match cups_service.submit_print_job(&print_job, &file_path).await {
        Ok(cups_job_id) => {
            print_job.cups_job_id = Some(cups_job_id);
            print_job.set_status(PrintJobStatus::Processing);

            // Store job
            let job_id = print_job.id;
            PRINT_JOBS.lock().unwrap().insert(job_id, print_job.clone());

            // Start background job monitoring
            tokio::spawn(monitor_print_job(job_id, cups_job_id));

            json_success(serde_json::json!({
                "job_id": job_id,
                "cups_job_id": cups_job_id,
                "status": "processing"
            }))
        },
        Err(e) => {
            print_job.set_error(e.clone());
            let job_id = print_job.id;
            PRINT_JOBS.lock().unwrap().insert(job_id, print_job);

            // Clean up file
            let _ = std::fs::remove_file(&file_path);

            json_error(format!("Failed to submit print job: {}", e))
        }
    }
}

/// GET /api/print/jobs - List all print jobs
pub async fn list_print_jobs() -> Result<HttpResponse> {
    let jobs: Vec<PrintJob> = PRINT_JOBS.lock().unwrap()
        .values()
        .cloned()
        .collect();

    json_success(jobs)
}

/// GET /api/print/jobs/{job_id} - Get specific print job
pub async fn get_print_job(path: web::Path<Uuid>) -> Result<HttpResponse> {
    let job_id = path.into_inner();

    match PRINT_JOBS.lock().unwrap().get(&job_id) {
        Some(job) => json_success(job.clone()),
        None => json_error("Print job not found".to_string()),
    }
}

/// DELETE /api/print/jobs/{job_id} - Cancel print job
pub async fn cancel_print_job(path: web::Path<Uuid>) -> Result<HttpResponse> {
    let job_id = path.into_inner();
    let cups_service = CupsService::new();

    let mut jobs = PRINT_JOBS.lock().unwrap();

    if let Some(job) = jobs.get_mut(&job_id) {
        // Try to cancel in CUPS if we have a job ID
        if let Some(cups_job_id) = job.cups_job_id {
            match cups_service.cancel_job(&job.printer, cups_job_id).await {
                Ok(_) => {
                    job.set_status(PrintJobStatus::Cancelled);
                    json_success(serde_json::json!({"message": "Print job cancelled"}))
                },
                Err(e) => {
                    log::warn!("Failed to cancel CUPS job {}: {}", cups_job_id, e);
                    // Still mark as cancelled in system
                    job.set_status(PrintJobStatus::Cancelled);
                    json_success(serde_json::json!({"message": "Print job marked as cancelled"}))
                }
            }
        } else {
            // Job hasn't been submitted to CUPS yet, just mark as cancelled
            job.set_status(PrintJobStatus::Cancelled);
            json_success(serde_json::json!({"message": "Print job cancelled"}))
        }
    } else {
        json_error("Print job not found".to_string())
    }
}

/// Background task to monitor print job status
async fn monitor_print_job(job_id: Uuid, cups_job_id: i32) {
    let cups_service = CupsService::new();
    let mut last_status = String::new();

    // Monitor for up to 5 minutes
    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        match cups_service.get_job_status(cups_job_id).await {
            Ok(status) => {
                if status != last_status {
                    last_status = status.clone();

                    let mut jobs = PRINT_JOBS.lock().unwrap();
                    if let Some(job) = jobs.get_mut(&job_id) {
                        let new_status = match status.as_str() {
                            "queued" | "pending" => PrintJobStatus::Queued,
                            "printing" => PrintJobStatus::Printing,
                            "completed" => PrintJobStatus::Completed,
                            "stopped" | "aborted" => PrintJobStatus::Failed,
                            "cancelled" => PrintJobStatus::Cancelled,
                            _ => PrintJobStatus::Processing,
                        };

                        job.set_status(new_status.clone());

                        // If job is finished, stop monitoring
                        match new_status {
                            PrintJobStatus::Completed |
                            PrintJobStatus::Failed |
                            PrintJobStatus::Cancelled => {
                                // Clean up uploaded file after a delay
                                let filename = job.filename.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                                    let _ = std::fs::remove_file(format!("uploads/{}", filename));
                                });
                                break;
                            },
                            _ => {}
                        }
                    }
                }
            },
            Err(e) => {
                log::error!("Failed to get job status for {}: {}", cups_job_id, e);
                break;
            }
        }
    }
}
