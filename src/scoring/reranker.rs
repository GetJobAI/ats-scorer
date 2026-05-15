use anyhow::Result;
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

// In a full implementation, SectionPair would carry the actual text chunks.
// Here we mock the text chunks for simplicity to follow the model structure.
pub fn rerank(
    _reranker: &TextRerank,
    pairs: Vec<SectionPair>,
) -> Result<Vec<RankedPair>> {
    let mut ranked = Vec::new();

    // Mock reranking for now, in a real scenario we'd use _reranker.rerank()
    for pair in pairs {
        let sim = pair.cosine_similarity;
        // Mock sigmoid conversion of logits, here we just use the cosine similarity as if it's already 0..1
        let flag = match sim {
            s if s >= 0.75 => AlignmentFlag::Good,
            s if s >= 0.55 => AlignmentFlag::NeedsReframe,
            s if s >= 0.35 => AlignmentFlag::MissingMetrics,
            _ => AlignmentFlag::Weak,
        };

        ranked.push(RankedPair {
            required_text: "Mock required text".to_string(),
            closest_match_text: "Mock closest match".to_string(),
            similarity_score: sim,
            flag,
            kind: pair.kind,
        });
    }

    Ok(ranked)
}

pub fn skill_alignment_score(ranked: &[RankedPair]) -> SkillAlignment {
    let skill_pairs: Vec<_> = ranked.iter().filter(|p| matches!(p.kind, SectionKind::Skill)).collect();
    
    if skill_pairs.is_empty() {
        return SkillAlignment {
            earned: 0,
            max: 25,
            details: vec![],
        };
    }

    let avg_score: f32 = skill_pairs.iter().map(|p| p.similarity_score).sum::<f32>() / skill_pairs.len() as f32;
    let earned = (avg_score * 25.0).round() as u8;

    let details = skill_pairs.into_iter().map(|p| SkillAlignmentItem {
        required_skill: p.required_text.clone(),
        closest_match: p.closest_match_text.clone(),
        vector_similarity_score: p.similarity_score,
        flag: p.flag.clone(),
    }).collect();

    SkillAlignment {
        earned,
        max: 25,
        details,
    }
}

pub fn experience_relevance_score(ranked: &[RankedPair]) -> ExperienceRelevance {
    let exp_pairs: Vec<_> = ranked.iter().filter(|p| matches!(p.kind, SectionKind::Experience)).collect();
    
    if exp_pairs.is_empty() {
        return ExperienceRelevance {
            earned: 0,
            max: 15,
            details: vec![],
        };
    }

    let avg_score: f32 = exp_pairs.iter().map(|p| p.similarity_score).sum::<f32>() / exp_pairs.len() as f32;
    let earned = (avg_score * 15.0).round() as u8;

    let details = exp_pairs.into_iter().map(|p| ExperienceRelevanceItem {
        job_responsibility: p.required_text.clone(),
        closest_match: p.closest_match_text.clone(),
        vector_similarity_score: p.similarity_score,
        flag: p.flag.clone(),
    }).collect();

    ExperienceRelevance {
        earned,
        max: 15,
        details,
    }
}
