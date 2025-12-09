use actix_files::Files;
use actix_web::{web, App, HttpServer, middleware::Logger};
use std::io;


mod handlers;
mod services;
mod models;
mod utils;
mod database;

use handlers::{print, scan, system, events};
use crate::database::init_database;
use crate::models::{AppState, JobQueue};

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("Starting Print/Scan Manager server");
    
    std::fs::create_dir_all("uploads").unwrap_or_else(|e| {
        log::warn!("Could not create uploads directory: {}", e);
    });
    std::fs::create_dir_all("scans").unwrap_or_else(|e| {
        log::warn!("Could not create scans directory: {}", e);
    });

    log::info!("Starting database local server");
    let pool = init_database().await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    log::info!("Creating app state");
    let app_state = AppState::new().await;

    log::info!("Found devices:\n{}", app_state.show_devices().await);

    let job_queue = JobQueue::new();
    let event_state = events::EventState::new();

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
                    .route("/print/jobs/{job_id}", web::post().to(print::cancel_print_job))
                    .route("/print/jobs/{job_id}", web::delete().to(print::delete_print_job_record))

                    // Scan endpoints
                    .route("/scanners", web::get().to(scan::list_scanners))
                    .route("/scan", web::post().to(scan::start_scan))
                    .route("/scan/jobs", web::get().to(scan::list_scan_jobs))
                    .route("/scan/jobs/{job_id}", web::get().to(scan::get_scan_job))
                    .route("/scan/jobs/{job_id}", web::delete().to(scan::delete_scan_job_record))
                    .route("/scan/download/{job_id}", web::get().to(scan::download_scan))

                    // System endpoints
                    .route("/system/status", web::get().to(system::get_status))
                    .route("/system/settings", web::get().to(system::get_settings))
                    .route("/system/settings", web::post().to(system::update_settings))
                    .route("/system/nozzle/check", web::post().to(system::nozzle_check))
                    .route("/system/nozzle/clean", web::post().to(system::nozzle_clean))

                    // SSE endpoint
                    .route("/events/stream", web::get().to(events::event_stream))

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
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(job_queue.clone()))
            .app_data(web::Data::new(event_state.clone()))
    })
        .bind("0.0.0.0:8080")?
        .workers(4)
        .run()
        .await
}
