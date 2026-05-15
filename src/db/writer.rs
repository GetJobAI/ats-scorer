use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::ScoreResult;

pub async fn upsert_ats_score(pool: &PgPool, result: &ScoreResult) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let breakdown_json = result.breakdown.to_json();

    let record = sqlx::query(
        r#"
        INSERT INTO ats_scores (id, resume_id, job_analysis_id, score, breakdown)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (resume_id, job_analysis_id)
        DO UPDATE SET score = EXCLUDED.score, breakdown = EXCLUDED.breakdown
        RETURNING id
        "#
    )
    .bind(id)
    .bind(result.resume_id)
    .bind(result.job_id)
    .bind(result.total_score as i16)
    .bind(breakdown_json)
    .fetch_one(pool)
    .await?;

    Ok(record.get("id"))
}
