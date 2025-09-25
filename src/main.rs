use actix_files::Files;
use actix_web::{web, App, HttpServer, middleware::Logger};
use std::io;
use tokio::time::Instant;

mod handlers;
mod services;
mod models;
mod utils;
mod database;

use handlers::{print, scan, system};
use crate::database::init_database;

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Default logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("Starting Print/Scan Manager server");

    // Creation of necessary directories
    std::fs::create_dir_all("uploads").unwrap_or_else(|e| {
        log::warn!("Could not create uploads directory: {}", e);
    });
    std::fs::create_dir_all("scans").unwrap_or_else(|e| {
        log::warn!("Could not create scans directory: {}", e);
    });

    log::info!("Starting database local server");
    let pool = init_database().await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;


    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // API routes
            .service(
                web::scope("/api")
                    // Print endpoints
                    .route("/printers", web::get().to(print::list_printers))
                    .route("/print", web::post().to(print::submit_print_job))
                    .route("/print/jobs", web::get().to(print::list_print_jobs))
                    .route("/print/jobs/{job_id}", web::get().to(print::get_print_job))
                    .route("/print/jobs/{job_id}", web::delete().to(print::cancel_print_job))

                    // Scan endpoints
                    .route("/scanners", web::get().to(scan::list_scanners))
                    .route("/scan", web::post().to(scan::start_scan))
                    .route("/scan/jobs", web::get().to(scan::list_scan_jobs))
                    .route("/scan/jobs/{job_id}", web::get().to(scan::get_scan_job))
                    .route("/scan/download/{job_id}", web::get().to(scan::download_scan))


                    // System endpoints
                    .route("/system/status", web::get().to(system::get_status))
                    .route("/system/settings", web::get().to(system::get_settings))
                    .route("/system/settings", web::post().to(system::update_settings))
                    .route("/system/nozzle/check", web::post().to(system::nozzle_check))
                    .route("/system/nozzle/clean", web::post().to(system::nozzle_clean))

                    // File management
                    .route("/files/uploads", web::get().to(system::list_uploads))
                    .route("/files/uploads/{filename}", web::delete().to(system::delete_upload))
                    .route("/files/scans", web::get().to(system::list_scans))
                    .route("/files/scans/{filename}", web::delete().to(system::delete_scan))
            )
            // Web pages
            .route("/", web::get().to(system::index))
            .route("/print", web::get().to(system::print_page))
            .route("/scan", web::get().to(system::scan_page))

            // Static files
            .service(Files::new("/static", "./static").show_files_listing())

            // JSON payload size (for file uploads)
            .app_data(web::PayloadConfig::new(50 * 1024 * 1024)) // 50MB max
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(models::AppState { start_time: Instant::now() }))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
