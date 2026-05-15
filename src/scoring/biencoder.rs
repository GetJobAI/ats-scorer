use crate::models::{SectionTexts, SectionVectors};

pub enum SectionKind {
    Skill,
    Experience,
}

pub struct SectionPair {
    pub kind: SectionKind,
    pub cosine_similarity: f32,
    pub resume_text: String,
    pub job_text: String,
}

pub fn biencoder_pairs(
    resume_vectors: &SectionVectors,
    job_vectors: &SectionVectors,
    resume_texts: &SectionTexts,
    job_texts: &SectionTexts,
) -> Vec<SectionPair> {
    let mut pairs = Vec::new();

    if let (Some(r_skills), Some(j_skills)) = (&resume_vectors.skills_vec, &job_vectors.skills_vec) {
        let sim = cosine_similarity(r_skills, j_skills);
        pairs.push(SectionPair {
            kind: SectionKind::Skill,
            cosine_similarity: sim,
            resume_text: resume_texts.skills_text.clone().unwrap_or_default(),
            job_text: job_texts.skills_text.clone().unwrap_or_default(),
        });
    }

    if let (Some(r_exp), Some(j_req)) = (&resume_vectors.experience_vec, &job_vectors.requirements_vec) {
        let sim = cosine_similarity(r_exp, j_req);
        pairs.push(SectionPair {
            kind: SectionKind::Experience,
            cosine_similarity: sim,
            resume_text: resume_texts.experience_text.clone().unwrap_or_default(),
            job_text: job_texts.requirements_text.clone().unwrap_or_default(),
        });
    }

    pairs.sort_by(|a, b| b.cosine_similarity.partial_cmp(&a.cosine_similarity).unwrap());

    pairs
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}
