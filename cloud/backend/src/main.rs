//! `iomt-backend`: serves the REST API and the gRPC ingest, or seeds a dev database.
//!
//! ```text
//! iomt-backend serve                                  # default
//! iomt-backend migrate                                # apply migrations and exit
//! iomt-backend seed-dev --device-id <32 hex> ...      # dev accounts and wearables
//! ```

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Context as _;
use clap::{Parser, Subcommand};
use iomt_backend::config::Config;
use iomt_backend::infra::jwt::JwtKeys;
use iomt_backend::ingest::{Ingest, MAX_BATCH_BYTES};
use iomt_backend::state::AppState;
use iomt_backend::{MIGRATOR, api, seed};
use iomt_protocol::telemetry_v1::telemetry_ingest_service_server::TelemetryIngestServiceServer;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(version, about = "IoMT cloud backend")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Serve the REST API and the gRPC ingest (default).
    Serve,
    /// Apply database migrations and exit.
    Migrate,
    /// Create development accounts and assign wearables to the seeded patients.
    SeedDev {
        /// Wearable id to register, 32 lowercase hex characters; repeatable.
        #[arg(long = "device-id")]
        device_ids: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();
    let cli = Cli::parse();
    let cfg = Config::from_env()?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await
        .context("cannot connect to the database")?;
    MIGRATOR.run(&db).await.context("migrations failed")?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Migrate => {
            tracing::info!("migrations applied");
            Ok(())
        }
        Command::SeedDev { device_ids } => {
            seed::seed_dev(&db, &device_ids).await?;
            println!(
                "seeded: admin@iomt.local, bs.minh@iomt.local and {} patients; password {}; {} wearables assigned",
                seed::DEV_PATIENTS.len(),
                seed::DEV_PASSWORD,
                device_ids.len()
            );
            Ok(())
        }
        Command::Serve => serve(cfg, db).await,
    }
}

async fn serve(cfg: Config, db: sqlx::PgPool) -> anyhow::Result<()> {
    if cfg.jwt_key_pkcs8_b64.is_none() {
        tracing::warn!(
            "JWT_ED25519_PKCS8_B64 not set: using a throwaway signing key; sessions end on restart"
        );
    }
    let jwt = JwtKeys::load_or_generate(cfg.jwt_key_pkcs8_b64.as_deref())?;
    let state = AppState::new(db.clone(), jwt, cfg.tenant_id)
        .map_err(|_| anyhow::anyhow!("cannot prepare password hashing"))?;

    let engine = state.engine.clone();
    tokio::spawn(async move { engine.run_periodic(db, Duration::from_secs(30)).await });

    let grpc = tonic::transport::Server::builder()
        .add_service(
            TelemetryIngestServiceServer::new(Ingest::new(state.clone()))
                .max_decoding_message_size(MAX_BATCH_BYTES),
        )
        .serve_with_shutdown(cfg.grpc_addr, shutdown());
    tracing::info!(addr = %cfg.grpc_addr, "gRPC ingest listening");

    let app = api::router(state, cfg.cors_origin.as_deref());
    let listener = tokio::net::TcpListener::bind(cfg.http_addr)
        .await
        .with_context(|| format!("cannot bind {}", cfg.http_addr))?;
    tracing::info!(addr = %cfg.http_addr, "REST API listening");
    let http = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown());

    let (h, g) = tokio::join!(http, grpc);
    h.context("REST server failed")?;
    g.context("gRPC server failed")?;
    Ok(())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
