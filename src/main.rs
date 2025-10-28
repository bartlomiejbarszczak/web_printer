use actix_files::Files;
use actix_web::{web, App, HttpServer, middleware::Logger, HttpResponse, Error};
use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_session::{SessionMiddleware, storage::CookieSessionStore, Session, SessionExt};
use actix_web::cookie::Key;
use rustls::{ServerConfig, pki_types::{PrivateKeyDer, CertificateDer}};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::{self, BufReader};
use actix_web::middleware::from_fn;
use actix_web::middleware::Next;
use uuid::Uuid;

mod handlers;
mod services;
mod models;
mod utils;
mod database;

use handlers::{print, scan, system};
use crate::database::init_database;
use crate::models::{AppState, JobQueue};


fn load_rustls_config() -> ServerConfig {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let cert_file = &mut BufReader::new(
        File::open("cert.pem").expect("Failed to open cert.pem")
    );
    let key_file = &mut BufReader::new(
        File::open("key.pem").expect("Failed to open key.pem")
    );

    let cert_chain: Vec<CertificateDer> = certs(cert_file)
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to load certificates");

    let mut keys: Vec<PrivateKeyDer> = pkcs8_private_keys(key_file)
        .map(|key| key.map(Into::into))
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to load private key");

    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))
        .expect("Failed to build TLS config")
}

async fn csrf_middleware(
    req: actix_web::dev::ServiceRequest,
    next: Next<impl actix_web::body::MessageBody>,
) -> Result<actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>, Error> {
    use actix_web::http::Method;

    if matches!(req.method(), &Method::GET | &Method::HEAD | &Method::OPTIONS) {
        return next.call(req).await;
    }

    let session = req.get_session();
    let session_token = session.get::<String>("csrf_token").ok().flatten();

    let header_token = req
        .headers()
        .get("X-CSRF-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    match (session_token, header_token) {
        (Some(sess), Some(head)) if sess == head => {
            next.call(req).await
        }
        _ => {
            log::warn!("CSRF token mismatch for {} {}", req.method(), req.path());
            Err(actix_web::error::ErrorForbidden("CSRF token mismatch"))
        }
    }
}

async fn get_csrf_token(session: Session) -> HttpResponse {
    let token = Uuid::new_v4().to_string();
    if let Err(e) = session.insert("csrf_token", token.clone()) {
        log::error!("Failed to store CSRF token: {}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().json(serde_json::json!({
        "csrf_token": token
    }))
}

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

    log::info!("Initializing database");
    let pool = init_database().await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    log::info!("Creating app state");
    let app_state = AppState::new().await;

    log::info!("Found devices:\n{}", app_state.show_devices().await);

    let job_queue = JobQueue::new();

    let secret_key = Key::generate();

    let governor_conf = GovernorConfigBuilder::default()
        .seconds_per_request(1)
        .burst_size(10)
        .finish()
        .unwrap();

    log::info!("Starting HTTPS server on https://0.0.0.0:8443");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("https://localhost:8443")
            .allowed_origin("https://127.0.0.1:8443")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::ACCEPT,
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::HeaderName::from_static("x-csrf-token"),
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .wrap(Governor::new(&governor_conf))
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key.clone()
                )
                    .cookie_secure(true)  // Only send over HTTPS
                    .cookie_http_only(true)  // Prevent JavaScript access
                    .cookie_same_site(actix_web::cookie::SameSite::Strict)
                    .cookie_name("print-scan-session".to_string())
                    .build()
            )

            .route("/", web::get().to(system::index))
            .route("/print", web::get().to(system::print_page))
            .route("/scan", web::get().to(system::scan_page))

            .route("/api/csrf-token", web::get().to(get_csrf_token))
            .service(
                web::scope("/api")
                    .wrap(from_fn(csrf_middleware))

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
                    .route("/scan/remove/{job_id}", web::delete().to(scan::delete_scan_file))
                    .route("/scan/download/{job_id}", web::get().to(scan::download_scan))

                    // System endpoints
                    .route("/system/status", web::get().to(system::get_status))
                    .route("/system/settings", web::get().to(system::get_settings))
                    .route("/system/settings", web::post().to(system::update_settings))
                    .route("/system/nozzle/check", web::post().to(system::nozzle_check))
                    .route("/system/nozzle/clean", web::post().to(system::nozzle_clean))
                    .route("/system/get-recent", web::get().to(system::get_recent_activity))
            )

            // Static files
            .service(Files::new("/static", "./static").show_files_listing())

            // JSON payload size (for file uploads)
            .app_data(web::PayloadConfig::new(50 * 1024 * 1024)) // 50MB max
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(job_queue.clone()))
    })
        .bind_rustls_0_23("0.0.0.0:8443", load_rustls_config())?
        .run()
        .await
}