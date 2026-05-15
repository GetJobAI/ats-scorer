use anyhow::{Context, Result};
use fastembed::TextRerank;

use crate::models::{
    AlignmentFlag, ExperienceRelevance, ExperienceRelevanceItem, SkillAlignment, SkillAlignmentItem,
};
use crate::scoring::biencoder::{SectionKind, SectionPair};

pub struct RankedPair {
    pub required_text: String,
    pub closest_match_text: String,
    pub similarity_score: f32,
    pub flag: AlignmentFlag,
    pub kind: SectionKind,
}

pub fn rerank(reranker: &mut TextRerank, pairs: Vec<SectionPair>) -> Result<Vec<RankedPair>> {
    let mut ranked = Vec::new();

    for pair in pairs {
        let score = if pair.job_text.is_empty() || pair.resume_text.is_empty() {
            // Fall back to cosine similarity if text wasn't stored in Qdrant yet
            pair.cosine_similarity
        } else {
            let results = reranker
                .rerank(&pair.job_text, vec![&pair.resume_text], false, None)
                .context("TextRerank inference failed")?;
            let logit = results.first().map(|r| r.score).unwrap_or(0.0) as f32;
            sigmoid(logit)
        };

        ranked.push(RankedPair {
            required_text: pair.job_text,
            closest_match_text: pair.resume_text,
            similarity_score: score,
            flag: alignment_flag(score),
            kind: pair.kind,
        });
    }

    Ok(ranked)
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn alignment_flag(sim: f32) -> AlignmentFlag {
    if sim >= 0.75 {
        AlignmentFlag::Good
    } else if sim >= 0.55 {
        AlignmentFlag::NeedsReframe
    } else if sim >= 0.35 {
        AlignmentFlag::MissingMetrics
    } else {
        AlignmentFlag::Weak
    }
}

pub fn skill_alignment_score(ranked: &[RankedPair]) -> SkillAlignment {
    let skill_pairs: Vec<_> = ranked
        .iter()
        .filter(|p| matches!(p.kind, SectionKind::Skill))
        .collect();

    if skill_pairs.is_empty() {
        return SkillAlignment {
            earned: 0,
            max: 25,
            details: vec![],
        };
    }

    let avg_score: f32 =
        skill_pairs.iter().map(|p| p.similarity_score).sum::<f32>() / skill_pairs.len() as f32;
    let earned = (avg_score * 25.0).round() as u8;

    let details = skill_pairs
        .into_iter()
        .map(|p| SkillAlignmentItem {
            required_skill: p.required_text.clone(),
            closest_match: p.closest_match_text.clone(),
            vector_similarity_score: p.similarity_score,
            flag: p.flag.clone(),
        })
        .collect();

    SkillAlignment {
        earned,
        max: 25,
        details,
    }
}

pub fn experience_relevance_score(ranked: &[RankedPair]) -> ExperienceRelevance {
    let exp_pairs: Vec<_> = ranked
        .iter()
        .filter(|p| matches!(p.kind, SectionKind::Experience))
        .collect();

    if exp_pairs.is_empty() {
        return ExperienceRelevance {
            earned: 0,
            max: 15,
            details: vec![],
        };
    }

    let avg_score: f32 =
        exp_pairs.iter().map(|p| p.similarity_score).sum::<f32>() / exp_pairs.len() as f32;
    let earned = (avg_score * 15.0).round() as u8;

    let details = exp_pairs
        .into_iter()
        .map(|p| ExperienceRelevanceItem {
            job_responsibility: p.required_text.clone(),
            closest_match: p.closest_match_text.clone(),
            vector_similarity_score: p.similarity_score,
            flag: p.flag.clone(),
        })
        .collect();

    ExperienceRelevance {
        earned,
        max: 15,
        details,
    }
}
