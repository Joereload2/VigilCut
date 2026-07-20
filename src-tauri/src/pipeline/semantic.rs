//! Deterministic semantic event extraction from a transcript (no LLM).

use crate::models::transcript::Transcript;
use crate::models::visual::{SemanticEvent, SemanticKind};
use crate::pipeline::time_map::TimeMap;

const STOP_ES: &[&str] = &[
    "el", "la", "los", "las", "un", "una", "unos", "unas", "de", "del", "al", "a", "en", "y", "o",
    "que", "se", "es", "son", "por", "para", "con", "sin", "como", "más", "muy", "ya", "lo", "su",
    "sus", "me", "te", "nos", "les", "le", "mi", "tu", "si", "no", "esto", "esta", "ese", "esa",
    "hay", "fue", "ser", "está", "están", "porque", "cuando", "donde", "qué", "cuál",
];

/// Extract keyword/phrase/concept events; attach output spans via TimeMap.
pub fn extract_semantic_events(
    transcript: &Transcript,
    run_id: &str,
    time_map: &TimeMap,
) -> Vec<SemanticEvent> {
    let mut events = Vec::new();
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for seg in &transcript.segments {
        for tok in tokenize(&seg.text) {
            *freq.entry(tok).or_default() += 1;
        }
    }

    for seg in &transcript.segments {
        let tokens = tokenize(&seg.text);
        if tokens.is_empty() {
            continue;
        }

        // Keywords with controlled frequency
        for t in &tokens {
            let n = *freq.get(t).unwrap_or(&0);
            if n == 0 || n > 12 {
                continue;
            }
            if t.chars().count() < 4 {
                continue;
            }
            let score = (0.35 + (n as f64) * 0.08).min(0.85);
            if let Some(out) = time_map.primary_output_span(seg.span) {
                events.push(SemanticEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    run_id: run_id.into(),
                    kind: SemanticKind::Keyword,
                    source_span: seg.span,
                    output_span: Some(out),
                    label: t.clone(),
                    terms: vec![t.clone()],
                    score,
                    transcript_segment_ids: vec![seg.id.clone()],
                    method: "freq_keyword_es".into(),
                    payload: serde_json::json!({ "freq": n }),
                });
            }
        }

        // Bigrams as phrases
        for w in tokens.windows(2) {
            let phrase = format!("{} {}", w[0], w[1]);
            if phrase.chars().count() < 8 {
                continue;
            }
            if let Some(out) = time_map.primary_output_span(seg.span) {
                events.push(SemanticEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    run_id: run_id.into(),
                    kind: SemanticKind::Phrase,
                    source_span: seg.span,
                    output_span: Some(out),
                    label: phrase.clone(),
                    terms: w.to_vec(),
                    score: 0.55,
                    transcript_segment_ids: vec![seg.id.clone()],
                    method: "bigram_es".into(),
                    payload: serde_json::json!({}),
                });
            }
        }

        // Concept dictionary (domain seeds)
        for (concept, syns) in CONCEPT_SEED {
            let lower = seg.text.to_lowercase();
            if syns.iter().any(|s| lower.contains(s)) {
                if let Some(out) = time_map.primary_output_span(seg.span) {
                    events.push(SemanticEvent {
                        id: uuid::Uuid::new_v4().to_string(),
                        run_id: run_id.into(),
                        kind: SemanticKind::Concept,
                        source_span: seg.span,
                        output_span: Some(out),
                        label: (*concept).into(),
                        terms: syns.iter().map(|s| (*s).into()).collect(),
                        score: 0.78,
                        transcript_segment_ids: vec![seg.id.clone()],
                        method: "concept_dict".into(),
                        payload: serde_json::json!({ "concept": concept }),
                    });
                }
            }
        }
    }

    // Dedupe by label+approx time
    events.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut seen = std::collections::HashSet::new();
    events.retain(|e| {
        let key = format!(
            "{}:{:.1}",
            e.label.to_lowercase(),
            e.source_span.start.floor()
        );
        seen.insert(key)
    });
    events.truncate(80);
    events
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != 'á' && c != 'é' && c != 'í' && c != 'ó' && c != 'ú' && c != 'ñ')
        .map(|s| s.trim())
        .filter(|s| s.chars().count() >= 3)
        .filter(|s| !STOP_ES.contains(s))
        .map(|s| s.to_string())
        .collect()
}

const CONCEPT_SEED: &[(&str, &[&str])] = &[
    ("inflacion", &["inflación", "inflacion", "precios", "ipc"]),
    ("alimentos", &["alimentos", "comida", "mercado", "canasta"]),
    ("economia", &["economía", "economia", "mercados", "pib"]),
    ("tecnologia", &["tecnología", "tecnologia", "software", "app", "ia"]),
    ("salud", &["salud", "médico", "medico", "hospital"]),
    ("educacion", &["educación", "educacion", "escuela", "universidad"]),
    ("trabajo", &["trabajo", "empleo", "oficina", "empresa"]),
    ("viaje", &["viaje", "turismo", "aeropuerto", "hotel"]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::Span;
    use crate::models::transcript::{Transcript, TranscriptSegment, TranscriptStatus};

    #[test]
    fn extracts_concept_inflacion() {
        let mut tr = Transcript::new("x.mp4", "test");
        tr.status = TranscriptStatus::Ready;
        tr.segments.push(TranscriptSegment::new(
            Span::new(1.0, 5.0),
            "La inflación aumenta el costo de los alimentos en el mercado",
        ));
        let map = TimeMap::identity(60.0);
        let ev = extract_semantic_events(&tr, "run1", &map);
        assert!(ev.iter().any(|e| e.label.contains("inflacion") || e.terms.iter().any(|t| t.contains("inflación") || t.contains("alimentos"))));
    }
}
