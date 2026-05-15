use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{DocumentSections, ParseMarkers};

fn extract_all_text(value: &serde_json::Value) -> String {
    let mut texts = Vec::new();
    match value {
        serde_json::Value::String(s) => texts.push(s.clone()),
        serde_json::Value::Array(arr) => {
            for item in arr {
                let t = extract_all_text(item);
                if !t.is_empty() {
                    texts.push(t);
                }
            }
        }
        serde_json::Value::Object(obj) => {
            for v in obj.values() {
                let t = extract_all_text(v);
                if !t.is_empty() {
                    texts.push(t);
                }
            }
        }
        _ => {}
    }
    texts.join("\n")
}

pub async fn fetch_resume_sections(pool: &PgPool, resume_id: Uuid) -> Result<DocumentSections> {
    let record = sqlx::query("SELECT content FROM resumes WHERE id = $1")
        .bind(resume_id)
        .fetch_one(pool)
        .await?;

    let content: serde_json::Value = record.get("content");

    let full_text = extract_all_text(&content);

    let skills = content
        .get("skills")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());
    let experience_or_requirements = content
        .get("experience")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());
    let education = content
        .get("education")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());

    Ok(DocumentSections {
        full_text,
        skills,
        experience_or_requirements,
        education,
    })
}

pub async fn fetch_job_sections(pool: &PgPool, job_id: Uuid) -> Result<DocumentSections> {
    let record = sqlx::query("SELECT content FROM job_postings WHERE id = $1")
        .bind(job_id)
        .fetch_one(pool)
        .await?;

    let content: serde_json::Value = record.get("content");

    let full_text = extract_all_text(&content);

    let skills = content
        .get("skills")
        .map(extract_all_text)
        .filter(|s| !s.is_empty());
    let experience_or_requirements = content
        .get("requirements")
        .or_else(|| content.get("experience"))
        .map(extract_all_text)
        .filter(|s| !s.is_empty());

    Ok(DocumentSections {
        full_text,
        skills,
        experience_or_requirements,
        education: None,
    })
}

pub async fn fetch_parse_markers(pool: &PgPool, resume_id: Uuid) -> Result<ParseMarkers> {
    let record = sqlx::query("SELECT content->'meta' as meta FROM resumes WHERE id = $1")
        .bind(resume_id)
        .fetch_one(pool)
        .await?;

    let meta: Option<serde_json::Value> = record.get("meta");
    let meta = meta.unwrap_or_else(|| serde_json::json!({}));

    let fallback_used = meta
        .get("fallback_used")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ocr_used = meta
        .get("ocr_used")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let partial_parse = meta
        .get("partial_parse")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let layout_detected = meta
        .get("layout_detected")
        .and_then(|v| v.as_str())
        .unwrap_or("single_column")
        .to_string();

    let warnings = meta
        .get("warnings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|w| w.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(ParseMarkers {
        fallback_used,
        ocr_used,
        partial_parse,
        layout_detected,
        warnings,
    })
}
