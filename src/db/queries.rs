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

pub async fn fetch_job_metadata(pool: &PgPool, job_id: Uuid) -> Result<(String, String)> {
    let record = sqlx::query(
        "SELECT content->>'title' AS title, content->>'company' AS company FROM job_postings WHERE id = $1"
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;

    let title: Option<String> = record.get("title");
    let company: Option<String> = record.get("company");

    Ok((title.unwrap_or_default(), company.unwrap_or_default()))
}

pub async fn fetch_parse_markers(pool: &PgPool, resume_id: Uuid) -> Result<ParseMarkers> {
    let record = sqlx::query("SELECT content->'meta' as meta FROM resumes WHERE id = $1")
        .bind(resume_id)
        .fetch_one(pool)
        .await?;

    let meta: Option<serde_json::Value> = record.get("meta");
    let meta = meta.unwrap_or_else(|| serde_json::json!({}));

    let bool_flag = |key: &str| meta.get(key).and_then(|v| v.as_bool()).unwrap_or(false);

    Ok(ParseMarkers {
        has_complex_layout: bool_flag("has_complex_layout"),
        has_graphics: bool_flag("has_graphics"),
        has_headers_footers: bool_flag("has_headers_footers"),
        has_non_standard_fonts: bool_flag("has_non_standard_fonts"),
    })
}
