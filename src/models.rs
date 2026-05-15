use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Inbound event ────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct ManualScoreRequest {
    pub resume_id: Uuid,
    pub job_id: Uuid,
    pub user_id: Uuid,
}

// ── Internal pipeline types ──────────────────────────────────────

pub struct ScoringInput {
    pub resume_id: Uuid,
    pub job_id: Uuid,
    pub user_id: Uuid,
    pub resume_sections: DocumentSections,
    pub job_sections: DocumentSections,
    pub resume_vectors: SectionVectors,
    pub job_vectors: SectionVectors,
    pub resume_texts: SectionTexts,
    pub job_texts: SectionTexts,
    pub parse_markers: ParseMarkers,
}

#[derive(Debug, Clone)]
pub struct DocumentSections {
    pub full_text: String,
    pub skills: Option<String>,
    pub experience_or_requirements: Option<String>,
    pub education: Option<String>, // resume only
}

#[derive(Debug, Clone)]
pub struct SectionVectors {
    pub full_vec: Vec<f32>,
    pub skills_vec: Option<Vec<f32>>,
    pub experience_vec: Option<Vec<f32>>,
    pub education_vec: Option<Vec<f32>>,   // resume only
    pub requirements_vec: Option<Vec<f32>>, // job only
}

#[derive(Debug, Clone, Default)]
pub struct SectionTexts {
    pub full_text: String,
    pub skills_text: Option<String>,
    pub experience_text: Option<String>,
    pub education_text: Option<String>,    // resume only
    pub requirements_text: Option<String>, // job only
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParseMarkers {
    pub fallback_used: bool,
    pub ocr_used: bool,
    pub partial_parse: bool,
    pub layout_detected: String,
    pub warnings: Vec<String>,
}

// ── Pipeline output (maps 1:1 to the breakdown JSON schema) ─────

#[derive(Debug, Clone, Serialize)]
pub struct ScoreResult {
    pub resume_id: Uuid,
    pub job_id: Uuid,
    pub user_id: Uuid,
    pub total_score: u8,
    pub breakdown: Breakdown,
}

#[derive(Debug, Clone, Serialize)]
pub struct Breakdown {
    pub keyword_match_rate: KeywordMatchRate,
    pub skill_alignment: SkillAlignment,
    pub experience_relevance: ExperienceRelevance,
    pub format_and_parseability: FormatParseability,
}

impl Breakdown {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KeywordMatchRate {
    pub earned: u8,
    pub max: u8, // always 40
    pub details: KeywordDetails,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeywordDetails {
    pub matched: Vec<String>,
    pub partial: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillAlignment {
    pub earned: u8,
    pub max: u8, // always 25
    pub details: Vec<SkillAlignmentItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillAlignmentItem {
    pub required_skill: String,
    pub closest_match: String,
    pub vector_similarity_score: f32,
    pub flag: AlignmentFlag,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperienceRelevance {
    pub earned: u8,
    pub max: u8, // always 15
    pub details: Vec<ExperienceRelevanceItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperienceRelevanceItem {
    pub job_responsibility: String,
    pub closest_match: String,
    pub vector_similarity_score: f32,
    pub flag: AlignmentFlag,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum AlignmentFlag {
    Good,
    NeedsReframe,
    MissingMetrics,
    Weak,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormatParseability {
    pub earned: u8,
    pub max: u8, // always 20
    pub parsing_flags: ParseMarkers,
}

// ── Outbound event ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AtsScoreReadyEvent {
    pub ats_score_id: Uuid,
    pub resume_id: Uuid,
    pub job_id: Uuid,
    pub user_id: Uuid,
    pub total_score: u8,
}
