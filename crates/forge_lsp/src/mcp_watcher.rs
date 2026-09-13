//! `McpWatcher` — debounced file watcher for MCP config auto-reload.
//!
//! Watches a single config file (default `~/.config/forge/mcp.toml`) and
//! invokes a callback after the file has been stable for `debounce_ms`.
//! Uses mtime polling so no extra native dependency (`notify`) is required.

use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

/// Default debounce: 500 ms (matches spec).
pub const DEFAULT_DEBOUNCE_MS: u64 = 500;
/// Poll interval: 200 ms.
const POLL_MS: u64 = 200;

/// Handle returned by [`McpWatcher::spawn`]; dropping it stops the watcher.
pub struct McpWatcherHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for McpWatcherHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
    }
}

/// Debounced mtime watcher.
pub struct McpWatcher {
    path: PathBuf,
    debounce: Duration,
}

impl McpWatcher {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), debounce: Duration::from_millis(DEFAULT_DEBOUNCE_MS) }
    }
    pub fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce = Duration::from_millis(ms);
        self
    }
    pub fn path(&self) -> &Path { &self.path }

    /// Spawn a background thread that calls `on_reload` debounced.
    /// Returns a handle; dropping it stops the thread.
    pub fn spawn<F>(&self, on_reload: F) -> McpWatcherHandle
    where F: Fn() + Send + 'static {
        let path = self.path.clone();
        let debounce = self.debounce;
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let thread = std::thread::spawn(move || {
            let mut last_mtime: Option<SystemTime> = mtime_of(&path);
            let mut pending_since: Option<std::time::Instant> = None;
            while !stop2.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(POLL_MS));
                let cur = mtime_of(&path);
                if cur != last_mtime {
                    last_mtime = cur;
                    pending_since = Some(std::time::Instant::now());
                }
                if let Some(since) = pending_since {
                    if since.elapsed() >= debounce {
                        pending_since = None;
                        on_reload();
                    }
                }
            }
        });
        McpWatcherHandle { stop, thread: Some(thread) }
    }

    /// Synchronous helper used by tests: returns true if `path` was
    /// modified after `baseline` (mtime comparison).
    pub fn has_changed_since(&self, baseline: Option<SystemTime>) -> bool {
        mtime_of(&self.path) != baseline
    }
    pub fn current_mtime(&self) -> Option<SystemTime> { mtime_of(&self.path) }

    /// Default config path: `~/.config/forge/mcp.toml` (falls back to
    /// temp dir on Windows when HOME is unset).
    pub fn default_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            PathBuf::from(home).join(".config").join("forge").join("mcp.toml")
        } else {
            std::env::temp_dir().join("forge_mcp.toml")
        }
    }
}

fn mtime_of(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test] fn default_path_ends_with_mcp_toml() {
        assert!(McpWatcher::default_path().ends_with("mcp.toml"));
    }
    #[test] fn new_stores_path() {
        let w = McpWatcher::new("/tmp/foo.toml");
        assert_eq!(w.path(), Path::new("/tmp/foo.toml"));
    }
    #[test] fn with_debounce_sets_duration() {
        let w = McpWatcher::new("/tmp/x").with_debounce(123);
        assert_eq!(w.debounce, Duration::from_millis(123));
    }
    #[test] fn missing_file_has_no_mtime() {
        let w = McpWatcher::new("/tmp/forge_lsp_test_missing_9f3a.toml");
        let _ = std::fs::remove_file(w.path());
        assert!(w.current_mtime().is_none());
        assert!(!w.has_changed_since(None));
    }
    #[test] fn detects_creation() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("forge_watcher_test_{}.toml", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let w = McpWatcher::new(&p);
        let before = w.current_mtime();
        assert!(before.is_none());
        std::fs::write(&p, b"hello").unwrap();
        assert!(w.has_changed_since(before));
        let _ = std::fs::remove_file(&p);
    }
    #[test] fn detects_modification() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("forge_watcher_mod_{}.toml", std::process::id()));
        std::fs::write(&p, b"v1").unwrap();
        let w = McpWatcher::new(&p);
        let m1 = w.current_mtime();
        std::thread::sleep(Duration::from_millis(10));
        std::fs::write(&p, b"v2").unwrap();
        assert!(w.has_changed_since(m1));
        let _ = std::fs::remove_file(&p);
    }
    #[test] fn spawn_calls_callback_on_change() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("forge_watcher_spawn_{}.toml", std::process::id()));
        let _ = std::fs::remove_file(&p);
        std::fs::write(&p, b"init").unwrap();
        let w = McpWatcher::new(&p).with_debounce(100);
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c2 = count.clone();
        let _handle = w.spawn(move || { c2.fetch_add(1, Ordering::Relaxed); });
        std::thread::sleep(Duration::from_millis(150));
        std::fs::write(&p, b"changed").unwrap();
        // wait for debounce + poll
        std::thread::sleep(Duration::from_millis(500));
        assert!(count.load(Ordering::Relaxed) >= 1, "callback should have fired");
        let _ = std::fs::remove_file(&p);
    }
    #[test] fn spawn_debounce_coalesces_rapid_writes() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("forge_watcher_debounce_{}.toml", std::process::id()));
        let _ = std::fs::remove_file(&p);
        std::fs::write(&p, b"init").unwrap();
        let w = McpWatcher::new(&p).with_debounce(300);
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let c2 = count.clone();
        let _handle = w.spawn(move || { c2.fetch_add(1, Ordering::Relaxed); });
        std::thread::sleep(Duration::from_millis(100));
        for i in 0..5 {
            std::fs::write(&p, format!("v{i}")).unwrap();
            std::thread::sleep(Duration::from_millis(40));
        }
        std::thread::sleep(Duration::from_millis(700));
        let n = count.load(Ordering::Relaxed);
        assert!(n >= 1 && n <= 2, "debounce should coalesce, got {n}");
        let _ = std::fs::remove_file(&p);
    }
    #[test] fn has_changed_since_detects_deletion() {
        let dir = std::env::temp_dir();
        let p = dir.join(format!("forge_watcher_del_{}.toml", std::process::id()));
        std::fs::write(&p, b"hi").unwrap();
        let w = McpWatcher::new(&p);
        let m = w.current_mtime();
        std::fs::remove_file(&p).unwrap();
        assert!(w.has_changed_since(m));
    }
}
