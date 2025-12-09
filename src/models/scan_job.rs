use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;
use std::path::Path;
use crate::query_bind;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanJob {
    pub id: Uuid,
    pub scanner: String,
    pub vendor: String,
    pub model: String,
    pub status: ScanJobStatus,
    pub resolution: u32,
    pub format: ScanFormat,
    pub color_mode: ColorMode,
    pub page_size: ScanPageSize,    // Not used
    pub brightness: i32,        // Not used
    pub contrast: i32,          // Not used
    pub output_filename: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub file_size: Option<u64>,
    pub file_available: bool
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
pub enum ScanPageSize {
    A4,
    A5,
    Letter,
    Legal,
    Custom,
}

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub scanner: Option<String>,
    pub resolution: Option<u32>,
    pub format: Option<ScanFormat>,
    pub color_mode: Option<ColorMode>,
    pub page_size: Option<ScanPageSize>,
    pub brightness: Option<i32>,
    pub contrast: Option<i32>,
    pub filename: Option<String>,
}

impl Display for ScanJobStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let status_str = match self {
            ScanJobStatus::Queued => { "queued" }
            ScanJobStatus::Scanning => { "scanning" }
            ScanJobStatus::Processing => { "processing" }
            ScanJobStatus::Completed => { "completed" }
            ScanJobStatus::Failed => { "failed" }
            ScanJobStatus::Cancelled => { "cancelled" }
        };

        f.write_str(status_str)
    }
}


impl Eq for ScanJob {}

impl PartialEq<Self> for ScanJob {
    fn eq(&self, other: &Self) -> bool {
        self.completed_at == other.completed_at
    }
}

impl PartialOrd<Self> for ScanJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScanJob {
    fn cmp(&self, other: &Self) -> Ordering {
        self.completed_at.cmp(&other.completed_at)
    }
}

impl TryFrom<&SqliteRow> for ScanJob {
    type Error = sqlx::Error;

    fn try_from(row: &SqliteRow) -> Result<Self, Self::Error> {
        let status = match row.try_get("status")? {
            "queued" => ScanJobStatus::Queued,
            "scanning" => ScanJobStatus::Scanning,
            "processing" => ScanJobStatus::Processing,
            "completed" => ScanJobStatus::Completed,
            "failed" => ScanJobStatus::Failed,
            "cancelled" => ScanJobStatus::Cancelled,
            _ => return Err(sqlx::error::Error::InvalidArgument("Unrecognized status".to_string()))
        };

        let format = match row.try_get("format")? {
            "pdf" => ScanFormat::Pdf,
            "jpeg" | "jpg" => ScanFormat::Jpeg,
            "png" => ScanFormat::Png,
            "tiff" => ScanFormat::Tiff,
            _ => return Err(sqlx::error::Error::InvalidArgument("Unrecognized format".to_string()))
        };

        let color_mode = match row.try_get("color_mode")? {
            "color" => ColorMode::Color,
            "grayscale" => ColorMode::Grayscale,
            "monochrome" => ColorMode::Monochrome,
            _ =>  return Err(sqlx::Error::InvalidArgument("Unrecognized color mode".to_string()))
        };

        let page_size = match row.try_get("page_size")? {
            "a4" => ScanPageSize::A4,
            "a5" => ScanPageSize::A5,
            "letter" => ScanPageSize::Letter,
            "legal" => ScanPageSize::Legal,
            "custom" => ScanPageSize::Custom,
            _ => return Err(sqlx::Error::InvalidArgument("Unrecognized page size".to_string()))
        };

        let uuid = Uuid::parse_str(row.try_get("job_uuid")?)
            .map_err(|e| {sqlx::Error::InvalidArgument(e.to_string())})?;

        Ok(ScanJob {
            id: uuid,
            scanner: row.try_get("scanner_name")?,
            vendor: row.try_get("vendor")?,
            model: row.try_get("model")?,
            status,
            resolution: row.try_get("resolution")?,
            format,
            color_mode,
            page_size,
            brightness: row.try_get("brightness")?,
            contrast: row.try_get("contrast")?,
            output_filename: row.try_get("filename")?,
            created_at: row.try_get("created_at")?,
            started_at: row.try_get("started_at")?,
            completed_at: row.try_get("completed_at")?,
            error_message: row.try_get("error_message")?,
            file_size: row.try_get("file_size")?,
            file_available: row.try_get("file_available")?,
        })
    }
}



impl ScanJob {
    pub fn new(scanner: String, vendor: String, model: String, request: ScanRequest) -> Self {
        let id = Uuid::new_v4();
        let format = request.format.unwrap_or(ScanFormat::Pdf);
        let extension = match format {
            ScanFormat::Pdf => "pdf",
            ScanFormat::Jpeg => "jpg",
            ScanFormat::Png => "png",
            ScanFormat::Tiff => "tiff",
        };

        let mut filename = request.filename.and_then(|s| Some(add_missing_extension(&s, extension)))
            .unwrap_or_else(|| format!("scan_{}_{}.{}", Utc::now().format("%Y%m%d_%H%M%S"), &id.to_string()[..8], extension));

        validate_filename(&mut filename);

        Self {
            id,
            scanner,
            vendor,
            model,
            status: ScanJobStatus::Queued,
            resolution: request.resolution.unwrap_or(300),
            format,
            color_mode: request.color_mode.unwrap_or(ColorMode::Color),
            page_size: request.page_size.unwrap_or(ScanPageSize::A4),
            brightness: request.brightness.unwrap_or(0),
            contrast: request.contrast.unwrap_or(0),
            output_filename: Some(filename),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            file_size: None,
            file_available: false
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
            ScanPageSize::A4 => { "a4" },
            ScanPageSize::A5 => { "a5" },
            ScanPageSize::Letter => { "letter" }
            ScanPageSize::Legal => { "legal" }
            ScanPageSize::Custom => { "custom" }
        };

        let query = query_bind!(
            r#"
            INSERT INTO scan_jobs (
                job_uuid, scanner_name, vendor, model, filename, file_path, status,
                created_at, started_at, completed_at, error_message, resolution,
                format, color_mode, page_size, brightness, contrast, file_size, file_available
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id;
            "#,
            self.id.to_string(),
            self.scanner.clone(),
            self.vendor.clone(),
            self.model.clone(),
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
            self.file_size.map(|s| s as i64),
            self.file_available
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }

    pub async fn update_statues_in_db(&self, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
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
            SET status = ?, started_at = ?, completed_at = ?, error_message = ?, file_size = ?, file_available = ? WHERE job_uuid = ?;
            "#,
            status_str,
            self.started_at,
            self.completed_at,
            self.error_message.clone(),
            self.file_size.map(|s| s as i64),
            self.file_available,
            self.id.to_string()
        ).execute(pool).await?;

        Ok(query.rows_affected())
    }

    pub async fn remove_by_uuid(id: Uuid, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
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
        LIMIT ?
        ;"#,
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
            ORDER BY created_at DESC;
        "#
        ).fetch_all(pool).await?;
    
        let scan_jobs = rows.iter().map(ScanJob::try_from).collect::<Result<Vec<ScanJob>, sqlx::Error>>();
    
        Ok(scan_jobs?)
    }

    pub async fn find_by_statuses(statuses: Vec<ScanJobStatus>, pool: &SqlitePool) -> Result<Vec<ScanJob>, sqlx::Error> {
        let placeholders = statuses.iter().map(|_| {"?"}).collect::<Vec<_>>().join(",");
        let statuses = statuses.iter().map(|s| {s.to_string()}).collect::<Vec<String>>();

        let query_str = format!(
            "SELECT * FROM scan_jobs WHERE status IN ({}) ORDER BY created_at ASC;",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for status in statuses {
            query = query.bind(status);
        }

        let rows = query.fetch_all(pool).await?;
        let scan_jobs = rows.iter()
            .map(|x| {ScanJob::try_from(x)}).collect::<Result<Vec<ScanJob>, sqlx::Error>>();

        Ok(scan_jobs?)
    }
    
    pub async fn update_file_available_by_filename(filename: String, status: bool, pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let query = query_bind!(
            r#"
            UPDATE scan_jobs
            SET file_available = ? WHERE filename = ?;
            "#,
            status,
            filename
        ).execute(pool).await?;
        
        Ok(query.rows_affected())
    }
}


/// Helper function to add extension to filename if missing
fn add_missing_extension(filename: &str, extension: &str) -> String {
    let path = Path::new(filename);

    if path.extension().is_some() {
        return filename.to_string();
    }

    let ext = extension.trim_start_matches('.');

    format!("{}.{}", filename, ext)
}


fn is_file_existing(filename: &str) -> bool {
    let filepath = format!("scans/{}", filename);
    let path = Path::new(filepath.as_str());

    match path.try_exists() {
        Ok(true) => {
            path.is_file()
        }
        Ok(false) => {
            false
        }
        _ => {
            log::warn!("Error during checking file: {}", filepath);
            false
        }
    }
}

fn validate_filename(filename: &mut String) -> &mut String {
    let mut is_first_encounter = true;
    loop {
        match is_file_existing(&filename) {
            true => {
                let index = filename.find('.').unwrap();
                let mut count = 0;
                if let Some(slice) = filename.split('_').last() {
                    let number = slice
                        .split('.')
                        .take(1).collect::<String>()
                        .parse::<i32>().unwrap_or_else(|e| {
                        log::warn!("Could not parse the string {}", e); 0});

                    count = number + 1i32;
                }
                if is_first_encounter {
                    filename.insert_str(index, format!("_{count}").as_str());
                    is_first_encounter = false;
                } else {
                    let start = filename.find('_').unwrap();
                    let end = filename.find('.').unwrap();
                    filename.replace_range(start..end, format!("_{count}").as_str());
                }
            },
            false => break
        }
    }

    filename
}




mod tests {
    use super::*;

    #[test]
    fn validating_filename_test_existing() {
        let mut test_filename = String::from("scan.png");

        validate_filename(&mut test_filename);

        assert_eq!(test_filename, String::from("scan_2.png"));

    }
}


