use sqlx::SqlitePool;

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS scan_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_uuid TEXT UNIQUE NOT NULL,
            scanner_name TEXT NOT NULL,
            filename TEXT NOT NULL,
            file_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'queued',
            created_at DATETIME NOT NULL,
            started_at DATETIME,
            completed_at DATETIME,
            error_message TEXT,
            resolution INTEGER NOT NULL DEFAULT 300,
            format TEXT NOT NULL DEFAULT 'pdf',
            color_mode TEXT NOT NULL DEFAULT 'color',
            page_size TEXT NOT NULL DEFAULT 'a4',
            brightness INTEGER NOT NULL DEFAULT 0,
            contrast INTEGER NOT NULL DEFAULT 0,
            file_size INTEGER,
            page_count INTEGER,
            file_available BOOLEAN NOT NULL DEFAULT false
        )
        ;"#
    ).execute(pool).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS print_jobs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_uuid TEXT UNIQUE NOT NULL,
            cups_id_job INTEGER,
            printer_name TEXT NOT NULL,
            filename TEXT NOT NULL,
            filepath TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'queued',
            created_at DATETIME NOT NULL,
            started_at DATETIME,
            completed_at DATETIME,
            error_message TEXT,
            copies INTEGER NOT NULL DEFAULT 1,
            pages_range TEXT,
            duplex BOOLEAN DEFAULT false,
            color BOOLEAN DEFAULT true,
            page_size TEXT NOT NULL DEFAULT 'a4',
            original_filename TEXT,
            mime_type TEXT
        )
        ;"#
    ).execute(pool).await?;

    // Pi Zero 2W optimizations
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_jobs_status ON scan_jobs(status)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_jobs_created ON scan_jobs(created_at)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_scan_jobs_uuid ON scan_jobs(job_uuid)")
        .execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_print_jobs_status ON print_jobs(status)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_print_jobs_created ON print_jobs(created_at)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_print_jobs_uuid ON print_jobs(job_uuid)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_print_jobs_cups ON print_jobs(cups_id_job)")
        .execute(pool).await?;

    Ok(())
}