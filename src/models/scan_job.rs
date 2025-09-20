use std::any::Any;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use log::Record;
use sqlx::{Row, SqlitePool};
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;
use crate::query_bind;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanJob {
    pub id: Uuid,
    pub scanner: String,
    pub status: ScanJobStatus,
    pub resolution: u32,
    pub format: ScanFormat,
    pub color_mode: ColorMode,
    pub page_size: PageSize,    // Not used
    pub brightness: i32,        // Not used
    pub contrast: i32,          // Not used
    pub output_filename: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ScanJobStatus {
    Queued,
    Scanning,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ScanFormat {
    Pdf,
    Jpeg,
    Png,
    Tiff,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Color,
    Grayscale,
    Monochrome,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PageSize {
    A4,
    Letter,
    Legal,
    A3,
    Custom,
}

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub scanner: Option<String>,
    pub resolution: Option<u32>,
    pub format: Option<ScanFormat>,
    pub color_mode: Option<ColorMode>,
    pub page_size: Option<PageSize>,
    pub brightness: Option<i32>,
    pub contrast: Option<i32>,
}

impl TryFrom<&SqliteRow> for ScanJob {
    type Error = sqlx::Error;

    fn try_from(record: &SqliteRow) -> Result<Self, Self::Error> {
        let status = match record.try_get("status")? {
            "queued" => ScanJobStatus::Queued,
            "scanning" => ScanJobStatus::Scanning,
            "processing" => ScanJobStatus::Processing,
            "completed" => ScanJobStatus::Completed,
            "failed" => ScanJobStatus::Failed,
            "cancelled" => ScanJobStatus::Cancelled,
            _ => return Err(sqlx::error::Error::InvalidArgument("Unrecognized status".to_string()))
        };

        let format = match record.try_get("format")? {
            "pdf" => ScanFormat::Pdf,
            "jpeg" | "jpg" => ScanFormat::Jpeg,
            "png" => ScanFormat::Png,
            "tiff" => ScanFormat::Tiff,
            _ => return Err(sqlx::error::Error::InvalidArgument("Unrecognized format".to_string()))
        };

        let color_mode = match record.try_get("color_mode")? {
            "color" => ColorMode::Color,
            "grayscale" => ColorMode::Grayscale,
            "monochrome" => ColorMode::Monochrome,
            _ =>  return Err(sqlx::Error::InvalidArgument("Unrecognized color mode".to_string()))
        };

        let page_size = match record.try_get("page_size")? {
            "a4" => PageSize::A4,
            "a3" => PageSize::A3,
            "letter" => PageSize::Letter,
            "legal" => PageSize::Legal,
            "custom" => PageSize::Custom,
            _ => return Err(sqlx::Error::InvalidArgument("Unrecognized page size".to_string()))
        };

        let uuid = Uuid::parse_str(record.try_get("job_uuid")?)
            .map_err(|e| {sqlx::Error::InvalidArgument(e.to_string())})?;

        Ok(ScanJob {
            id: uuid,
            scanner: record.try_get("scanner_name")?,
            status,
            resolution: record.try_get("resolution")?,
            format,
            color_mode,
            page_size,
            brightness: record.try_get("brightness")?,
            contrast: record.try_get("contrast")?,
            output_filename: record.try_get("filename")?,
            created_at: record.try_get("created_at")?,
            started_at: record.try_get("started_at")?,
            completed_at: record.try_get("completed_at")?,
            error_message: record.try_get("error_message")?,
            file_size: record.try_get("file_size")?,
        })
    }
}



impl ScanJob {
    pub fn new(scanner: String, request: ScanRequest) -> Self {
        let id = Uuid::new_v4();
        let format = request.format.unwrap_or(ScanFormat::Pdf);
        let extension = match format {
            ScanFormat::Pdf => "pdf",
            ScanFormat::Jpeg => "jpg",
            ScanFormat::Png => "png",
            ScanFormat::Tiff => "tiff",
        };

        Self {
            id,
            scanner,
            status: ScanJobStatus::Queued,
            resolution: request.resolution.unwrap_or(300),
            format,
            color_mode: request.color_mode.unwrap_or(ColorMode::Color),
            page_size: request.page_size.unwrap_or(PageSize::A4),
            brightness: request.brightness.unwrap_or(0),
            contrast: request.contrast.unwrap_or(0),
            output_filename: Some(format!("scan_{}_{}.{}",
                                          Utc::now().format("%Y%m%d_%H%M%S"),
                                          &id.to_string()[..8],
                                          extension
            )),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            file_size: None,
        }
    }

    pub fn set_status(&mut self, status: ScanJobStatus) {
        self.status = status;
        match &self.status {
            ScanJobStatus::Scanning => {
                if self.started_at.is_none() {
                    self.started_at = Some(Utc::now());
                }
            },
            ScanJobStatus::Completed | ScanJobStatus::Failed | ScanJobStatus::Cancelled => {
                if self.completed_at.is_none() {
                    self.completed_at = Some(Utc::now());
                }
            },
            _ => {}
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.set_status(ScanJobStatus::Failed);
    }

    pub fn get_file_path(&self) -> Option<String> {
        self.output_filename.as_ref().map(|filename| {
            format!("scans/{}", filename)
        })
    }


    pub async fn save_to_db(&self, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let format_str = match self.format {
            ScanFormat::Pdf => { "pdf" }
            ScanFormat::Jpeg => { "jpeg" }
            ScanFormat::Png => { "png" }
            ScanFormat::Tiff => { "tiff" }
        };

        let color_mode_str = match self.color_mode {
            ColorMode::Color => { "color" }
            ColorMode::Grayscale => { "grayscale" }
            ColorMode::Monochrome => { "monochrome" }
        };

        let status_str = match self.status {
            ScanJobStatus::Queued => { "queued" }
            ScanJobStatus::Scanning => { "scanning" }
            ScanJobStatus::Processing => { "processing" }
            ScanJobStatus::Completed => { "completed" }
            ScanJobStatus::Failed => { "failed" }
            ScanJobStatus::Cancelled => { "cancelled" }
        };

        let page_size_str = match self.page_size {
            PageSize::A4 => { "a4" },
            PageSize::A3 => { "a3" },
            PageSize::Letter => { "letter" }
            PageSize::Legal => { "legal" }
            PageSize::Custom => { "custom" }
        };

        let query = query_bind!(
            r#"
            INSERT INTO scan_jobs (
                job_uuid, scanner_name, filename, file_path, status,
                created_at, started_at, completed_at, error_message,
                resolution, format, color_mode, page_size, brightness, contrast, file_size
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id;
            "#,
            self.id.to_string(),
            self.scanner.clone(),
            self.output_filename.clone().unwrap_or(format!("unnamed-{}", self.id)),
            self.get_file_path(),
            status_str,
            self.created_at,
            self.started_at,
            self.completed_at,
            self.error_message.clone(),
            self.resolution as i32,
            format_str,
            color_mode_str,
            page_size_str,
            self.brightness,
            self.contrast,
            self.file_size.map(|s| s as i64)
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }

    pub async fn update_in_db(&self, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let status_str = match self.status {
            ScanJobStatus::Queued => "queued",
            ScanJobStatus::Scanning => "scanning",
            ScanJobStatus::Processing => "processing",
            ScanJobStatus::Completed => "completed",
            ScanJobStatus::Failed => "failed",
            ScanJobStatus::Cancelled => "cancelled",
        };

        let query = query_bind!(
            r#"
            UPDATE scan_jobs
            SET status = ?, started_at = ?, completed_at = ?, error_message = ?, file_size = ? WHERE job_uuid = ?;
            "#,
            status_str,
            self.started_at,
            self.completed_at,
            self.error_message.clone(),
            self.file_size.map(|s| s as i64),
            self.id.to_string()
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }

    pub async fn remove_from_db(id: Uuid, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let query = query_bind!(
            r#"
            DELETE FROM scan_jobs WHERE job_uuid = ?;
            "#,
            id.to_string()
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }

    pub async fn find_by_uuid(uuid: Uuid, pool: &SqlitePool) -> Result<Option<ScanJob>, sqlx::Error> {
        let row_op = query_bind!(
            r#"
            SELECT * FROM scan_jobs WHERE job_uuid = ?;
            "#,
            uuid.to_string()
        ).fetch_optional(pool).await?;

        if let Some(row) = row_op {
            log::info!("Successfully fetched row");
            Ok(Some(ScanJob::try_from(&row)?))
        } else {
            log::warn!("No rows was fetched");
            Ok(None)
        }
    }

    pub async fn get_recent(limit: u32, pool: &SqlitePool ) -> Result<Vec<ScanJob>, sqlx::Error> {
        let rows = query_bind!(r#"
        SELECT * FROM scan_jobs
        ORDER BY created_at DESC
        LIMIT ?;
        "#,
        limit,
        ).fetch_all(pool).await?;

        let scan_jobs = rows
            .iter()
            .map(ScanJob::try_from)
            .collect::<Result<Vec<ScanJob>, sqlx::Error>>();

        Ok(scan_jobs?)
    }

    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<ScanJob>, sqlx::Error> {
        let rows = sqlx::query(r#"
            SELECT * FROM scan_jobs
        "#
        ).fetch_all(pool).await?;

        let scan_jobs = rows.iter().map(ScanJob::try_from).collect::<Result<Vec<ScanJob>, sqlx::Error>>();

        Ok(scan_jobs?)

    }

    pub async fn get_by_status(status: ScanJobStatus, pool: &SqlitePool) -> Result<Vec<ScanJob>, sqlx::Error> {
        let status_str = match status {
            ScanJobStatus::Queued => { "queued" }
            ScanJobStatus::Scanning => { "scanning" }
            ScanJobStatus::Processing => { "processing" }
            ScanJobStatus::Completed => { "completed" }
            ScanJobStatus::Failed => { "failed" }
            ScanJobStatus::Cancelled => { "cancelled" }
        };

        let rows = query_bind!(r#"
            SELECT * FROM scan_jobs
            WHERE status = ?
            ORDER BY created_at ASC;
        "#,
        status_str).fetch_all(pool).await?;

        let scan_jobs = rows.iter().map(ScanJob::try_from).collect::<Result<Vec<ScanJob>, sqlx::Error>>();

        Ok(scan_jobs?)
    }
}



