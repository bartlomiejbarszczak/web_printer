pub mod print_job;
pub mod scan_job;
pub mod job_queue;

use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use chrono::{DateTime, Utc};
pub use print_job::*;
pub use scan_job::*;
pub use job_queue::*;

use serde::{Deserialize, Serialize};
use std::sync::{Arc};
use sqlx::SqlitePool;
use tokio::sync::{RwLock};
use tokio::time::Instant;
use crate::services::cups::CupsService;
use crate::services::sane::SaneService;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Printer {
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub description: String,
    pub status: String,
    pub location: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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


#[derive(Debug, Clone)]
pub struct AppState {
    pub start_time: Instant,
    scanners: Arc<RwLock<Vec<Scanner>>>,
    printers: Arc<RwLock<Vec<Printer>>>,
}

impl Display for Scanner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Scanner - Name: {}, Vendor: {}, Model: {}", self.name, self.vendor, self.model)
    }
}

impl Display for Printer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"Printer - Name: {}, Vendor: {}, Model: {}", self.name, self.vendor, self.model)
    }
}

impl AppState {
    pub async fn new() -> Self {
        let scanners  = SaneService::new().get_scanners().await.unwrap_or_else(|e| {
            log::warn!("No scanners collected: {}", e);
            Vec::new()
        });
        let printers = CupsService::new().get_printers().await.unwrap_or_else(|e| {
            log::warn!("No printers collected: {}", e);
            Vec::new()
        });
        
        Self {
            start_time: Instant::now(),
            scanners: Arc::new(RwLock::new(scanners)),
            printers:Arc::new(RwLock::new(printers)),
        }
    }

    pub async fn add_scanner(&mut self, scanner: Scanner) {
        self.scanners.write().await.push(scanner);
    }

    pub async fn add_printer(&mut self, printer: Printer) {
        self.printers.write().await.push(printer);
    }

    pub async fn get_scanners(&self) -> Vec<Scanner> { 
        self.scanners.read().await.clone()
    }

    pub async fn get_printers(&self) -> Vec<Printer> {
        self.printers.read().await.clone()
    }

    pub async fn show_devices(&self) -> String {
        let mut devices = String::from("Scanners:\n\t");
        let scanners = self.scanners.read().await.iter().
            map(|scanner| {scanner.to_string()})
            .collect::<Vec<String>>().join("\n\t");
        devices.push_str(&scanners);

        devices.push_str("\nPrinters:\n\t");
        let printers = self.printers.read().await.iter().
            map(|printer| {printer.to_string()})
            .collect::<Vec<String>>().join("\n\t");
        devices.push_str(&printers);

        devices
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Job {
    Scan(ScanJob),
    Print(PrintJob),
}

impl Display for Job {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Job::Scan(sj) => {
                format!("ScanJob - {}", sj.id)
            }
            Job::Print(pj) => {
                format!("PrintJob - {}", pj.id)
            }
        };
        
        write!(f, "{}", str)
    }
}

impl Job {
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Job::Scan(sj) => sj.completed_at,
            Job::Print(pr) => pr.completed_at,
        }
    }

    pub async fn execute(&mut self, pool: &SqlitePool) {
        match self {
            Job::Scan(sj) => {
                if let Err(e) = execute_scan_job(sj.id, pool).await {
                    log::error!("Failed to execute scan job: {}", e);
                };
            }
            Job::Print(pj) => {
                if let Err(e) = execute_print_job(pj, pool).await {
                    log::error!("Failed to execute print job: {}", e);
                };
            }
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
