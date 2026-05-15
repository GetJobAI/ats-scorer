mod cli;
mod config;
mod db;
mod handlers;
mod models;
mod queue;
mod scoring;
mod vector_store;

use anyhow::{Context, Result};
use axum::{routing::get, Router};
use clap::Parser;
use fastembed::TextRerank;
use lapin::{Connection, ConnectionProperties};
use qdrant_client::Qdrant;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::signal;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{error, info};

use crate::{
    cli::{Cli, Command},
    config::Config,
};

pub struct AppContext {
    pub db_pool: sqlx::PgPool,
    pub qdrant_client: Qdrant,
    pub rabbitmq_channel: lapin::Channel,
    pub reranker: TextRerank,
    pub config: Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve => run_serve().await?,
        Command::DownloadModels => {
            info!("Initializing TextRerank to trigger ONNX downloads...");
            let _ = TextRerank::try_new(
                fastembed::RerankInitOptions::new(fastembed::RerankerModel::BGERerankerV2M3)
                    .with_show_download_progress(true),
            )
            .context("Failed to initialize and download TextRerank models")?;
            info!("Models downloaded successfully.");
        }
        Command::Score { resume_id, job_id } => {
            run_single_score(resume_id, job_id).await?;
        }
    }

    Ok(())
}

async fn run_serve() -> Result<()> {
    let config = Config::load()?;

    info!("Connecting to PostgreSQL...");
    let retry_strategy = ExponentialBackoff::from_millis(100).take(5);
    let db_pool = Retry::spawn(retry_strategy.clone(), || {
        PgPoolOptions::new().connect(&config.postgres_url)
    })
    .await
    .context("Failed to connect to PostgreSQL")?;

    info!("Connecting to Qdrant...");
    let qdrant_client = Qdrant::from_url(&config.qdrant_url).build()?;
    Retry::spawn(retry_strategy.clone(), || async {
        qdrant_client.health_check().await
    })
    .await
    .context("Failed to connect to Qdrant")?;

    info!("Connecting to RabbitMQ...");
    let rmq_conn = Retry::spawn(retry_strategy.clone(), || {
        Connection::connect(&config.rabbitmq_url, ConnectionProperties::default())
    })
    .await
    .context("Failed to connect to RabbitMQ")?;

    let rabbitmq_channel = rmq_conn.create_channel().await?;

    info!("Initializing Reranker Model...");
    let reranker = TextRerank::try_new(fastembed::RerankInitOptions::new(fastembed::RerankerModel::BGERerankerV2M3))
    .context("Failed to load TextRerank model")?;

    let app_context = Arc::new(AppContext {
        db_pool,
        qdrant_client,
        rabbitmq_channel: rabbitmq_channel.clone(),
        reranker,
        config,
    });

    let queue_name = app_context.config.rabbitmq_consume_queue.clone();

    info!("Starting consumer...");
    let consumer_task = tokio::spawn(async move {
        if let Err(e) =
            queue::consumer::start_consumer(rabbitmq_channel, &queue_name, app_context).await
        {
            error!("Consumer error: {}", e);
        }
    });

    let health_app = Router::new().route("/healthz", get(healthz));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Starting health server on 0.0.0.0:8080");

    let health_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, health_app).await {
            error!("Health server error: {}", e);
        }
    });

    signal::ctrl_c().await?;
    info!("Shutting down...");

    consumer_task.abort();
    health_task.abort();

    Ok(())
}

async fn healthz() -> &'static str {
    // Basic health check
    "OK"
}

async fn run_single_score(resume_id: uuid::Uuid, job_id: uuid::Uuid) -> Result<()> {
    info!("Scoring resume_id: {}, job_id: {} (Manual trigger via CLI)", resume_id, job_id);
    let config = Config::load()?;

    let db_pool = PgPoolOptions::new()
        .connect(&config.postgres_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    let qdrant_client = Qdrant::from_url(&config.qdrant_url).build()?;

    let rmq_conn = Connection::connect(&config.rabbitmq_url, ConnectionProperties::default())
        .await
        .context("Failed to connect to RabbitMQ")?;
    let rabbitmq_channel = rmq_conn.create_channel().await?;

    let reranker = TextRerank::try_new(fastembed::RerankInitOptions::new(fastembed::RerankerModel::BGERerankerV2M3))?;

    let ctx = AppContext {
        db_pool,
        qdrant_client,
        rabbitmq_channel,
        reranker,
        config,
    };

    let req = models::ManualScoreRequest {
        resume_id,
        job_id,
        user_id: uuid::Uuid::nil(), // dummy user id for CLI trigger
    };

    handlers::manual::handle_manual(&ctx, req).await?;

    info!("Scoring complete.");
    Ok(())
}
