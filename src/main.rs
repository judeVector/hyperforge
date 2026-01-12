use hyper::{Method, server::conn::http1};
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower::{ServiceBuilder, limit::ConcurrencyLimitLayer, timeout::TimeoutLayer};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

mod handler;
mod metric;
mod model;

use handler::{handle_request, shutdown_signal};
use metric::MetricsCollector;

#[derive(Clone)]
pub struct AppState {
    db_pool: PgPool,
    metrics: Arc<MetricsCollector>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE URL must be set");

    info!("Connecting to database...");

    let pool = PgPool::connect(&database_url).await?;

    info!("Connected to database successfully!");

    info!("Setting up database schema...");
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                email VARCHAR(255) NOT NULL UNIQUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
    )
    .execute(&pool)
    .await?;

    info!("Database schema ready!");

    let state = AppState {
        db_pool: pool,
        metrics: Arc::new(MetricsCollector::new()),
    };

    let tower_service = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(Any),
        )
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(ConcurrencyLimitLayer::new(100))
        .service_fn(move |req| {
            let state = state.clone();
            handle_request(req, state)
        });

    let hyper_service = TowerToHyperService::new(tower_service);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Starting server on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    let http = http1::Builder::new();
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();

    let mut signal = std::pin::pin!(shutdown_signal());

    loop {
        tokio::select! {
            Ok((stream, _addr)) = listener.accept() => {
                let io = TokioIo::new(stream);
                let svc = hyper_service.clone();
                let conn = http.serve_connection(io, svc);
                // watch this connection
                let fut = graceful.watch(conn);
                tokio::spawn(async move {
                    if let Err(e) = fut.await {
                        eprintln!("Error serving connection: {:?}", e);
                    }
                });
            },

            _ = &mut signal => {
                drop(listener);
                eprintln!("graceful shutdown signal received");
                // stop the accept loop
                break;
            }
        }
    }

    tokio::select! {
        _ = graceful.shutdown() => {
            eprintln!("all connections gracefully closed");
        },
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            eprintln!("timed out wait for all connections to close");
        }
    }

    Ok(())
}
