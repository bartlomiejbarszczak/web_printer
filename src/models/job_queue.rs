use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{Job, PrintJob, PrintJobStatus, ScanJob, ScanJobStatus};
use crate::services::cups::CupsService;
use crate::services::sane::SaneService;

#[derive(Clone)]
pub struct JobQueue {
    queue: Arc<Mutex<VecDeque<Job>>>,
    processing: Arc<Mutex<bool>>,
    processing_job_id: Arc<Mutex<Option<Uuid>>>
}

impl JobQueue {
    pub fn new() -> Self {
        JobQueue {
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(5))),
            processing: Arc::new(Mutex::new(false)),
            processing_job_id: Arc::new(Mutex::new(None))
        }
    }

    async fn push(&self, job: Job) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut queue = self.queue.lock().await;
        queue.push_back(job);
        Ok(())
    }

    async fn pop(&self) -> Result<Option<Job>, Box<dyn std::error::Error + Send + Sync>> {
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
        let status = *self.processing.lock().await;
        log::warn!("JobQueue is_processing: {:?}", status);
        status
    }

    async fn set_processing(&self, value: bool) {
        *self.processing.lock().await = value;
        log::warn!("JobQueue is set to: {}", value);
    }

    pub async fn get_current_queue(&self, pool: &SqlitePool) -> Vec<Job> {
        let mut q = self.queue.lock().await
            .iter()
            .map(|j| j.clone())
            .collect::<Vec<Job>>();

        if let Some(job_uuid) = self.processing_job_id.lock().await.clone() {
            match Job::get_job_by_id(job_uuid, pool).await {
                Err(e) => { log::warn!("Failed to get job: {}", e) },
                Ok(job_op) => {
                    if let Some(job) = job_op {
                        q.insert(0, job);
                    }
                }
            };
        };

        q
    }

    async fn set_processing_job_id(&self, value: Option<Uuid>) {
        *self.processing_job_id.lock().await = value;
    }
}

pub async fn add_to_job_queue(job_queue: &JobQueue, job: Job) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::warn!("Added new job to queue: {}", job);
    job_queue.push(job).await
}

pub async fn notify_scan_queue(job_queue: &JobQueue, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    if job_queue.is_empty().await {
        return Ok(());
    }

    if job_queue.is_processing().await {
        return Ok(());
    }

    let queue_len = job_queue.len().await;
    log::info!("Requests in queue: {}", queue_len);

    if let Err(e) = handle_job(job_queue, pool).await {
        log::error!("Failed to handle next job in queue: {}", e);
    }

    Ok(())
}

async fn handle_job(job_queue: &JobQueue, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(mut job) = job_queue.pop().await? {
        log::warn!("Processing job: {}", job);
        // FIXME wrap me
        job_queue.set_processing(true).await;
        job_queue.set_processing_job_id(Some(job.id())).await;

        job.execute(pool).await;

        job_queue.set_processing(false).await;
        job_queue.set_processing_job_id(None).await;
    }

    if let Err(e) = Box::pin(notify_scan_queue(job_queue, pool)).await {
        log::error!("Failed to notify scan queue: {}", e);
    }

    Ok(())
}

/// Background task to execute scan job
pub async fn execute_scan_job(job_id: Uuid, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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


pub async fn execute_print_job(print_job: &mut PrintJob, pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error + Send + Sync>>  {
    let cups_service = CupsService::new();
    let pool = pool.clone(); // fixme maybe its possible not cloning

    let file_path = print_job.get_file_path().unwrap_or_else(|| {
        log::warn!("Could not get file path from print job. Trying default one: uploads/{}", print_job.filename);
        format!("uploads/{}", print_job.filename)
    });

    match cups_service.submit_print_job(&print_job, &file_path).await {
        Ok(cups_job_id) => {
            print_job.set_cups_job_id(cups_job_id);
            print_job.set_status(PrintJobStatus::Processing);

            let job_id = print_job.id;

            print_job.update_in_db(&pool).await.map_err(|e| {
                log::error!("Failed to update print job: {}", e);
                e.to_string()
            })?;

            // Start background job monitoring
            tokio::spawn(async move {
                if let Err(e) = monitor_print_job(job_id, cups_job_id, &pool).await {
                    log::error!("Monitor print job {} failed: {}", job_id, e);
                };
            });

        },
        Err(e) => {
            print_job.set_error(e.clone());

            print_job.update_in_db(&pool).await.map_err(|e| {
                log::error!("Failed to update print job statuses: {}", e);
                e.to_string()
            })?;

            let _ = std::fs::remove_file(&file_path);
        }
    }

    Ok(())
}


/// Background task to monitor print job status
async fn monitor_print_job(job_id: Uuid, cups_job_id: i32, pool: &SqlitePool) -> actix_web::Result<(), sqlx::Error> {
    let cups_service = CupsService::new();
    let mut last_status = String::new();

    // Monitor for up to 5 minutes
    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        match cups_service.get_job_status(cups_job_id).await {
            Ok(status) => {
                if status != last_status {
                    last_status = status.clone();

                    if let Some(mut job) = PrintJob::find_by_uuid(job_id, pool).await? {
                        let new_status = match status.as_str() {
                            "queued" | "pending" => PrintJobStatus::Queued,
                            "printing" => PrintJobStatus::Printing,
                            "completed" => PrintJobStatus::Completed,
                            "stopped" | "aborted" => PrintJobStatus::Failed,
                            "cancelled" => PrintJobStatus::Cancelled,
                            "idle" => PrintJobStatus::Completed,
                            _ => PrintJobStatus::Processing,
                        };

                        job.set_status(new_status.clone());
                        job.update_in_db(pool).await?;

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

    Ok(())
}