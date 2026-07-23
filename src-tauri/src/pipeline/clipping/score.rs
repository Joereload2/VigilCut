//! Explainable clip scoring (not fake viral probability).

use crate::models::clipping::{ClipReason, ClipScoreBreakdown, SemanticUnit};

pub struct ScoredText {
    pub breakdown: ClipScoreBreakdown,
    pub score: f64,
    pub confidence: f64,
    pub reasons: Vec<ClipReason>,
    pub strengths: Vec<String>,
    pub risks: Vec<String>,
    pub title: String,
    pub summary: String,
}

pub fn score_unit(
    unit: &SemanticUnit,
    duration: f64,
    ideal: f64,
    min_d: f64,
    max_d: f64,
    has_real_transcript: bool,
) -> ScoredText {
    let text = unit.text.trim();
    let words: Vec<&str> = text.split_whitespace().collect();
    let n_words = words.len();
    let lower = text.to_lowercase();

    let mut b = ClipScoreBreakdown::default();
    let mut strengths = Vec::new();
    let mut risks = Vec::new();
    let mut reasons = Vec::new();

    // Hook: questions, imperatives, strong openers
    let hook = if text.contains('?')
        || starts_with_hook(&lower)
        || lower.contains("mira")
        || lower.contains("importante")
        || lower.contains("nunca")
        || lower.contains("siempre")
    {
        strengths.push("Buen gancho o apertura fuerte".into());
        reasons.push(reason("hook", "Gancho inicial", 0.16));
        0.85
    } else if n_words >= 4 {
        0.55
    } else {
        risks.push("Apertura débil o demasiado corta".into());
        0.35
    };
    b.hook_quality = hook;

    // Coherence: multi-sentence or multi-clause
    b.semantic_coherence = if text.contains('.') || text.contains(',') || n_words >= 12 {
        strengths.push("Idea con desarrollo".into());
        0.8
    } else if n_words >= 6 {
        0.6
    } else {
        0.4
    };

    // Standalone: complete thought markers
    b.standalone = if ends_complete(&lower) || text.ends_with('.') || text.ends_with('!') {
        strengths.push("Cierre o idea autocontenida".into());
        reasons.push(reason("standalone", "Funciona sin contexto largo", 0.14));
        0.82
    } else {
        risks.push("Puede necesitar contexto previo".into());
        0.5
    };

    b.clarity = if has_real_transcript {
        if (5..=120).contains(&n_words) {
            0.8
        } else {
            0.55
        }
    } else {
        risks.push("Sin transcripción real: scoring heurístico por audio".into());
        0.4
    };

    b.energy = unit.energy.clamp(0.3, 0.95);
    if b.energy > 0.7 {
        strengths.push("Energía / densidad verbal alta".into());
    }

    // Density: words per second
    let wps = n_words as f64 / duration.max(0.5);
    b.information_density = if (1.5..4.5).contains(&wps) {
        0.8
    } else if wps < 0.8 {
        risks.push("Baja densidad de información".into());
        0.35
    } else {
        0.55
    };

    b.has_conclusion = if lower.contains("por eso")
        || lower.contains("en resumen")
        || lower.contains("conclusión")
        || lower.contains("por lo tanto")
        || lower.contains("la clave")
        || text.ends_with('.')
    {
        strengths.push("Cierre o conclusión detectable".into());
        0.75
    } else {
        0.45
    };

    // Duration fit
    if duration < min_d {
        b.duration_fit = 0.25;
        risks.push(format!("Más corto que el mínimo ({min_d:.0}s)"));
        b.incomplete_penalty = 0.4;
    } else if duration > max_d {
        b.duration_fit = 0.35;
        risks.push(format!("Más largo que el máximo ({max_d:.0}s)"));
        b.incomplete_penalty = 0.15;
    } else {
        let dist = (duration - ideal).abs() / ideal.max(1.0);
        b.duration_fit = (1.0 - dist).clamp(0.4, 1.0);
        strengths.push("Duración dentro del perfil".into());
        b.incomplete_penalty = 0.0;
    }

    b.silence_penalty = 0.0; // filled by generator if long internal gaps

    let score = b.total();
    let confidence = if has_real_transcript {
        (0.55 + score / 250.0).clamp(0.4, 0.92)
    } else {
        (0.35 + score / 300.0).clamp(0.25, 0.7)
    };

    let title = provisional_title(text);
    let summary = if text.chars().count() > 140 {
        let t: String = text.chars().take(137).collect();
        format!("{t}…")
    } else {
        text.to_string()
    };

    ScoredText {
        breakdown: b,
        score,
        confidence,
        reasons,
        strengths,
        risks,
        title,
        summary,
    }
}

fn reason(code: &str, label: &str, weight: f64) -> ClipReason {
    ClipReason {
        code: code.into(),
        label: label.into(),
        weight,
    }
}

fn starts_with_hook(lower: &str) -> bool {
    [
        "cómo",
        "como ",
        "por qué",
        "porque ",
        "what ",
        "why ",
        "how ",
        "no hagas",
        "deja de",
        "si quieres",
        "el error",
        "la verdad",
        "secret",
    ]
    .iter()
    .any(|p| lower.starts_with(p) || lower.contains(&format!(" {p}")))
}

fn ends_complete(lower: &str) -> bool {
    lower.ends_with('.')
        || lower.ends_with('!')
        || lower.ends_with('?')
        || lower.contains("punto final")
}

fn provisional_title(text: &str) -> String {
    let lower = text.trim().to_lowercase();
    // Speech-fallback placeholders get numbered later in finalize_clip_titles
    if lower.is_empty()
        || lower.contains("[habla")
        || lower.contains("[segmento")
        || text.chars().filter(|c| c.is_alphabetic()).count() < 3
    {
        return "Clip".into();
    }
    let clean = text
        .trim()
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '[' || c == ']');
    let words: Vec<&str> = clean.split_whitespace().take(8).collect();
    if words.is_empty() {
        return "Clip".into();
    }
    let mut t = words.join(" ");
    if clean.split_whitespace().count() > 8 {
        t.push('…');
    }
    let mut c = t.chars();
    match c.next() {
        None => "Clip".into(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::Span;

    #[test]
    fn scores_question_hook_higher() {
        let u = SemanticUnit {
            id: "1".into(),
            span: Span::new(0.0, 25.0),
            text: "¿Por qué fallan la mayoría de creadores? Porque no editan con intención clara."
                .into(),
            cue_ids: vec![],
            energy: 0.8,
        };
        let s = score_unit(&u, 25.0, 30.0, 20.0, 40.0, true);
        assert!(s.score > 55.0, "score={}", s.score);
        assert!(!s.strengths.is_empty());
    }
}
