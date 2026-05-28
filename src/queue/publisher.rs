use anyhow::Result;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, ExchangeKind,
};

use crate::models::AtsScoreReadyEvent;

pub async fn publish_score_ready(
    channel: &Channel,
    exchange: &str,
    event: AtsScoreReadyEvent,
) -> Result<()> {
    channel
        .exchange_declare(
            exchange.into(),
            ExchangeKind::Fanout,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    let payload = serde_json::to_string(&event)?;

    channel
        .basic_publish(
            exchange.into(),
            "".into(),
            BasicPublishOptions::default(),
            payload.as_bytes(),
            BasicProperties::default()
                .with_delivery_mode(2)
                .with_content_type("application/json".into()),
        )
        .await?;

    Ok(())
}
