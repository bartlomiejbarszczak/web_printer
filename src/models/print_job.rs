use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrintJob {
    pub id: Uuid,
    pub filename: String,
    pub printer: String,
    pub status: PrintJobStatus,
    pub copies: u32,
    pub pages: Option<String>,
    pub duplex: bool,
    pub color: bool,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub cups_job_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PrintJobStatus {
    Queued,
    Processing,
    Printing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Deserialize)]
pub struct PrintRequest {
    pub printer: Option<String>,
    pub copies: Option<u32>,
    pub pages: Option<String>,
    pub duplex: Option<bool>,
    pub color: Option<bool>,
}

impl PrintJob {
    pub fn new(filename: String, printer: String, request: PrintRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            filename,
            printer,
            status: PrintJobStatus::Queued,
            copies: request.copies.unwrap_or(1),
            pages: request.pages,
            duplex: request.duplex.unwrap_or(false),
            color: request.color.unwrap_or(true),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            cups_job_id: None,
        }
    }

    pub fn set_status(&mut self, status: PrintJobStatus) {
        self.status = status;
        match &self.status {
            PrintJobStatus::Processing => {
                if self.started_at.is_none() {
                    self.started_at = Some(Utc::now());
                }
            },
            PrintJobStatus::Completed | PrintJobStatus::Failed | PrintJobStatus::Cancelled => {
                if self.completed_at.is_none() {
                    self.completed_at = Some(Utc::now());
                }
            },
            _ => {}
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.set_status(PrintJobStatus::Failed);
    }
}