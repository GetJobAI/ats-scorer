use anyhow::{Context, Result};
use qdrant_client::{
    Qdrant,
    qdrant::{Condition, Filter},
};
use uuid::Uuid;

use crate::models::{SectionTexts, SectionVectors};

pub async fn fetch_section_vectors(
    client: &Qdrant,
    collection_name: &str,
    source_id: Uuid,
) -> Result<Option<(SectionVectors, SectionTexts)>> {
    let filter = Filter::must([Condition::matches("source_id", source_id.to_string())]);

    let response = client
        .scroll(qdrant_client::qdrant::ScrollPoints {
            collection_name: collection_name.to_string(),
            filter: Some(filter),
            with_vectors: Some(true.into()),
            with_payload: Some(true.into()),
            limit: Some(10),
            ..Default::default()
        })
        .await
        .context("Failed to scroll points in Qdrant")?;

    if response.result.is_empty() {
        return Ok(None);
    }

    let mut vectors = SectionVectors {
        full_vec: Vec::new(),
        skills_vec: None,
        experience_vec: None,
        education_vec: None,
        requirements_vec: None,
    };
    let mut texts = SectionTexts::default();

    let mut has_full_vec = false;

    for point in response.result {
        let payload = point.payload;

        let section_type = payload.get("section_type").and_then(|v| {
            if let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = &v.kind {
                Some(s.as_str())
            } else {
                None
            }
        });

        let text = payload
            .get("text")
            .and_then(|v| {
                if let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = &v.kind {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let vector = match point.vectors {
            Some(v) => {
                if let Some(qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v)) =
                    v.vectors_options
                {
                    v.data
                } else {
                    continue;
                }
            }
            None => continue,
        };

        match section_type {
            Some("resume_full") | Some("job_full") => {
                vectors.full_vec = vector;
                texts.full_text = text;
                has_full_vec = true;
            }
            Some("resume_skills") | Some("job_skills") => {
                vectors.skills_vec = Some(vector);
                texts.skills_text = Some(text);
            }
            Some("resume_experience") => {
                vectors.experience_vec = Some(vector);
                texts.experience_text = Some(text);
            }
            Some("resume_education") => {
                vectors.education_vec = Some(vector);
                texts.education_text = Some(text);
            }
            Some("job_requirements") => {
                vectors.requirements_vec = Some(vector);
                texts.requirements_text = Some(text);
            }
            _ => {}
        }
    }

    if !has_full_vec {
        return Ok(None);
    }

    Ok(Some((vectors, texts)))
}
