//! Shared helpers for smoke / e2e tests (synthetic media via bundled FFmpeg).

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Path to project-bundled ffmpeg.exe (src-tauri/binaries).
pub fn bundled_ffmpeg() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        })
}

/// Prefer sidecar under `src-tauri/binaries/`; fall back to `ffmpeg` on PATH (CI).
pub fn resolve_ffmpeg() -> PathBuf {
    let bundled = bundled_ffmpeg();
    if bundled.is_file() {
        return bundled;
    }
    // CI often installs system FFmpeg via choco/apt without running setup:ffmpeg.
    which_ffmpeg().unwrap_or(bundled)
}

fn which_ffmpeg() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    // PATH lookup without shell
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        // Windows: also accept bare "ffmpeg" if present
        if cfg!(windows) {
            let bare = dir.join("ffmpeg");
            if bare.is_file() {
                return Some(bare);
            }
        }
    }
    None
}

pub fn ensure_ffmpeg() {
    let ff = resolve_ffmpeg();
    assert!(
        ff.is_file(),
        "ffmpeg not found (tried {} and PATH) — run npm run setup:ffmpeg or install ffmpeg",
        bundled_ffmpeg().display()
    );
}

fn run_ffmpeg(args: &[&str]) {
    ensure_ffmpeg();
    let mut cmd = Command::new(resolve_ffmpeg());
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().expect("spawn ffmpeg");
    if !out.status.success() {
        panic!("ffmpeg failed:\n{}", String::from_utf8_lossy(&out.stderr));
    }
}

/// 3s black video + tone with a mid silence (1s–2s) for VAD / silence pipeline.
#[allow(dead_code)] // used by smoke_pipeline / smoke_clipping; not every crate test
pub fn make_talking_head_fixture(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    // lavfi: color video + sine audio; mute middle second so silencedetect finds a gap
    run_ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        "color=c=black:s=320x240:d=3:r=25",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=880:sample_rate=44100:duration=3",
        "-af",
        "volume=enable='between(t,1,2)':volume=0",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-preset",
        "ultrafast",
        "-c:a",
        "aac",
        "-shortest",
        &path.to_string_lossy(),
    ]);
    assert!(path.is_file() && path.metadata().unwrap().len() > 1000);
}

/// Temp dir under target/ so CI and local leave a predictable footprint.
pub fn test_workspace(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-workspace")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("test workspace");
    dir
}

/// Longer fixture (~30s) with two silence gaps for multi-candidate clipping.
#[allow(dead_code)] // used by e2e_clipping; not every crate test
pub fn make_long_talking_fixture(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    run_ffmpeg(&[
        "-y",
        "-f",
        "lavfi",
        "-i",
        "color=c=black:s=320x240:d=30:r=25",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=660:sample_rate=44100:duration=30",
        "-af",
        "volume=enable='between(t,8,10)+between(t,18,20)':volume=0",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-preset",
        "ultrafast",
        "-c:a",
        "aac",
        "-shortest",
        &path.to_string_lossy(),
    ]);
    assert!(path.is_file() && path.metadata().unwrap().len() > 1000);
}
