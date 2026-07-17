use serde::Serialize;

use crate::error::AppResult;
use crate::models::preset::ColorOptions;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorStats {
    pub filter_graph: String,
    pub message: String,
}

/// Returns FFmpeg eq filter for basic lighting/color.
/// Auto-levels analysis can be expanded with signalstats later.
#[tauri::command]
pub fn analyze_color_stats(options: ColorOptions) -> AppResult<ColorStats> {
    if !options.enabled {
        return Ok(ColorStats {
            filter_graph: String::new(),
            message: "Color correction disabled".into(),
        });
    }

    let filter = format!(
        "eq=brightness={}:contrast={}:saturation={}:gamma={}",
        options.brightness, options.contrast, options.saturation, options.gamma
    );

    Ok(ColorStats {
        filter_graph: filter,
        message: if options.auto_levels {
            "Auto-levels requested (basic eq applied; full histogram pass planned)".into()
        } else {
            "Manual color eq ready".into()
        },
    })
}
