use anyhow::Result;
use lapin::{
    Channel, ExchangeKind,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::FieldTable,
};
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

use crate::handlers::manual::handle_manual;
use crate::models::ManualScoreRequest;
use crate::AppContext;

pub async fn start_consumer(
    channel: Channel,
    exchange_name: &str,
    queue_name: &str,
    routing_key: &str,
    ctx: Arc<AppContext>,
) -> Result<()> {
    channel
        .exchange_declare(
            exchange_name.into(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            queue_name.into(),
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            queue_name.into(),
            exchange_name.into(),
            routing_key.into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            queue_name.into(),
            "ats_scorer_consumer".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!(
        exchange = exchange_name,
        queue = queue_name,
        routing_key = routing_key,
        "Started consumer"
    );

    while let Some(delivery) = consumer.next().await {
        let delivery = match delivery {
            Ok(d) => d,
            Err(e) => {
                error!("Error receiving delivery: {}", e);
                continue;
            }
        };

        let payload = match std::str::from_utf8(&delivery.data) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to parse delivery data as UTF-8: {}", e);
                let _ = delivery
                    .nack(BasicNackOptions {
                        requeue: false,
                        ..Default::default()
                    })
                    .await;
                continue;
            }
        };

        let request: ManualScoreRequest = match serde_json::from_str(payload) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to deserialize request: {}", e);
                let _ = delivery
                    .nack(BasicNackOptions {
                        requeue: false,
                        ..Default::default()
                    })
                    .await;
                continue;
            }
        };

        match handle_manual(&ctx, request).await {
            Ok(_) => {
                if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                    error!("Failed to ack message: {}", e);
                }
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("Vectors not ready") {
                    warn!(error = %e, "Vectors not ready, requeueing after delay");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let _ = delivery
                        .nack(BasicNackOptions {
                            requeue: true,
                            ..Default::default()
                        })
                        .await;
                } else {
                    error!("Handler failed, requeuing: {}", e);
                    let _ = delivery
                        .nack(BasicNackOptions {
                            requeue: true,
                            ..Default::default()
                        })
                        .await;
                }
            }
        }
    }

    Ok(())
}
