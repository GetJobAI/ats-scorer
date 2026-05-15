use crate::models::{FormatParseability, ParseMarkers};

pub fn parseability_score(markers: &ParseMarkers) -> FormatParseability {
    let mut score: i16 = 20;

    if markers.fallback_used {
        score -= 5;
    }
    if markers.ocr_used {
        score -= 3;
    }
    if markers.partial_parse {
        score -= 7;
    }
    if markers.layout_detected != "single_column" {
        score -= 5;
    }
    
    let warning_penalty = (markers.warnings.len() as i16 * 2).min(6);
    score -= warning_penalty;

    let earned = score.max(0) as u8;

    FormatParseability {
        earned,
        max: 20,
        parsing_flags: markers.clone(),
    }
}
