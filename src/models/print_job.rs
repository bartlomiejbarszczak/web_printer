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
    pub vendor: String,
    pub model: String,
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
    pub page_size: PrintPageSize,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PrintPageSize {
    A4,
    A5,
    A6,
    B5,
    B6,
    Postcard,
    Letter,
    Legal,
}

#[derive(Debug, Deserialize)]
pub struct PrintRequest {
    pub printer: Option<String>,
    pub copies: Option<u32>,
    pub pages: Option<String>,
    pub duplex: Option<bool>,
    pub color: Option<bool>,
    pub page_size: Option<PrintPageSize>
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


impl Display for PrintPageSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            PrintPageSize::A4 => "a4".to_string(),
            PrintPageSize::A5 => "a5".to_string(),
            PrintPageSize::A6 => "a6".to_string(),
            PrintPageSize::B5 => "b5".to_string(),
            PrintPageSize::B6 => "b6".to_string(),
            PrintPageSize::Postcard => "postcard".to_string(),
            PrintPageSize::Letter => "letter".to_string(),
            PrintPageSize::Legal => "legal".to_string(),
        };
        
        write!(f, "{}", str)
    }
}

impl PrintPageSize {
    pub fn from(s: String) -> PrintPageSize {
        match s.as_ref() {
            "a4" => PrintPageSize::A4,
            "a5" => PrintPageSize::A5,
            "a6" => PrintPageSize::A6,
            "b5" => PrintPageSize::B5,
            "b6" => PrintPageSize::B6,
            "postcard" => PrintPageSize::Postcard,
            "letter" => PrintPageSize::Letter,
            "legal" => PrintPageSize::Legal,
            _ => {
                log::warn!("Unsupported page size: {}.\tUsing the A4 page size", s);
                PrintPageSize::A4
            },
        }
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

        let page_size = match row.try_get("page_size")? {
            "a4" => PrintPageSize::A4,
            "a5" => PrintPageSize::A5,
            "a6" => PrintPageSize::A6,
            "b5" => PrintPageSize::B5,
            "b6" => PrintPageSize::B6,
            "postcard" => PrintPageSize::Postcard,
            "letter" => PrintPageSize::Letter,
            "legal" => PrintPageSize::Legal,
            _ => return Err(sqlx::error::Error::InvalidArgument("Unrecognized status".to_string()))
        };
        

        let uuid = Uuid::parse_str(row.try_get("job_uuid")?)
            .map_err(|e| {sqlx::Error::InvalidArgument(e.to_string())})?;


        Ok(PrintJob {
            id: uuid,
            filename: row.try_get("filename")?,
            printer: row.try_get("printer_name")?,
            vendor: row.try_get("vendor")?,
            model: row.try_get("model")?,
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
            page_size,
        })
    }
}

impl PrintJob {
    pub fn new(filename: String, printer: String, vendor: String, model: String, request: PrintRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            filename,
            printer,
            vendor,
            model,
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
            page_size: request.page_size.unwrap_or(PrintPageSize::A4),
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
    
    pub fn set_cups_job_id(&mut self, cups_job_id: i32) {
        self.cups_job_id = Some(cups_job_id);
    }

    pub fn get_file_path(&self) -> Option<String> {
        Some(format!("uploads/{}", self.filename))
    }


    pub async fn save_to_db(&self, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let status_str = self.status.to_string();
        let page_size_str = self.page_size.to_string();

        let format = Path::new(self.filename.as_str()).extension().and_then(OsStr::to_str);

        let query = query_bind!(
            r#"
            INSERT INTO print_jobs (
                job_uuid, cups_id_job, printer_name, vendor, model, filename, filepath, status,
                created_at, started_at, completed_at, error_message, copies,
                pages_range, duplex, color, page_size, original_filename, mime_type
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
            "#,
            self.id.to_string(),
            self.cups_job_id,
            self.printer.clone(),
            self.vendor.clone(),
            self.model.clone(),
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
            page_size_str,
            self.filename.clone(),
            format
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }


    pub async fn update_in_db(&self, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let status_str = self.status.to_string();

        let query = query_bind!(
            r#"
            UPDATE print_jobs
            SET cups_id_job = ?,status = ?, started_at = ?, completed_at = ?, error_message = ? WHERE job_uuid = ?;
            "#,
            self.cups_job_id,
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

    pub async fn find_by_statuses(statuses: Vec<PrintJobStatus>, pool: &SqlitePool) -> Result<Vec<PrintJob>, sqlx::Error> {
        let placeholders = statuses.iter().map(|_| {"?"}).collect::<Vec<_>>().join(",");
        let statuses = statuses.iter().map(|s| {s.to_string()}).collect::<Vec<String>>();

        let query_str = format!(
            "SELECT * FROM print_jobs WHERE status IN ({}) ORDER BY created_at ASC;",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for status in statuses {
            query = query.bind(status);
        }

        let rows = query.fetch_all(pool).await?;
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