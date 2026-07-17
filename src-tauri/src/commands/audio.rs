use serde::Serialize;

use crate::error::AppResult;
use crate::models::preset::AudioEnhanceOptions;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEnhancePreview {
    pub filter_graph: String,
    pub message: String,
}

/// Builds the FFmpeg audio filter graph for denoise/normalize.
/// Full offline render happens at export time.
#[tauri::command]
pub fn enhance_audio_preview(options: AudioEnhanceOptions) -> AppResult<AudioEnhancePreview> {
    if !options.enabled {
        return Ok(AudioEnhancePreview {
            filter_graph: String::new(),
            message: "Audio enhance disabled".into(),
        });
    }

    let mut filters = Vec::new();

    if let Some(hz) = options.highpass_hz {
        filters.push(format!("highpass=f={hz}"));
    }
    if options.denoise {
        // afftdn is widely available; strength mapped loosely via nr
        let nr = (options.denoise_strength * 20.0).clamp(1.0, 30.0);
        filters.push(format!("afftdn=nr={nr}"));
    }
    if options.compress {
        filters.push("acompressor=threshold=-18dB:ratio=3:attack=20:release=250".into());
    }
    if options.normalize {
        // loudnorm two-pass is ideal; single-pass for preview graph
        filters.push(format!(
            "loudnorm=I={}:TP=-1.5:LRA=11",
            options.target_lufs
        ));
    }

    let graph = filters.join(",");
    Ok(AudioEnhancePreview {
        message: format!("Audio filter ready ({} stages)", filters.len()),
        filter_graph: graph,
    })
}
