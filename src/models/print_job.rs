use std::ffi::OsStr;
use std::fmt::Display;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;
use crate::query_bind;
use std::path::Path;

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


impl Display for PrintJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            PrintJobStatus::Queued => String::from("queued"),
            PrintJobStatus::Processing => String::from("processing"),
            PrintJobStatus::Printing => String::from("printing"),
            PrintJobStatus::Completed => String::from("completed"),
            PrintJobStatus::Failed => String::from("failed"),
            PrintJobStatus::Cancelled => String::from("cancelled"),
        };
        write!(f, "{}", str)
    }
}

impl TryFrom<&SqliteRow> for PrintJob {
    type Error = sqlx::Error;

    fn try_from(row: &SqliteRow) -> Result<Self, Self::Error> {
        let status = match row.try_get("status")? {
            "queued" => PrintJobStatus::Queued,
            "processing" => PrintJobStatus::Processing,
            "printing" => PrintJobStatus::Printing,
            "completed" => PrintJobStatus::Completed,
            "failed" => PrintJobStatus::Failed,
            "cancelled" => PrintJobStatus::Cancelled,
            _ => {
                log::error!("Status parsing error for row");
                return Err(sqlx::Error::InvalidArgument("Unrecognized status".to_string()))
            }
        };

        let uuid = Uuid::parse_str(row.try_get("job_uuid")?)
            .map_err(|e| {sqlx::Error::InvalidArgument(e.to_string())})?;


        Ok(PrintJob {
            id: uuid,
            filename: row.try_get("filename")?,
            printer: row.try_get("printer_name")?,
            status,
            copies: row.try_get("copies")?,
            pages: row.try_get("pages_range")?,
            duplex: row.try_get("duplex")?,
            color: row.try_get("color")?,
            created_at: row.try_get("created_at")?,
            started_at: row.try_get("started_at")?,
            completed_at: row.try_get("completed_at")?,
            error_message: row.try_get("error_message")?,
            cups_job_id: row.try_get("cups_id_job")?,
        })
    }
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

    pub fn get_file_path(&self) -> Option<String> {
        Some(format!("uploads/{}", self.filename))
    }


    pub async fn save_to_db(&self, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let status_str = self.status.to_string();

        let format = Path::new(self.filename.as_str()).extension().and_then(OsStr::to_str);

        let query = query_bind!(
            r#"
            INSERT INTO print_jobs (
                job_uuid, cups_id_job, printer_name, filename, filepath, status,
                created_at, started_at, completed_at, error_message, copies,
                pages_range, duplex, color, original_filename, mime_type
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
            "#,
            self.id.to_string(),
            self.cups_job_id,
            self.printer.clone(),
            self.filename.clone(),
            self.get_file_path(),
            status_str,
            self.created_at,
            self.started_at,
            self.completed_at,
            self.error_message.clone(),
            self.copies,
            self.pages.clone(),
            self.duplex,
            self.color,
            self.filename.clone(),
            format
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }


    pub async fn update_statuses_in_db(&self, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let status_str = self.status.to_string();

        let query = query_bind!(
            r#"
            UPDATE print_jobs
            SET status = ?, started_at = ?, completed_at = ?, error_message = ? WHERE job_uuid = ?;
            "#,
            status_str,
            self.started_at,
            self.completed_at,
            self.error_message.clone(),
            self.id.to_string()
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }

    pub async fn remove_by_uuid(uuid: Uuid, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let query = query_bind!(
            r#"
            DELETE FROM print_jobs WHERE job_uuid = ?;
            "#,
            uuid.to_string()
        ).execute(pool).await?;
    
        Ok(query.rows_affected())
    }

    pub async fn find_by_uuid(uuid: Uuid, pool: &SqlitePool) -> Result<Option<PrintJob>, sqlx::Error> {
        let row_op = query_bind!(
            r#"
                SELECT * FROM print_jobs WHERE job_uuid = ?;
            "#,
            uuid.to_string()
        ).fetch_optional(pool).await?;

        if let Some(row) = row_op {
            log::info!("Successfully fetch row from print_jobs table");
            Ok(Some(PrintJob::try_from(&row)?))
        } else {
            log::warn!("No rows fetched from print_jobs table");
            Ok(None)
        }
    }

    pub async fn find_by_status(status: PrintJobStatus, pool: &SqlitePool) -> Result<Vec<PrintJob>, sqlx::Error> {
        let rows = query_bind!(
            r#"
            SELECT * FROM print_jobs WHERE status = ?
            ORDER BY created_at ASC;
            "#,
            status.to_string()
        ).fetch_all(pool).await?;
    
        let print_jobs = rows.iter()
            .map(|x| {PrintJob::try_from(x)}).collect::<Result<Vec<PrintJob>, sqlx::Error>>();
    
        Ok(print_jobs?)
    }

    pub async fn get_recent(limit: u32, pool: &SqlitePool) -> Result<Vec<PrintJob>, sqlx::Error> {
        let rows = query_bind!(
            r#"
            SELECT * FROM print_jobs
            ORDER BY created_at DESC
            LIMIT ?
            ;"#,
            limit
        ).fetch_all(pool).await?;
    
        let print_jobs = rows.iter()
            .map(|x| {PrintJob::try_from(x)}).collect::<Result<Vec<PrintJob>, sqlx::Error>>();
    
        Ok(print_jobs?)
    }
    
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<PrintJob>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM print_jobs
            ORDER BY created_at DESC;
            "#,
        ).fetch_all(pool).await?;

        let print_jobs = rows.iter()
            .map(|x| {PrintJob::try_from(x)}).collect::<Result<Vec<PrintJob>, sqlx::Error>>();

        Ok(print_jobs?)
    }

}