//! Resident generation supervisor — processes queue without UI (Codex CRIT-001).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::{debug, info, warn};

use crate::pipeline::visual::generation::daily_feed;
use crate::pipeline::visual::generation::worker::{recover_stale_running, worker_tick};

static STARTED: AtomicBool = AtomicBool::new(false);

/// Wake flag so enqueue can nudge the loop without busy-waiting hard.
static WAKE: AtomicBool = AtomicBool::new(false);

pub fn request_wake() {
    WAKE.store(true, Ordering::SeqCst);
}

/// Spawn once from Tauri `setup`. Safe if called multiple times.
pub fn ensure_started() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    // Recover stuck jobs from previous process immediately (sync).
    if let Err(e) = recover_stale_running() {
        warn!("visual supervisor: recover_stale_running: {e}");
    }
    tauri::async_runtime::spawn(async {
        info!("visual generation supervisor started");
        run_loop().await;
    });
}

async fn run_loop() {
    let mut idle_ticks = 0u32;
    loop {
        // Process up to a few jobs per cycle
        match worker_tick(2).await {
            Ok(n) if n > 0 => {
                idle_ticks = 0;
                debug!("supervisor processed {n} job(s)");
                continue;
            }
            Ok(_) => {}
            Err(e) => {
                warn!("supervisor worker_tick error: {e}");
                idle_ticks = idle_ticks.saturating_add(1);
            }
        }

        // Daily feed when idle of video work
        if idle_ticks % 15 == 5 {
            match daily_feed::run_daily_cycle().await {
                Ok(v) => {
                    if v.get("ok").and_then(|x| x.as_bool()) == Some(true) {
                        debug!("daily cycle ok: {v}");
                        continue;
                    }
                }
                Err(e) => warn!("daily cycle error: {e}"),
            }
        }

        // Sleep: short if wake requested, else 2s
        if WAKE.swap(false, Ordering::SeqCst) {
            idle_ticks = 0;
            tokio::time::sleep(Duration::from_millis(50)).await;
        } else {
            idle_ticks = idle_ticks.saturating_add(1);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

/// For tests: one recovery + one tick without infinite loop.
#[cfg(test)]
pub async fn run_once_for_test() -> crate::error::AppResult<u32> {
    recover_stale_running()?;
    worker_tick(1).await
}

/// Hold a cancel flag registry so cancel can abort mid-generate (best-effort).
pub mod cancel_registry {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    static FLAGS: Mutex<Option<HashMap<String, Arc<AtomicBool>>>> = Mutex::new(None);

    fn map() -> std::sync::MutexGuard<'static, Option<HashMap<String, Arc<AtomicBool>>>> {
        FLAGS.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn register(job_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        let mut g = map();
        let m = g.get_or_insert_with(HashMap::new);
        m.insert(job_id.to_string(), flag.clone());
        flag
    }

    pub fn request_cancel(job_id: &str) {
        let g = map();
        if let Some(m) = g.as_ref() {
            if let Some(f) = m.get(job_id) {
                f.store(true, Ordering::SeqCst);
            }
        }
    }

    pub fn is_cancelled(flag: &AtomicBool) -> bool {
        flag.load(Ordering::SeqCst)
    }

    pub fn clear(job_id: &str) {
        let mut g = map();
        if let Some(m) = g.as_mut() {
            m.remove(job_id);
        }
    }
}
