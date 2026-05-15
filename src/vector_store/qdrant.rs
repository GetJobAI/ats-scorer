use anyhow::{Context, Result};
use qdrant_client::{
    qdrant::{Condition, Filter},
    Qdrant,
};
use uuid::Uuid;

use crate::models::SectionVectors;

pub async fn fetch_section_vectors(
    client: &Qdrant,
    collection_name: &str,
    source_id: Uuid,
) -> Result<Option<SectionVectors>> {
    let filter = Filter::must([Condition::matches(
        "source_id",
        source_id.to_string(),
    )]);

    // Query points by source_id. We fetch them via scroll or search without vector.
    let response = client
        .scroll(
            qdrant_client::qdrant::ScrollPoints {
                collection_name: collection_name.to_string(),
                filter: Some(filter),
                with_vectors: Some(true.into()),
                with_payload: Some(true.into()),
                limit: Some(10), // Should only be up to 4-5 per source_id
                ..Default::default()
            },
        )
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

    let mut has_full_vec = false;

    for point in response.result {
        let payload = point.payload;
        
        let section_type_value = payload.get("section_type").and_then(|v| {
            if let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = &v.kind {
                Some(s.as_str())
            } else {
                None
            }
        });

        let vector = match point.vectors {
            Some(v) => {
                if let Some(qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v)) = v.vectors_options {
                    v.data
                } else {
                    continue;
                }
            },
            None => continue,
        };

        match section_type_value {
            Some("resume_full") | Some("job_full") => {
                vectors.full_vec = vector;
                has_full_vec = true;
            }
            Some("resume_skills") | Some("job_skills") => vectors.skills_vec = Some(vector),
            Some("resume_experience") => vectors.experience_vec = Some(vector),
            Some("resume_education") => vectors.education_vec = Some(vector),
            Some("job_requirements") => vectors.requirements_vec = Some(vector),
            _ => {}
        }
    }

    if !has_full_vec {
        // Technically this shouldn't happen if we have *any* vectors, but safety first
        return Ok(None);
    }

    Ok(Some(vectors))
}
