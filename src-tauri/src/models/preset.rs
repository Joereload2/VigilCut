use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::segment::SilenceDetectionOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingPreset {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub silence: SilenceDetectionOptions,
    pub audio: AudioEnhanceOptions,
    pub color: ColorOptions,
    pub export: ExportOptions,
    pub is_builtin: bool,
}

impl Default for ProcessingPreset {
    fn default() -> Self {
        Self {
            id: "default".into(),
            name: "Default / Predeterminado".into(),
            description: Some("Balanced silence removal for talking-head content".into()),
            silence: SilenceDetectionOptions::default(),
            audio: AudioEnhanceOptions::default(),
            color: ColorOptions::default(),
            export: ExportOptions::default(),
            is_builtin: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEnhanceOptions {
    pub enabled: bool,
    pub denoise: bool,
    pub denoise_strength: f64,
    pub normalize: bool,
    pub target_lufs: f64,
    pub highpass_hz: Option<u32>,
    pub compress: bool,
}

impl Default for AudioEnhanceOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            denoise: true,
            denoise_strength: 0.35,
            normalize: true,
            target_lufs: -14.0,
            highpass_hz: Some(80),
            compress: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorOptions {
    pub enabled: bool,
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub gamma: f64,
    pub auto_levels: bool,
}

impl Default for ColorOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            auto_levels: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub container: String,
    pub video_codec: String,
    pub audio_codec: String,
    pub crf: u8,
    pub preset: String,
    pub audio_bitrate_k: u32,
    /// Re-encode vs stream-copy when possible
    pub reencode: bool,
    /// Skip cut segments in export (always true for silence-cut workflow)
    pub apply_cuts: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            container: "mp4".into(),
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
            crf: 18,
            preset: "medium".into(),
            audio_bitrate_k: 192,
            reencode: true,
            apply_cuts: true,
        }
    }
}

pub fn builtin_presets() -> Vec<ProcessingPreset> {
    vec![
        ProcessingPreset::default(),
        ProcessingPreset {
            id: "podcast".into(),
            name: "Podcast / Interview".into(),
            description: Some("Aggressive silence cut + strong denoise/normalize".into()),
            silence: SilenceDetectionOptions {
                min_silence_duration: 0.35,
                padding: 0.08,
                threshold: 0.45,
                prefer_silero: true,
                auto_cut_silence: true,
            },
            audio: AudioEnhanceOptions {
                enabled: true,
                denoise: true,
                denoise_strength: 0.5,
                normalize: true,
                target_lufs: -16.0,
                highpass_hz: Some(70),
                compress: true,
            },
            color: ColorOptions::default(),
            export: ExportOptions::default(),
            is_builtin: true,
        },
        ProcessingPreset {
            id: "youtube-talking-head".into(),
            name: "YouTube Talking Head".into(),
            description: Some("Moderate silence cut, light color pop, loudness -14 LUFS".into()),
            silence: SilenceDetectionOptions {
                min_silence_duration: 0.5,
                padding: 0.15,
                threshold: 0.5,
                prefer_silero: true,
                auto_cut_silence: true,
            },
            audio: AudioEnhanceOptions {
                enabled: true,
                denoise: true,
                denoise_strength: 0.3,
                normalize: true,
                target_lufs: -14.0,
                highpass_hz: Some(80),
                compress: false,
            },
            color: ColorOptions {
                enabled: true,
                brightness: 0.03,
                contrast: 1.05,
                saturation: 1.08,
                gamma: 1.0,
                auto_levels: false,
            },
            export: ExportOptions {
                crf: 18,
                preset: "slow".into(),
                ..ExportOptions::default()
            },
            is_builtin: true,
        },
        ProcessingPreset {
            id: "gentle".into(),
            name: "Gentle / Conservador".into(),
            description: Some("Only long silences; safe padding for natural pacing".into()),
            silence: SilenceDetectionOptions {
                min_silence_duration: 0.8,
                padding: 0.2,
                threshold: 0.55,
                prefer_silero: true,
                auto_cut_silence: true,
            },
            audio: AudioEnhanceOptions {
                enabled: false,
                ..AudioEnhanceOptions::default()
            },
            color: ColorOptions::default(),
            export: ExportOptions::default(),
            is_builtin: true,
        },
        ProcessingPreset {
            id: Uuid::new_v4().to_string(),
            name: "Clip Select".into(),
            description: Some("Pre-select best takes; silence not auto-cut".into()),
            silence: SilenceDetectionOptions {
                min_silence_duration: 0.4,
                padding: 0.1,
                threshold: 0.5,
                prefer_silero: true,
                auto_cut_silence: false,
            },
            audio: AudioEnhanceOptions::default(),
            color: ColorOptions::default(),
            export: ExportOptions::default(),
            is_builtin: true,
        },
    ]
}
