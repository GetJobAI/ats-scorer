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
            .map(|s| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
            .filter(|s| !s.is_empty() && !stop_words.contains(s))
            .map(|s| stemmer.stem(&s).into_owned())
            .collect()
    };

    let resume_tokens = tokenize_and_stem(&resume.full_text);

    let job_text = format!(
        "{} {}",
        job.skills.as_deref().unwrap_or(""),
        job.experience_or_requirements.as_deref().unwrap_or("")
    );
    let job_tokens = tokenize_and_stem(&job_text);

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

    let mut matched = Vec::new();
    let mut partial = Vec::new();
    let mut missing = Vec::new();

    for token in &job_tokens {
        if resume_tokens.contains(token) {
            matched.push(token.clone());
        } else if resume_tokens.iter().any(|r| bigram_jaccard(token, r) > 0.5) {
            partial.push(token.clone());
        } else {
            missing.push(token.clone());
        }
    }

    // matched = full weight, partial = half weight
    let weighted = matched.len() as f32 + partial.len() as f32 * 0.5;
    let earned = ((weighted / total_keywords as f32) * 40.0).round() as u8;

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

/// Bigram Jaccard similarity between two strings — respects character ordering.
/// Returns a value in [0.0, 1.0]; > 0.5 is treated as a partial stem match.
fn bigram_jaccard(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    let a_bigrams: HashSet<(char, char)> = a.chars().zip(a.chars().skip(1)).collect();
    let b_bigrams: HashSet<(char, char)> = b.chars().zip(b.chars().skip(1)).collect();
    if a_bigrams.is_empty() && b_bigrams.is_empty() {
        return 0.0;
    }
    let intersection = a_bigrams.intersection(&b_bigrams).count();
    let union = a_bigrams.union(&b_bigrams).count();
    intersection as f32 / union as f32
}
