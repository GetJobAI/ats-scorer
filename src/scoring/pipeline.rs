use anyhow::{Result, anyhow};
use fastembed::TextRerank;

use crate::models::{Breakdown, ScoreResult, ScoringInput};
use crate::scoring::{biencoder, parseability, reranker, tfidf};

pub async fn run_pipeline(
    reranker_model: &std::sync::Mutex<TextRerank>,
    input: ScoringInput,
) -> Result<ScoreResult> {
    let (tfidf_earned, keyword_match_rate) =
        tfidf::tfidf_score(&input.resume_sections, &input.job_sections);

    let pairs = biencoder::biencoder_pairs(
        &input.resume_vectors,
        &input.job_vectors,
        &input.resume_texts,
        &input.job_texts,
    );

    let ranked = {
        let mut model = reranker_model.lock().map_err(|_| anyhow!("reranker mutex poisoned"))?;
        reranker::rerank(&mut model, pairs)?
    };

    let skill_alignment = reranker::skill_alignment_score(&ranked);
    let experience_relevance = reranker::experience_relevance_score(&ranked);

    let format_and_parseability = parseability::parseability_score(&input.parse_markers);

    let total_score = tfidf_earned
        + skill_alignment.earned
        + experience_relevance.earned
        + format_and_parseability.earned;

    Ok(ScoreResult {
        resume_id: input.resume_id,
        job_id: input.job_id,
        user_id: input.user_id,
        total_score,
        breakdown: Breakdown {
            keyword_match_rate,
            skill_alignment,
            experience_relevance,
            format_and_parseability,
        },
    })
}
