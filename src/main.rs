use crate::{
    app::App,
    auth::{ApiKeys, require_auth},
    counter::{ClickCounter, start_counter_flusher},
    db::PostgresDb,
    db_pool::DbPool,
    handler::{handle_create, handle_health, handle_public_create, handle_redirect},
    migrations::run_migrations,
};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use axum_turnstile::TurnstileLayer;
use clap::Parser;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod auth;
mod counter;
mod db;
mod db_pool;
mod handler;
mod migrations;
mod models;
mod schema;
mod signals;

#[cfg(not(debug_assertions))]
#[must_use]
pub const fn is_debug() -> bool {
    false
}

#[cfg(debug_assertions)]
#[must_use]
pub const fn is_debug() -> bool {
    true
}

#[derive(Default, Parser, Debug)]
struct Arguments {
    #[arg(long, default_value_t = true, help = "Relax CORS", env = "RELAX_CORS")]
    cors_relaxed: bool,

    #[arg(long, default_value_t = 8080, help = "Port to listen on", env = "PORT")]
    port: u16,

    #[arg(long, default_value_t = 100, help = "Cache size", env = "CACHE_SIZE")]
    cache_size: usize,

    #[arg(long, default_value_t = 6, help = "Hash length", env = "HASH_LENGTH")]
    hash_length: usize,

    #[arg(
        long,
        default_value_t = 3,
        help = "Stats DB Flush Interval",
        env = "STATS_FLUSH_INTERVAL_SECS"
    )]
    stats_flush_interval_secs: u64,

    #[arg(long, help = "Logging level of the Rust log", env = "RUST_LOG")]
    #[clap(default_value_t = String::from("info,tower_http=debug"))]
    rust_log_level: String,

    #[arg(long, env = "DATABASE_URL")]
    db_url: String,

    #[arg(
        long,
        default_value_t = 10,
        help = "DB pool size",
        env = "DB_POOL_SIZE"
    )]
    db_pool_size: usize,

    #[arg(long, default_value_t = String::from("http://localhost:8080"), env = "URL_PREFIX")]
    url_prefix: String,

    #[arg(long, default_value_t = String::new(), env = "KEYS")]
    keys: String,

    #[arg(long, default_value_t = String::from("1x0000000000000000000000000000000AA"), env = "TURNSTILE_SECRET")]
    turnstile_secret: String,
}

fn setup_cors(relaxed: bool) -> CorsLayer {
    if relaxed {
        tracing::info!("cors setup: very_permissive");
        CorsLayer::very_permissive().allow_credentials(true)
    } else {
        tracing::info!("cors setup: default");
        CorsLayer::new()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();

    let log_level = args.rust_log_level;

    let cors_relaxed = args.cors_relaxed;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(log_level.clone()))
        .with(tracing_subscriber::fmt::layer().with_ansi(is_debug()))
        .init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Could not install rustls default crypto provider.");

    run_migrations(&args.db_url)?;

    let dbpool = DbPool::build(&args.db_url, args.db_pool_size).await?;

    let counter = Arc::new(ClickCounter::new());

    tokio::spawn(start_counter_flusher(
        Arc::clone(&counter),
        dbpool.clone(),
        Duration::from_secs(args.stats_flush_interval_secs),
    ));

    let api_keys = ApiKeys::new(&args.keys);

    let app = App::new(
        args.url_prefix,
        args.hash_length,
        Arc::new(PostgresDb::new(dbpool)),
        Arc::clone(&counter),
        args.cache_size,
    );

    let pub_api = Router::new()
        .route("/shorten", post(handle_public_create))
        .layer(TurnstileLayer::from_secret(args.turnstile_secret));

    let router = Router::new()
        //authenticated routes
        .route("/link/create", post(handle_create))
        .route_layer(middleware::from_fn_with_state(api_keys, require_auth))
        //public routes
        .route("/{id}", get(handle_redirect))
        .merge(pub_api)
        .route("/health", get(handle_health))
        .layer(TraceLayer::new_for_http())
        .layer(setup_cors(cors_relaxed))
        .with_state(Arc::clone(&app));

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));

    tracing::info!("listening on https://{}", addr);
    tracing::info!("listening on http://{}", addr);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    signals::create_term_signal_handler(tx);

    let listener = TcpListener::bind(addr).await?;

    let server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    );

    let graceful = server.with_graceful_shutdown(async {
        rx.await.ok();
    });

    if let Err(e) = graceful.await {
        tracing::error!("server error: {}", e);
    }

    Ok(())
}
