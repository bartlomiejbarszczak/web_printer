pub mod print_job;
pub mod scan_job;
pub mod scan_queue;

use std::cmp::Ordering;
use chrono::{DateTime, Utc};
pub use print_job::*;
pub use scan_job::*;
pub use scan_queue::*;

use serde::{Deserialize, Serialize};
use tokio::time::Instant;

#[macro_export]
macro_rules! query_bind {
    ($query:expr $(, $param:expr)* $(,)?) => {{
        let mut query = sqlx::query($query);
        $(
            query = query.bind($param);
        )*
        query
    }};
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Printer {
    pub name: String,
    pub description: String,
    pub status: String,
    pub location: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scanner {
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub device_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cups_available: bool,
    pub sane_available: bool,
    pub active_print_jobs: usize,
    pub active_scan_jobs: usize,
    pub disk_space_mb: Option<u64>,
    pub uptime_str: String
}


pub struct AppState {
    pub start_time: Instant,
}

#[derive(Serialize, Deserialize)]
pub enum Job {
    Scan(ScanJob),
    Print(PrintJob),
}

impl Job {
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Job::Scan(sj) => sj.completed_at,
            Job::Print(pr) => pr.completed_at,
        }
    }
}

impl Eq for Job {}

impl PartialEq<Self> for Job {
    fn eq(&self, other: &Self) -> bool {
        self.completed_at() == other.completed_at()
    }
}

impl PartialOrd<Self> for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        self.completed_at().cmp(&other.completed_at())
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message: Some(message),
            data: None,
        }
    }
}
