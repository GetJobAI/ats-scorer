use anyhow::Result;
use lapin::{
    options::BasicPublishOptions, BasicProperties, Channel,
};

use crate::models::AtsScoreReadyEvent;

pub async fn publish_score_ready(
    channel: &Channel,
    exchange: &str,
    routing_key: &str,
    event: AtsScoreReadyEvent,
) -> Result<()> {
    let payload = serde_json::to_string(&event)?;

    channel
        .basic_publish(
            exchange.into(),
            routing_key.into(),
            BasicPublishOptions::default(),
            payload.as_bytes(),
            BasicProperties::default()
                .with_delivery_mode(2) // Persistent
                .with_content_type("application/json".into()),
        )
        .await?;

    Ok(())
}
