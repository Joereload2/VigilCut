//! Silero VAD via ONNX Runtime (`ort`).
//! Falls back gracefully if the model or runtime is unavailable.
//!
//! Sprint A: disk cache of silence ranges (same media+params → no re-VAD)
//! + cooperative cancel checkpoints during long runs.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::pipeline::features::{ensure_audio_16k, media_cache_key};
use crate::state::AppState;

const SAMPLE_RATE: u32 = 16_000;
const WINDOW: usize = 512; // 32 ms @ 16 kHz
/// Check cancel every N windows (~0.5s of audio at 16 kHz / 512).
const CANCEL_EVERY: usize = 16;

#[derive(Debug, Serialize, Deserialize)]
struct VadCacheFile {
    method: String,
    min_silence_duration: f64,
    speech_threshold: f64,
    ranges: Vec<[f64; 2]>,
}

fn vad_cache_path(
    media_path: &Path,
    min_silence_duration: f64,
    speech_threshold: f64,
) -> AppResult<PathBuf> {
    let key = media_cache_key(media_path)?;
    let dir = AppState::cache_dir()?.join("features").join(&key);
    std::fs::create_dir_all(&dir)?;
    // Quantize params so tiny float noise does not bust cache.
    let ms = (min_silence_duration * 1000.0).round() as i64;
    let thr = (speech_threshold.clamp(0.15, 0.85) * 100.0).round() as i64;
    Ok(dir.join(format!("vad_silero_ms{ms}_ thr{thr}.json").replace(" thr", "_thr")))
}

fn load_vad_cache(
    media_path: &Path,
    min_silence_duration: f64,
    speech_threshold: f64,
) -> Option<Vec<(f64, f64)>> {
    let path = vad_cache_path(media_path, min_silence_duration, speech_threshold).ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    let file: VadCacheFile = serde_json::from_str(&text).ok()?;
    if file.method != "silero_vad" {
        return None;
    }
    Some(file.ranges.into_iter().map(|r| (r[0], r[1])).collect())
}

fn save_vad_cache(
    media_path: &Path,
    min_silence_duration: f64,
    speech_threshold: f64,
    ranges: &[(f64, f64)],
) {
    let Ok(path) = vad_cache_path(media_path, min_silence_duration, speech_threshold) else {
        return;
    };
    let file = VadCacheFile {
        method: "silero_vad".into(),
        min_silence_duration,
        speech_threshold,
        ranges: ranges.iter().map(|(s, e)| [*s, *e]).collect(),
    };
    if let Ok(json) = serde_json::to_string(&file) {
        let _ = std::fs::write(path, json);
    }
}

/// Returns silence ranges (start, end) in seconds using Silero VAD ONNX.
pub async fn detect_silences_silero(
    media_path: &Path,
    min_silence_duration: f64,
    speech_threshold: f64,
) -> AppResult<Vec<(f64, f64)>> {
    detect_silences_silero_cancellable(media_path, min_silence_duration, speech_threshold, None)
        .await
}

/// Same as [`detect_silences_silero`] with optional cooperative cancel.
pub async fn detect_silences_silero_cancellable(
    media_path: &Path,
    min_silence_duration: f64,
    speech_threshold: f64,
    cancelled: Option<&AtomicBool>,
) -> AppResult<Vec<(f64, f64)>> {
    if let Some(cached) = load_vad_cache(media_path, min_silence_duration, speech_threshold) {
        tracing::info!("Silero VAD cache hit ({} ranges)", cached.len());
        return Ok(cached);
    }

    let wav_path = ensure_audio_16k(media_path).await?;
    if cancelled.map(|c| c.load(Ordering::SeqCst)).unwrap_or(false) {
        return Err(AppError::Cancelled);
    }
    let samples = read_wav_f32_mono(&wav_path)?;
    if samples.len() < WINDOW {
        return Ok(Vec::new());
    }

    let model = AppState::models_dir()?.join("silero_vad.onnx");
    if !model.is_file() {
        return Err(AppError::NotFound(format!(
            "Silero model missing: {}",
            model.display()
        )));
    }

    let probs = run_silero_probs(&model, &samples, cancelled)?;
    let thr = speech_threshold.clamp(0.15, 0.85) as f32;
    let frame_sec = WINDOW as f64 / SAMPLE_RATE as f64;
    let mut silence = Vec::new();
    let mut in_sil = false;
    let mut sil_start = 0.0_f64;

    for (i, p) in probs.iter().enumerate() {
        let t = i as f64 * frame_sec;
        let is_speech = *p >= thr;
        if !is_speech {
            if !in_sil {
                in_sil = true;
                sil_start = t;
            }
        } else if in_sil {
            let end = t;
            if end - sil_start >= min_silence_duration {
                silence.push((sil_start, end));
            }
            in_sil = false;
        }
    }
    if in_sil {
        let end = samples.len() as f64 / SAMPLE_RATE as f64;
        if end - sil_start >= min_silence_duration {
            silence.push((sil_start, end));
        }
    }

    save_vad_cache(media_path, min_silence_duration, speech_threshold, &silence);
    Ok(silence)
}

fn run_silero_probs(
    model_path: &Path,
    samples: &[f32],
    cancelled: Option<&AtomicBool>,
) -> AppResult<Vec<f32>> {
    use ort::session::Session;

    let mut session = Session::builder()
        .map_err(|e| AppError::Message(format!("ort session: {e}")))?
        .commit_from_file(model_path)
        .map_err(|e| AppError::Message(format!("ort load model: {e}")))?;

    let mut state = vec![0.0f32; 2 * 128];
    let mut context = vec![0.0f32; 64];
    let mut probs = Vec::new();

    let n_windows = samples.len() / WINDOW;
    for w in 0..n_windows {
        if w % CANCEL_EVERY == 0
            && cancelled.map(|c| c.load(Ordering::SeqCst)).unwrap_or(false)
        {
            return Err(AppError::Cancelled);
        }
        let start = w * WINDOW;
        let chunk = &samples[start..start + WINDOW];
        let mut input_vec = context.clone();
        input_vec.extend_from_slice(chunk);

        let result = run_window_v5(&mut session, &input_vec, &mut state)
            .or_else(|_| run_window_simple(&mut session, chunk));

        match result {
            Ok((prob, new_ctx)) => {
                probs.push(prob);
                if let Some(ctx) = new_ctx {
                    if ctx.len() == context.len() {
                        context.copy_from_slice(&ctx);
                    }
                } else if chunk.len() >= 64 {
                    context.copy_from_slice(&chunk[chunk.len() - 64..]);
                }
            }
            Err(e) => {
                if w == 0 {
                    return Err(e);
                }
                break;
            }
        }
    }

    if probs.is_empty() {
        return Err(AppError::Message("Silero produced no frames".into()));
    }
    Ok(probs)
}

fn run_window_simple(
    session: &mut ort::session::Session,
    chunk: &[f32],
) -> AppResult<(f32, Option<Vec<f32>>)> {
    use ort::value::Tensor;

    let input = Tensor::from_array(([1usize, chunk.len()], chunk.to_vec()))
        .map_err(|e| AppError::Message(e.to_string()))?;

    let outputs = session
        .run(ort::inputs!["input" => input])
        .map_err(|e| AppError::Message(format!("ort run simple: {e}")))?;

    let prob = extract_prob(&outputs)?;
    Ok((prob, None))
}

fn run_window_v5(
    session: &mut ort::session::Session,
    input_with_ctx: &[f32],
    state: &mut [f32],
) -> AppResult<(f32, Option<Vec<f32>>)> {
    use ort::value::Tensor;

    let n = input_with_ctx.len();
    let input = Tensor::from_array(([1usize, n], input_with_ctx.to_vec()))
        .map_err(|e| AppError::Message(e.to_string()))?;
    let state_t = Tensor::from_array(([2usize, 1, 128], state.to_vec()))
        .map_err(|e| AppError::Message(e.to_string()))?;
    let sr_t = Tensor::from_array(((), vec![SAMPLE_RATE as i64]))
        .or_else(|_| Tensor::from_array(([1usize], vec![SAMPLE_RATE as i64])))
        .map_err(|e| AppError::Message(e.to_string()))?;

    let outputs = session
        .run(ort::inputs![
            "input" => input,
            "state" => state_t,
            "sr" => sr_t,
        ])
        .map_err(|e| AppError::Message(format!("ort run v5: {e}")))?;

    if let Some(out_state) = outputs.get("state").or_else(|| outputs.get("hn")) {
        if let Ok((_, data)) = out_state.try_extract_tensor::<f32>() {
            let flat: Vec<f32> = data.to_vec();
            if flat.len() == state.len() {
                state.copy_from_slice(&flat);
            }
        }
    }

    let prob = extract_prob(&outputs)?;
    let ctx = if input_with_ctx.len() >= 64 {
        Some(input_with_ctx[input_with_ctx.len() - 64..].to_vec())
    } else {
        None
    };
    Ok((prob, ctx))
}

fn extract_prob(outputs: &ort::session::SessionOutputs<'_>) -> AppResult<f32> {
    for name in ["output", "prob", "logits"] {
        if let Some(v) = outputs.get(name) {
            if let Ok((_, data)) = v.try_extract_tensor::<f32>() {
                if let Some(p) = data.iter().next() {
                    return Ok(p.clamp(0.0, 1.0));
                }
            }
        }
    }
    let first = outputs
        .iter()
        .next()
        .ok_or_else(|| AppError::Message("no ort outputs".into()))?;
    let (_, data) = first
        .1
        .try_extract_tensor::<f32>()
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(data.iter().copied().next().unwrap_or(0.0).clamp(0.0, 1.0))
}

fn read_wav_f32_mono(path: &Path) -> AppResult<Vec<f32>> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 44 {
        return Err(AppError::Invalid("wav too small".into()));
    }
    let mut i = 12usize;
    let mut data_off = 44usize;
    let mut data_len = bytes.len().saturating_sub(44);
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len =
            u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        if id == b"data" {
            data_off = i + 8;
            data_len = len.min(bytes.len().saturating_sub(data_off));
            break;
        }
        i += 8 + len + (len % 2);
    }
    let pcm = &bytes[data_off..data_off + data_len];
    let mut samples = Vec::with_capacity(pcm.len() / 2);
    for c in pcm.chunks_exact(2) {
        let s = i16::from_le_bytes([c[0], c[1]]);
        samples.push(s as f32 / 32768.0);
    }
    Ok(samples)
}
