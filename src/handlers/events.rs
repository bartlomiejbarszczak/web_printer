use std::cmp::Ordering;
use actix_web::{web, Responder};
use actix_web_lab::sse::{self, Sse, Data as SseData};
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use sqlx::SqlitePool;
use serde_json::json;

use crate::models::{PrintJob, ScanJob, ScanJobStatus, PrintJobStatus, JobQueue};
use crate::utils::get_disk_space;

#[derive(Clone)]
pub struct EventState {
    pub queue_version: Arc<RwLock<u64>>,
    pub status_version: Arc<RwLock<u64>>,
}

impl EventState {
    pub fn new() -> Self {
        Self {
            queue_version: Arc::new(RwLock::new(0)),
            status_version: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn increment_queue_version(&self) {
        let mut version = self.queue_version.write().await;
        *version += 1;

        match version.cmp(&u64::MAX) {
            Ordering::Equal => {
                *version = 0;
            }
            _ => {}
        }

    }

    pub async fn increment_status_version(&self) {
        let mut version = self.status_version.write().await;
        *version += 1;

        match version.cmp(&u64::MAX) {
            Ordering::Equal => {
                *version = 0;
            }
            _ => {}
        }
    }
}

/// SSE endpoint that streams updates to clients
pub async fn event_stream(
    job_queue: web::Data<JobQueue>,
    pool: web::Data<SqlitePool>,
    event_state: web::Data<EventState>,
) -> impl Responder {
    let pool = pool.clone();
    let event_state = event_state.clone();

    let stream = create_event_stream(job_queue, pool, event_state);

    Sse::from_stream(stream)
        .with_keep_alive(Duration::from_secs(15))
}

fn create_event_stream(
    job_queue: web::Data<JobQueue>,
    pool: web::Data<SqlitePool>,
    event_state: web::Data<EventState>,
) -> Pin<Box<dyn Stream<Item = Result<sse::Event, std::io::Error>> + Send>> {
    let mut interval_stream = IntervalStream::new(interval(Duration::from_millis(500)));

    let mut last_queue_version = 0u64;
    let mut last_status_version = 0u64;
    let mut last_recent_version = 0u64;

    let stream = async_stream::stream! {
        let data = job_queue.get_current_queue(&pool).await;
        if let Ok(sse_data) = SseData::new_json(&json!({
            "type": "queue_update",
            "queue": data
        })) {
            yield Ok(sse::Event::Data(sse_data));
        }

        while let Some(_) = interval_stream.next().await {
            let current_queue_version = *event_state.queue_version.read().await;
            if current_queue_version != last_queue_version {
                last_queue_version = current_queue_version;

                let data = job_queue.get_current_queue(&pool).await;
                if let Ok(sse_data) = SseData::new_json(&json!({
                    "type": "queue_update",
                    "queue": data
                })) {
                    yield Ok(sse::Event::Data(sse_data));
                }
            }

            let current_status_version = *event_state.status_version.read().await;
            if current_status_version != last_status_version {
                last_status_version = current_status_version;

                let status_result = get_system_status(&pool).await;
                match status_result {
                    Ok(status) => {
                        if let Ok(sse_data) = SseData::new_json(&json!({
                            "type": "status_update",
                            "status": status
                        })) {
                            yield Ok(sse::Event::Data(sse_data));
                        }
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        log::error!("Failed to get status: {}", err_msg);
                    }
                }
            }
        }
    };

    Box::pin(stream)
}

async fn get_system_status(pool: &SqlitePool) -> Result<serde_json::Value, sqlx::Error> {
    let active_prints = PrintJob::find_by_statuses(
        vec![PrintJobStatus::Printing, PrintJobStatus::Processing],
        pool
    ).await?.len();

    let active_scans = ScanJob::find_by_statuses(
        vec![ScanJobStatus::Scanning, ScanJobStatus::Processing],
        pool
    ).await?.len();

    let disk_space_mb = get_disk_space().await;

    Ok(json!({
        "active_prints": active_prints,
        "active_scans": active_scans,
        "disk_space_mb": disk_space_mb,
    }))
}


