//! Cooperative cancel for long analysis / export jobs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::error::{AppError, AppResult};

#[derive(Default)]
pub struct JobControl {
    cancelled: AtomicBool,
    ffmpeg_pid: Mutex<Option<u32>>,
}

impl JobControl {
    /// Call at the start of each user-facing long job.
    pub fn begin(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
        if let Ok(mut g) = self.ffmpeg_pid.lock() {
            *g = None;
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(mut g) = self.ffmpeg_pid.lock() {
            if let Some(pid) = g.take() {
                kill_pid(pid);
            }
        }
    }

    pub fn set_ffmpeg_pid(&self, pid: Option<u32>) {
        if let Ok(mut g) = self.ffmpeg_pid.lock() {
            *g = pid;
        }
    }

    pub fn check(&self) -> AppResult<()> {
        if self.is_cancelled() {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
    }
    tracing::info!("Cancelled job — killed process {pid}");
}
