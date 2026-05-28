use anyhow::Result;

use crate::models::{AtsScoreReadyEvent, ManualScoreRequest, ScoringInput};
use crate::queue::publisher::publish_score_ready;
use crate::{db, scoring, vector_store, AppContext};

pub async fn handle_manual(ctx: &AppContext, req: ManualScoreRequest) -> Result<()> {
    let resume_sections = db::queries::fetch_resume_sections(&ctx.db_pool, req.resume_id).await?;
    let job_sections = db::queries::fetch_job_sections(&ctx.db_pool, req.job_id).await?;

    let (resume_vectors, resume_texts) = vector_store::qdrant::fetch_section_vectors(
        &ctx.qdrant_client,
        &ctx.config.qdrant_collection,
        req.resume_id,
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("Vectors not ready for resume_id {}", req.resume_id))?;

    let (job_vectors, job_texts) = vector_store::qdrant::fetch_section_vectors(
        &ctx.qdrant_client,
        &ctx.config.qdrant_collection,
        req.job_id,
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("Vectors not ready for job_id {}", req.job_id))?;

    let parse_markers = db::queries::fetch_parse_markers(&ctx.db_pool, req.resume_id).await?;

    let scoring_input = ScoringInput {
        resume_id: req.resume_id,
        job_id: req.job_id,
        user_id: req.user_id,
        resume_sections,
        job_sections,
        resume_vectors,
        job_vectors,
        resume_texts,
        job_texts,
        parse_markers,
    };

    let score_result = scoring::pipeline::run_pipeline(&ctx.reranker, scoring_input).await?;

    let (job_title, company_name) = db::queries::fetch_job_metadata(&ctx.db_pool, req.job_id).await?;

    db::writer::upsert_ats_score(&ctx.db_pool, &score_result).await?;

    publish_score_ready(
        &ctx.rabbitmq_channel,
        &ctx.config.rabbitmq_publish_exchange,
        AtsScoreReadyEvent {
            resume_id: req.resume_id,
            job_id: req.job_id,
            job_title,
            company_name,
            total_score: score_result.total_score,
            breakdown: score_result.breakdown,
        },
    )
    .await?;

    Ok(())
}
