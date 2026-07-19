//! Shared helpers for smoke / e2e tests (synthetic media via bundled FFmpeg).

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Path to project-bundled ffmpeg.exe (src-tauri/binaries).
pub fn bundled_ffmpeg() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    })
}

pub fn ensure_ffmpeg() {
    let ff = bundled_ffmpeg();
    assert!(
        ff.is_file(),
        "bundled ffmpeg missing at {} — run npm run setup:ffmpeg",
        ff.display()
    );
}

fn run_ffmpeg(args: &[&str]) {
    ensure_ffmpeg();
    let mut cmd = Command::new(bundled_ffmpeg());
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().expect("spawn ffmpeg");
    if !out.status.success() {
        panic!(
            "ffmpeg failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// 3s black video + tone with a mid silence (1s–2s) for VAD / silence pipeline.
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
