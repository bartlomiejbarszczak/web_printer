use actix_web::{web, HttpResponse, Result};
use actix_files::NamedFile;
use actix_web::error::ErrorInternalServerError;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tokio::process::Command;
use tokio::time::{Instant, Duration};
use crate::handlers::{json_success, internal_error, json_error};
use crate::models::{AppState, PrintJob, ScanJob, SystemStatus, Job, JobQueue};
use crate::services::cups::CupsService;
use crate::services::sane::SaneService;
use crate::services::escputil::MaintenanceService;



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
pub async fn get_status(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let cups_service = CupsService::new();
    let sane_service = SaneService::new();

    let cups_available = cups_service.is_available().await;

    let sane_available = sane_service.is_available().await;

    let active_print_jobs = cups_service.get_active_jobs()
        .await
        .map_err(|e| { ErrorInternalServerError(e)})?
        .len();

    let active_scan_jobs = 0;

    let disk_space_mb = get_disk_space().await;
    let uptime_str = get_uptime(app_state.start_time).await;

    let status = SystemStatus {
        cups_available,
        sane_available,
        active_print_jobs,
        active_scan_jobs,
        disk_space_mb,
        uptime_str
    };

    json_success(status)
}

/// GET /api/system/settings - Get system settings
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

/// POST /api/system/settings - Update system settings
pub async fn update_settings() -> Result<HttpResponse> {
    // TODO Implement settings update logic here

    internal_error("Not implemented".to_string())
}


/// GET /api/system/recent - Get recent 5 jobs
pub async fn get_recent_activity(pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let limit = 4;

    let (scan_jobs_r, print_jobs_r) = tokio::try_join!(
        ScanJob::get_recent(limit, &pool),
        PrintJob::get_recent(limit, &pool)
    ).map_err(|e| {
        log::error!("Error getting recent activity: {}", e);
        ErrorInternalServerError(e)
    })?;

    let mut recent_jobs = scan_jobs_r.iter()
        .map(|sj| Job::Scan(sj.clone()))
        .chain(
            print_jobs_r.iter()
                .map(|pj| Job::Print(pj.clone())))
        .filter(|x| was_within_last_hour(x.completed_at()))
        .collect::<Vec<Job>>();

    recent_jobs.sort();
    recent_jobs.reverse();

    let recent_jobs = recent_jobs.into_iter().take(limit as usize).collect::<Vec<Job>>();

    log::info!("Successfully updated recent activity");
    json_success(recent_jobs)
}

/// POST /api/system/nozzle/check
pub async fn nozzle_check() -> Result<HttpResponse> {
    let service = MaintenanceService::new();
    
    if !service.is_available().await {
        return json_error("false".to_string())
    }
    
    log::info!("Perform nozzle check");
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
    
    log::info!("Perform nozzle clean");
    match service.do_nozzle_heads_cleaning().await {
        Ok(_) => json_success("true"),
        Err(e) => json_error(e),
    }
}


/// GET /api/system/queue
pub async fn get_current_queue(job_queue: web::Data<JobQueue>, pool: web::Data<SqlitePool>) -> Result<HttpResponse> {
    let jq = job_queue.get_ref();
    let data = jq.get_current_queue(&pool).await;
    
    json_success(data)
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

/// Helper function to get current uptime
async fn get_uptime(start_time: Instant) -> String {
    let uptime = start_time.elapsed();

    format_duration(uptime)
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();

    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(format!("{}d", days));
    }
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{}s", seconds));
    }

    parts.join(" ")
}

fn was_within_last_hour(completed_at: Option<DateTime<Utc>>) -> bool {
    let completed_at = match completed_at {
        Some(com) => com,
        None => return false,
    };

    let now = Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);

    completed_at >= one_hour_ago && completed_at <= now
}