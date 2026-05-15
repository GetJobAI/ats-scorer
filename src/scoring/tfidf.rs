use rust_stemmers::{Algorithm, Stemmer};
use std::collections::HashSet;
use stop_words::{LANGUAGE, get};

use crate::models::{DocumentSections, KeywordDetails, KeywordMatchRate};

pub fn tfidf_score(resume: &DocumentSections, job: &DocumentSections) -> (u8, KeywordMatchRate) {
    let stemmer = Stemmer::create(Algorithm::English);
    let stop_words: HashSet<String> = get(LANGUAGE::English).into_iter().collect();

    let tokenize_and_stem = |text: &str| -> HashSet<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|s| {
                // Remove basic punctuation
                s.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|s| !s.is_empty() && !stop_words.contains(s))
            .map(|s| stemmer.stem(&s).into_owned())
            .collect()
    };

    let resume_tokens = tokenize_and_stem(&resume.full_text);

    // We treat job skills and requirements as the target keywords
    let job_text = format!(
        "{} {}",
        job.skills.as_deref().unwrap_or(""),
        job.experience_or_requirements.as_deref().unwrap_or("")
    );
    let job_tokens = tokenize_and_stem(&job_text);

    let mut matched = Vec::new();
    let mut missing = Vec::new();
    let partial = Vec::new(); // Simplified: we only do exact stem matches for now

    for token in &job_tokens {
        if resume_tokens.contains(token) {
            matched.push(token.clone());
        } else {
            missing.push(token.clone());
        }
    }

    let total_keywords = job_tokens.len();
    if total_keywords == 0 {
        return (
            40,
            KeywordMatchRate {
                earned: 40,
                max: 40,
                details: KeywordDetails {
                    matched: vec![],
                    partial: vec![],
                    missing: vec![],
                },
            },
        );
    }

    let earned = ((matched.len() as f32 / total_keywords as f32) * 40.0).round() as u8;

    (
        earned,
        KeywordMatchRate {
            earned,
            max: 40,
            details: KeywordDetails {
                matched,
                partial,
                missing,
            },
        },
    )
}
