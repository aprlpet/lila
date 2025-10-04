mod auth;
mod config;
mod error;
mod handlers;
mod models;
mod storage;

use std::{sync::Arc, time::Duration};

use axum::{
    Router, middleware,
    routing::{delete, get, put},
};
use handlers::objects::AppState;
use storage::{FileStorage, MetadataStore};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    cors::CorsLayer,
};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lila=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting lila");
    tracing::info!("Created by april");

    let config = models::Config::load()?;
    tracing::info!("Configuration loaded successfully");
    tracing::debug!(
        "Server will bind to {}:{}",
        config.server_host,
        config.server_port
    );
    tracing::debug!("Storage path: {}", config.storage_path);
    tracing::debug!("Database URL: {}", config.database_url);
    tracing::debug!(
        "Rate limit: {} req/s, burst: {}",
        config.rate_limit_per_second,
        config.rate_limit_burst_size
    );
    tracing::debug!("Max upload size: {} MB", config.max_upload_size_mb);

    let metadata = MetadataStore::new(&config.database_url).await?;
    tracing::info!("Metadata store initialized");

    let storage = FileStorage::new(&config.storage_path).await?;
    tracing::info!("File storage initialized");

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(config.rate_limit_per_second)
            .burst_size(config.rate_limit_burst_size)
            .finish()
            .unwrap(),
    );

    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(interval);
            tracing::debug!("rate limit storage size: {}", governor_limiter.len());
            governor_limiter.retain_recent();
        }
    });

    let state = AppState {
        metadata,
        storage,
        auth_token: config.auth_token.clone(),
        max_upload_size: config.max_upload_size_mb,
    };

    let cors = CorsLayer::permissive();

    let protected_routes = Router::new()
        .route("/api/v1/objects", get(handlers::objects::list_objects))
        .route("/api/v1/objects/{*key}", put(handlers::objects::put_object))
        .route("/api/v1/objects/{*key}", get(handlers::objects::get_object))
        .route(
            "/api/v1/objects/{*key}",
            delete(handlers::objects::delete_object),
        )
        .route(
            "/api/v1/metadata/{*key}",
            get(handlers::objects::get_object_metadata),
        )
        .route(
            "/api/v1/info/{*key}",
            get(handlers::objects::get_object_info),
        )
        .route(
            "/api/v1/folders/{*prefix}",
            delete(handlers::objects::delete_folder),
        )
        .route("/api/v1/stats", get(handlers::stats::get_stats))
        .route("/api/v1/search", get(handlers::objects::search_objects))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        .layer(GovernorLayer::new(governor_conf.clone()));

    let app = Router::new()
        .route("/", get(handlers::index::index))
        .route("/favicon.ico", get(handlers::index::favicon))
        .route("/github", get(handlers::index::github_redirect))
        .merge(protected_routes)
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);

    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("GitHub: https://github.com/aprlpet/lila");

    axum::serve(listener, app).await?;

    Ok(())
}