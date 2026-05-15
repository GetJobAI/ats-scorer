use crate::models::{FormatParseability, ParseMarkers};

pub fn parseability_score(markers: &ParseMarkers) -> FormatParseability {
    let mut score: i16 = 20;

    if markers.has_complex_layout {
        score -= 5;
    }
    if markers.has_graphics {
        score -= 5;
    }
    if markers.has_headers_footers {
        score -= 3;
    }
    if markers.has_non_standard_fonts {
        score -= 7;
    }

    let earned = score.max(0) as u8;

    FormatParseability {
        earned,
        max: 20,
        parsing_flags: markers.clone(),
    }
}
