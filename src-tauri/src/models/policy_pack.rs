use serde::{Deserialize, Serialize};

use super::edl::PolicyConfig;

/// Named policy pack for factory channels / formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyPack {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub policy: PolicyConfig,
    /// Also auto-cut high-confidence fillers when captions exist
    #[serde(default = "default_true")]
    pub cut_fillers: bool,
    /// Export top short MP4 clips
    #[serde(default = "default_true")]
    pub export_shorts: bool,
    pub is_builtin: bool,
}

fn default_true() -> bool {
    true
}

pub fn builtin_policy_packs() -> Vec<PolicyPack> {
    vec![
        PolicyPack {
            id: "factory".into(),
            name: "Factory default".into(),
            description: Some("Auto-corte agresivo de silencios; excepciones solo dudosas".into()),
            policy: PolicyConfig::default(),
            cut_fillers: true,
            export_shorts: true,
            is_builtin: true,
        },
        PolicyPack {
            id: "youtube".into(),
            name: "YouTube talking head".into(),
            description: Some("Silencios medios, padding generoso, buen ritmo".into()),
            policy: PolicyConfig {
                auto_approve_min_score: 0.82,
                min_silence_duration: 0.5,
                padding: 0.15,
                threshold: 0.5,
                prefer_silero: true,
            },
            cut_fillers: true,
            export_shorts: true,
            is_builtin: true,
        },
        PolicyPack {
            id: "podcast".into(),
            name: "Podcast / entrevista".into(),
            description: Some("Cortes más agresivos; menos padding".into()),
            policy: PolicyConfig {
                auto_approve_min_score: 0.78,
                min_silence_duration: 0.35,
                padding: 0.08,
                threshold: 0.45,
                prefer_silero: true,
            },
            cut_fillers: true,
            export_shorts: false,
            is_builtin: true,
        },
        PolicyPack {
            id: "gentle".into(),
            name: "Conservador".into(),
            description: Some("Solo silencios largos y muy claros".into()),
            policy: PolicyConfig {
                auto_approve_min_score: 0.9,
                min_silence_duration: 0.8,
                padding: 0.2,
                threshold: 0.55,
                prefer_silero: true,
            },
            cut_fillers: false,
            export_shorts: true,
            is_builtin: true,
        },
        PolicyPack {
            id: "shorts-first".into(),
            name: "Shorts first".into(),
            description: Some("Prioriza densidad y export de clips verticales".into()),
            policy: PolicyConfig {
                auto_approve_min_score: 0.8,
                min_silence_duration: 0.35,
                padding: 0.1,
                threshold: 0.48,
                prefer_silero: true,
            },
            cut_fillers: true,
            export_shorts: true,
            is_builtin: true,
        },
    ]
}
