use actix_web::{web, HttpResponse, Result};
use actix_files::NamedFile;
use std::path::PathBuf;
use sqlx::SqlitePool;
use crate::handlers::{json_success, internal_error, json_error};
use crate::models::{ScanJob, SystemStatus};
use crate::services::cups::CupsService;
use crate::services::sane::SaneService;
use crate::services::escputil::MaintenanceService;
use tokio::process::Command;


/// GET / - Serve main dashboard page
pub async fn index() -> Result<NamedFile> {
    Ok(NamedFile::open_async("templates/index.html").await?)
}

/// GET /print - Serve print management page
pub async fn print_page() -> Result<NamedFile> {
    Ok(NamedFile::open_async("templates/print.html").await?)
}

/// GET /scan - Serve scan management page
pub async fn scan_page() -> Result<NamedFile> {
    Ok(NamedFile::open_async("templates/scan.html").await?)
}

/// GET /api/system/status - Get system status
pub async fn get_status() -> Result<HttpResponse> {
    let cups_service = CupsService::new();
    let sane_service = SaneService::new();

    let cups_available = cups_service.is_available().await;

    let sane_available = sane_service.is_available().await;

    let active_print_jobs = 0;
    let active_scan_jobs = 0;

    let disk_space_mb = get_disk_space().await;

    let status = SystemStatus {
        cups_available,
        sane_available,
        active_print_jobs,
        active_scan_jobs,
        disk_space_mb,
    };

    json_success(status)
}

/// GET /api/system/settings - Get system settings (placeholder)
pub async fn get_settings() -> Result<HttpResponse> {
    // TODO create table with default settings and store in database
    let settings = serde_json::json!({
        "default_resolution": 300,
        "auto_cleanup": true,
        "max_file_size_mb": 50,
        "supported_formats": ["pdf", "jpeg", "png", "tiff"]
    });

    json_success(settings)
}

/// POST /api/system/settings - Update system settings (placeholder)
pub async fn update_settings() -> Result<HttpResponse> {
    // Implement settings update logic here
    json_success(serde_json::json!({"message": "Settings updated successfully"}))
}

/// POST /api/system/nozzle/check
pub async fn nozzle_check() -> Result<HttpResponse> {
    let service = MaintenanceService::new();
    
    if !service.is_available().await {
        return json_error("false".to_string())
    }
    
    match service.do_nozzle_heads_check().await {
        Ok(_) => json_success("true"),
        Err(e) => json_error(e),
    }
}

/// POST /api/system/nozzle/clean
pub async fn nozzle_clean() -> Result<HttpResponse> {
    let service = MaintenanceService::new();

    if !service.is_available().await {
        return json_error("false".to_string())
    }
    
    match service.do_nozzle_heads_cleaning().await {
        Ok(_) => json_success("true"),
        Err(e) => json_error(e),
    }
}

/// GET /api/files/uploads - List uploaded files
pub async fn list_uploads() -> Result<HttpResponse> {
    match std::fs::read_dir("uploads") {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.file_name().to_string_lossy().eq(".gitkeep"){
                        continue;
                    }
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            let file_info = serde_json::json!({
                                "name": entry.file_name().to_string_lossy(),
                                "size": metadata.len(),
                                "modified": metadata.modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                            });
                            files.push(file_info);
                        }
                    }
                }
            }
            json_success(files)
        },
        Err(e) => internal_error(format!("Failed to list uploads: {}", e)),
    }
}

/// DELETE /api/files/uploads/{filename} - Delete uploaded file
pub async fn delete_upload(path: web::Path<String>) -> Result<HttpResponse> {
    let filename = path.into_inner();

    match std::fs::remove_file(format!("uploads/{}", filename)) {
        Ok(_) => json_success(format!("File {} deleted successfully", filename)),
        Err(e) => json_error(format!("Failed to delete upload: {}", e)),
    }

    // TODO
    //  remove record from database
}

/// GET /api/files/scans - List scan files
pub async fn list_scans() -> Result<HttpResponse> {
    match std::fs::read_dir("scans") {
        Ok(entries) => {
            let mut files = Vec::new();
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if entry.file_name().to_string_lossy().eq(".gitkeep") {
                            continue;
                        }

                        if metadata.is_file() {
                            let file_info = serde_json::json!({
                                "name": entry.file_name().to_string_lossy(),
                                "size": metadata.len(),
                                "modified": metadata.modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                            });
                            files.push(file_info);
                        }
                    }
                }
            }
            json_success(files)
        },
        Err(e) => internal_error(format!("Failed to list scans: {}", e)),
    }
}

/// DELETE /api/files/scans/{filename} - Delete scan file
pub async fn delete_scan(path: web::Path<String>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let filename = path.into_inner();
    let pool = pool.as_ref();

    // ScanJob::remove_from_db().await;
    
    match std::fs::remove_file(format!("scans/{}", filename)) {
        Ok(_) => json_success(format!("File {} deleted successfully", filename)),
        Err(e) => json_error(format!("Failed to delete upload: {}", e)),
    }
    
}



/// Helper function to get available disk space
async fn get_disk_space() -> Option<u64> {
    match Command::new("df").args(["-m", "."]).output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(available) = parts[3].parse::<u64>() {
                        return Some(available);
                    }
                }
            }
            None
        },
        _ => None,
    }
}
