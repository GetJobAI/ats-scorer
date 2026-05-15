use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{DocumentSections, ParseMarkers};

pub async fn fetch_resume_sections(pool: &PgPool, resume_id: Uuid) -> Result<DocumentSections> {
    let record = sqlx::query(
        r#"
        SELECT full_text, skills, experience, education
        FROM resume_sections
        WHERE resume_id = $1
        "#
    )
    .bind(resume_id)
    .fetch_one(pool)
    .await?;

    Ok(DocumentSections {
        full_text: record.get("full_text"),
        skills: record.get("skills"),
        experience_or_requirements: record.get("experience"),
        education: record.get("education"),
    })
}

pub async fn fetch_job_sections(pool: &PgPool, job_id: Uuid) -> Result<DocumentSections> {
    let record = sqlx::query(
        r#"
        SELECT full_text, skills, requirements
        FROM job_analysis_sections
        WHERE job_id = $1
        "#
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;

    Ok(DocumentSections {
        full_text: record.get("full_text"),
        skills: record.get("skills"),
        experience_or_requirements: record.get("requirements"),
        education: None, // jobs don't have education
    })
}

pub async fn fetch_parse_markers(pool: &PgPool, resume_id: Uuid) -> Result<ParseMarkers> {
    let record = sqlx::query(
        r#"
        SELECT has_complex_layout, has_graphics, has_headers_footers, has_non_standard_fonts
        FROM resume_parse_markers
        WHERE resume_id = $1
        "#
    )
    .bind(resume_id)
    .fetch_one(pool)
    .await?;

    Ok(ParseMarkers {
        has_complex_layout: record.get("has_complex_layout"),
        has_graphics: record.get("has_graphics"),
        has_headers_footers: record.get("has_headers_footers"),
        has_non_standard_fonts: record.get("has_non_standard_fonts"),
    })
}
