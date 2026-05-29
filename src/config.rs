use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub postgres_url: String,

    pub qdrant_url: String,
    pub qdrant_collection: String,
    pub qdrant_api_key: String,

    pub rabbitmq_url: String,
    pub rabbitmq_consume_exchange: String,
    pub rabbitmq_consume_routing_key: String,
    pub rabbitmq_consume_queue: String,
    pub rabbitmq_publish_exchange: String,

    pub tfidf_top_k: usize,
    pub reranker_top_k: usize,

    pub embedding_batch_size: usize,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        envy::from_env::<Config>()
            .map_err(|e| anyhow::anyhow!("Failed to load configuration from environment: {}", e))
    }
}
