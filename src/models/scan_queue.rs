use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{ScanJob, ScanJobStatus};
use crate::services::sane::SaneService;


pub struct ScanJobQueue {
    queue: Arc<Mutex<VecDeque<ScanJob>>>,
    processing: Arc<Mutex<bool>>,
}

impl ScanJobQueue {
    pub fn new() -> Self {
        ScanJobQueue {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            processing: Arc::new(Mutex::new(false)),
        }
    }

    async fn push(&self, job: ScanJob) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut queue = self.queue.lock().await;
        queue.push_back(job);
        Ok(())
    }

    async fn pop(&self) -> Result<Option<ScanJob>, Box<dyn std::error::Error + Send + Sync>> {
        let mut queue = self.queue.lock().await;
        Ok(queue.pop_front())
    }

    async fn is_empty(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.is_empty()
    }

    async fn len(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    async fn is_processing(&self) -> bool {
        *self.processing.lock().await
    }

    async fn set_processing(&self, value: bool) {
        *self.processing.lock().await = value;
    }
}

pub async fn add_to_scan_queue(s_queue: &ScanJobQueue, scan_job: ScanJob) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    s_queue.push(scan_job).await
}

pub async fn notify_scan_queue(s_queue: &ScanJobQueue, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    if s_queue.is_empty().await {
        return Ok(());
    }

    if s_queue.is_processing().await {
        return Ok(());
    }

    let queue_len = s_queue.len().await;
    log::info!("Requests in queue: {}", queue_len);

    if let Err(e) = handle_scan_job(s_queue, pool).await {
        log::error!("Failed to handle next job in queue: {}", e);
    }

    Ok(())
}


async fn handle_scan_job(s_queue: &ScanJobQueue, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(scan_job) = s_queue.pop().await? {
        s_queue.set_processing(true).await;
        execute_scan_job(scan_job.id, pool).await?;
        s_queue.set_processing(false).await;
    }

    if let Err(e) = Box::pin(notify_scan_queue(s_queue, pool)).await {
        log::error!("Failed to notify scan queue: {}", e);
    }

    Ok(())
}


/// Background task to execute scan job
async fn execute_scan_job(job_id: Uuid, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sane_service = SaneService::new();

    // Get job from storage
    let mut job = match ScanJob::find_by_uuid(job_id, pool).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            log::warn!("Scan job {} not found in storage", job_id);
            return Err("Job not found".into());
        }
        Err(e) => {
            return Err(e.into());
        }
    };

    // Update status to scanning
    job.set_status(ScanJobStatus::Scanning);
    job.update_statues_in_db(pool).await?;

    // Execute the scan
    match sane_service.start_scan(&job).await {
        Ok(output_path) => {
            // Update job with file metadata if available
            if let Ok(metadata) = std::fs::metadata(&output_path) {
                job.file_size = Some(metadata.len());
                job.file_available = true;
            }
            job.set_status(ScanJobStatus::Completed);
            job.update_statues_in_db(pool).await?;

            log::info!("Scan job {} completed successfully", job_id);
        }
        Err(e) => {
            // Store error in job record
            job.set_error(e.clone());
            job.update_statues_in_db(pool).await?;

            log::error!("Scan job {} failed: {}", job_id, e);
        }
    }

    Ok(())
}