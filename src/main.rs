use actix_session::{SessionExt, SessionMiddleware, storage::RedisSessionStore};
use actix_files::Files;
use actix_web::{web, App, HttpServer, middleware::Logger, cookie::Key};
use actix_limitation::{Limiter, RateLimiter};
use std::{io, io::BufReader};
use std::time::Duration;
use rustls::ServerConfig;
use std::fs::File;
mod handlers;
mod services;
mod models;
mod utils;
mod database;

use handlers::{print, scan, system, events};
use crate::database::init_database;
use crate::models::{AppState, JobQueue};


const REDIS_URL: &str = "redis://127.0.0.1:6379";
const BIND_ADDRESS: &str = "0.0.0.0:8080";
const MAX_UPLOAD_SIZE: usize = 50 * 1024 * 1024;

fn get_tls_config() -> Result<ServerConfig, io::Error> {
    let mut certs_file = BufReader::new(File::open("certs/cert.pem")?);
    let mut keys_file = BufReader::new(File::open("certs/key.pem")?);

    let tls_certs = rustls_pemfile::certs(&mut certs_file)
        .collect::<Result<Vec<_>, _>>()?;
    let tls_keys = rustls_pemfile::pkcs8_private_keys(&mut keys_file)
        .next().unwrap()?;

    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_keys))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    Ok(tls_config)
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default().map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to install crypto provider"))?;

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

    // Setting up Redis session store
    let redis_connection_string = REDIS_URL;
    log::info!("Connecting to Redis for sessions...");
    let redis_store = RedisSessionStore::new(redis_connection_string)
        .await
        .map_err(|e| {
            log::error!("Failed to connect to Redis: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;
    log::info!("Successfully connected to Redis session store");

    let secret_key = if let Ok(key_str) = std::env::var("WEB_PRINTER_SESSION_SECRET_KEY") {
        let key_bytes = key_str.as_bytes();
        if key_bytes.len() != 64 {
            log::error!("WEB_PRINTER_SESSION_SECRET_KEY must be exactly 64 bytes, got {}", key_bytes.len());
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid secret key length"));
        }
        Key::from(key_bytes)
    } else {
        log::warn!("WEB_PRINTER_SESSION_SECRET_KEY not set, generating random key");
        Key::generate()
    };

    let limiter = web::Data::new(
        Limiter::builder(redis_connection_string)
            .key_by(|req| {
                req.get_session()
                    .get::<String>("user_id")
                    .ok()
                    .flatten()
                    .or_else(|| {
                        req.cookie("rate-api-id")
                            .map(|c| c.value().to_string())
                    })
                    .or_else(|| {
                        req.connection_info()
                            .realip_remote_addr()
                            .map(|s| s.to_string())
                    })
            })
            .limit(120usize)
            .period(Duration::from_secs(60u64))
            .build()
            .map_err(|e| {
                log::error!("Redis error: {}", e);
                io::Error::new(io::ErrorKind::Other, e) })?,
    );
    log::info!("Successfully created rate-limiter");

    let tls_config = get_tls_config().map_err(|e| {
        log::error!("Failed to load TLS certificates: {}. Have you created cert files?", e);
        e
    })?;
    log::info!("Successfully loaded cert files");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(
                SessionMiddleware::builder(redis_store.clone(), secret_key.clone())
                    .cookie_name("session-id".to_string())
                    .cookie_secure(true)
                    .cookie_http_only(true)
                    .cookie_same_site(actix_web::cookie::SameSite::Lax)
                    .session_lifecycle(
                        actix_session::config::PersistentSession::default()
                            .session_ttl(actix_web::cookie::time::Duration::hours(24i64))
                    )
                    .build()
            )
            .wrap(RateLimiter::default())
            .app_data(limiter.clone())

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
            .app_data(web::PayloadConfig::new(MAX_UPLOAD_SIZE))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(job_queue.clone()))
            .app_data(web::Data::new(event_state.clone()))
    })
        .bind(BIND_ADDRESS)?
        // .bind_rustls_0_23(BIND_ADDRESS, tls_config)?
        .workers(4)
        .run()
        .await
}