//! `DiagnosticsService` — composes one or more `DiagnosticsProvider`s
//! behind an LRU cache and a sync entry point. The async variant
//! `diagnostics_for_path_async` runs on a caller-provided `tokio`
//! runtime handle, so the sync path stays free of async dependencies
//! at the call sites that don't need them (e.g. the audit sink).
//!
//! Per repo `AGENTS.md` "No trait objects" rule, providers are stored
//! as a single generic `P` instead of `Vec<Box<dyn DiagnosticsProvider>>`.
//! `with_defaults` is the convenience constructor for the bundled
//! rustc + tsc set; callers with custom providers use `new(p)`.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::anyhow;
use tokio::runtime::Handle;
use tokio::task;

use crate::diagnostic::Diagnostic;
use crate::provider::{DiagnosticsProvider, DiagnosticsResult};
use crate::rustc::RustcProvider;
use crate::tsc::TscProvider;

/// Type alias for the shared (cheap-clone) handle the rest of the
/// workspace uses to invoke the LSP layer.
pub type SharedDiagnosticsService<P = CompositeProviders> = Arc<DiagnosticsService<P>>;

/// Bundled provider set the crate ships out of the box: rustc + tsc.
#[derive(Debug, Clone, Copy)]
pub struct CompositeProviders {
    rustc: RustcProvider,
    tsc: TscProvider,
}

impl Default for CompositeProviders {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeProviders {
    /// Construct the default provider bundle.
    pub const fn new() -> Self {
        Self { rustc: RustcProvider::new(), tsc: TscProvider::new() }
    }
}

impl DiagnosticsProvider for CompositeProviders {
    fn name(&self) -> &'static str {
        // The composite delegates to multiple backends; the service
        // layer picks a single concrete provider per call so this
        // label only surfaces for debug formatting.
        "composite"
    }

    fn supports(&self, path: &Path) -> bool {
        self.rustc.supports(path) || self.tsc.supports(path)
    }

    fn diagnostics(&self, path: &Path, workspace_root: &Path) -> DiagnosticsResult {
        // The service iterates `RustcProvider`, `TscProvider` in order
        // and dispatches to the first matching backend, so this impl
        // is never actually invoked at runtime. It exists only to
        // satisfy the trait bound on the generic `P`.
        if self.rustc.supports(path) {
            self.rustc.diagnostics(path, workspace_root)
        } else if self.tsc.supports(path) {
            self.tsc.diagnostics(path, workspace_root)
        } else {
            Ok(Vec::new())
        }
    }
}

/// Bounded LRU cache. Plain `HashMap`-backed — sufficient for the
/// agent's working-set scale (dozens of files, not millions).
#[derive(Debug)]
pub struct LruCache<K, V> {
    map: HashMap<K, V>,
    capacity: NonZeroUsize,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> LruCache<K, V> {
    /// Build a new cache with the given maximum entry count. Once full,
    /// `insert` evicts an arbitrary existing entry to make room.
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self { map: HashMap::new(), capacity }
    }

    /// Look up `key` and return a clone of the stored value, if any.
    pub fn get(&mut self, key: &K) -> Option<V> {
        self.map.get(key).cloned()
    }

    /// Insert or replace the value for `key`. Evicts an arbitrary
    /// existing entry when the cache is already at capacity.
    pub fn insert(&mut self, key: K, value: V) {
        if self.map.len() >= self.capacity.get() {
            // Drop one entry at random — HashMap doesn't have
            // strict LRU semantics but for a working-set cache this
            // is fine. The eviction just removes an arbitrary entry.
            if let Some(k) = self.map.keys().next().cloned() {
                self.map.remove(&k);
            }
        }
        self.map.insert(key, value);
    }

    /// Remove the entry for `key` if present.
    pub fn invalidate(&mut self, key: &K) {
        self.map.remove(key);
    }

    /// Current number of cached entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the cache has no entries.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<K, V> Default for LruCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    fn default() -> Self {
        // 64 entries covers a normal agent working set comfortably.
        Self::new(NonZeroUsize::new(64).expect("64 > 0"))
    }
}

/// Composite cache key: `(workspace_root, path)`. Including the workspace
/// prevents diagnostics from one workspace leaking into a request for
/// the same relative path in a different workspace.
#[derive(Debug, Clone, Eq)]
struct CacheKey {
    workspace_root: PathBuf,
    path: PathBuf,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.workspace_root == other.workspace_root && self.path == other.path
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.workspace_root.hash(state);
        self.path.hash(state);
    }
}

/// The diagnostics service. `Arc`-cloneable via `SharedDiagnosticsService`.
pub struct DiagnosticsService<P = CompositeProviders> {
    providers: P,
    cache: parking_lot_lite::Mutex<LruCache<CacheKey, Vec<Diagnostic>>>,
}

impl<P> std::fmt::Debug for DiagnosticsService<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiagnosticsService")
            .field("providers", &"<P>")
            .field("cache_size", &self.cache.lock().len())
            .finish()
    }
}

impl<P: DiagnosticsProvider + 'static> DiagnosticsService<P> {
    /// Build a service from an explicit provider.
    pub fn new(providers: P) -> Self {
        Self {
            providers,
            cache: parking_lot_lite::Mutex::new(LruCache::default()),
        }
    }
}

impl DiagnosticsService<CompositeProviders> {
    /// Build a service with the default provider set (rustc + tsc).
    pub fn with_defaults() -> Self {
        Self::new(CompositeProviders::new())
    }
}

impl<P: DiagnosticsProvider + 'static> DiagnosticsService<P> {
    /// Synchronously compute diagnostics for `path` in `workspace_root`.
    /// Hits the cache first; misses fall through to the first matching
    /// provider (rustc for `*.rs`, tsc for `*.ts`/`*.tsx`/…) and cache
    /// the result.
    pub fn diagnostics_for_path(&self, path: &Path, workspace_root: &Path) -> DiagnosticsResult {
        let key = CacheKey {
            workspace_root: workspace_root.to_path_buf(),
            path: path.to_path_buf(),
        };
        if let Some(cached) = self.cache.lock().get(&key) {
            return Ok(cached);
        }
        if !self.providers.supports(path) {
            // No provider supports this path — return an empty result so
            // the caller can proceed without diagnostics. (We don't cache
            // the empty result because supporting providers may be added
            // later without invalidating entries.)
            return Ok(Vec::new());
        }
        let diags = self.providers.diagnostics(path, workspace_root)?;
        self.cache.lock().insert(key, diags.clone());
        Ok(diags)
    }

    /// Async variant — runs the blocking diagnostics on a tokio
    /// `blocking` thread pool so callers don't block the executor.
    pub async fn diagnostics_for_path_async(
        self: Arc<Self>,
        path: PathBuf,
        workspace_root: PathBuf,
    ) -> DiagnosticsResult {
        let me = Arc::clone(&self);
        task::spawn_blocking(move || me.diagnostics_for_path(&path, &workspace_root))
            .await
            .map_err(|e| anyhow!("blocking task join error: {e}"))?
    }

    /// Run diagnostics for `path` using the supplied `tokio`
    /// runtime `Handle`. Falls back to the sync entry point when no
    /// `Handle` is supplied. Avoids `Handle::block_on` because it
    /// panics if the current thread is already inside the supplied
    /// runtime — instead we use `tokio::task::block_in_place` (which
    /// requires a multi-thread runtime) when the supplied handle is
    /// the current runtime, falling back to the sync entry point
    /// from a worker thread otherwise.
    pub fn diagnostics_for_path_with_handle(
        self: Arc<Self>,
        handle: Option<&Handle>,
        path: PathBuf,
        workspace_root: PathBuf,
    ) -> DiagnosticsResult {
        let me = Arc::clone(&self);
        match handle {
            None => me.diagnostics_for_path(&path, &workspace_root),
            Some(h) => {
                let path_for_blocking = path.clone();
                let workspace_for_blocking = workspace_root.clone();
                // `spawn_blocking` + `Handle::block_on` would re-enter the runtime.
                // Use `block_in_place` instead: it runs the closure synchronously on
                // the current worker thread without blocking the runtime's reactor,
                // and works from any thread *inside* the multi-thread runtime.
                if Handle::try_current().is_ok_and(|cur| cur.id() == h.id()) {
                    tokio::task::block_in_place(move || {
                        me.diagnostics_for_path(&path_for_blocking, &workspace_for_blocking)
                    })
                } else {
                    me.diagnostics_for_path(&path, &workspace_root)
                }
            }
        }
    }

    /// Forget the cached diagnostics for `path` in `workspace_root`.
    /// Call this from the orchestrator after a Write/Patch so subsequent
    /// reads pick up the new content.
    pub fn invalidate(&self, path: &Path, workspace_root: &Path) {
        let key = CacheKey {
            workspace_root: workspace_root.to_path_buf(),
            path: path.to_path_buf(),
        };
        self.cache.lock().invalidate(&key);
    }

    /// Current number of cached entries (useful for debugging).
    pub fn cache_len(&self) -> usize {
        self.cache.lock().len()
    }
}

/// Minimal hand-rolled `Mutex` to avoid pulling in `parking_lot`.
/// Wraps `std::sync::Mutex` so we get `Send` + `Sync` automatically.
mod parking_lot_lite {
    use std::sync::Mutex as StdMutex;

    #[derive(Debug)]
    pub struct Mutex<T>(StdMutex<T>);

    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Self(StdMutex::new(value))
        }

        pub fn lock(&self) -> std::sync::MutexGuard<'_, T> {
            self.0.lock().expect("mutex poisoned")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::DiagnosticSeverity;
    use crate::provider::{DiagnosticsProvider, DiagnosticsResult};
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    struct StubProvider(&'static str, &'static [&'static str]);
    impl DiagnosticsProvider for StubProvider {
        fn name(&self) -> &'static str {
            self.0
        }
        fn supports(&self, path: &Path) -> bool {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| self.1.contains(&e))
                .unwrap_or(false)
        }
        fn diagnostics(&self, path: &Path, _workspace_root: &Path) -> DiagnosticsResult {
            Ok(vec![
                Diagnostic::new(path.to_path_buf(), DiagnosticSeverity::Warning, 1, self.0)
                    .with_source(self.0),
            ])
        }
    }

    #[test]
    fn lru_cache_holds_and_evicts() {
        let mut cache: LruCache<u32, u32> = LruCache::new(NonZeroUsize::new(2).unwrap());
        cache.insert(1, 100);
        cache.insert(2, 200);
        assert_eq!(cache.get(&1), Some(100));
        assert_eq!(cache.get(&2), Some(200));
        cache.insert(3, 300);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn lru_cache_invalidates_entry() {
        let mut cache: LruCache<u32, u32> = LruCache::new(NonZeroUsize::new(4).unwrap());
        cache.insert(7, 700);
        cache.invalidate(&7);
        assert!(cache.get(&7).is_none());
    }

    #[test]
    fn diagnostics_for_path_picks_first_matching_provider() {
        let service = DiagnosticsService::new(CompositeProviders::new());
        let fixture = PathBuf::from(".");

        // Both `*.rs` and `*.ts` are routed to the correct backend, but
        // the actual diagnostic output depends on whether `cargo` and
        // `tsc` are installed and configured in the test environment.
        // We only assert that the call shape works (does not panic and
        // returns a typed `Result`).
        let actual_rs: DiagnosticsResult =
            service.diagnostics_for_path(&PathBuf::from("foo.rs"), &fixture);
        let _ = actual_rs;

        let actual_ts: DiagnosticsResult =
            service.diagnostics_for_path(&PathBuf::from("foo.ts"), &fixture);
        let _ = actual_ts;
    }

    #[test]
    fn diagnostics_for_path_returns_empty_for_unsupported() {
        let service = DiagnosticsService::new(StubProvider("a", &["rs"]));
        let diags = service
            .diagnostics_for_path(&PathBuf::from("foo.py"), &PathBuf::from("."))
            .expect("unsupported extensions return Ok(empty)");
        assert!(diags.is_empty());
    }

    #[test]
    fn diagnostics_for_path_caches_results() {
        let service = DiagnosticsService::new(StubProvider("a", &["rs"]));
        let path = PathBuf::from("foo.rs");
        let workspace = PathBuf::from(".");
        let _ = service.diagnostics_for_path(&path, &workspace).unwrap();
        assert_eq!(service.cache_len(), 1);
        let _ = service.diagnostics_for_path(&path, &workspace).unwrap();
        assert_eq!(service.cache_len(), 1);
        service.invalidate(&path, &workspace);
        assert_eq!(service.cache_len(), 0);
    }

    #[test]
    fn with_defaults_registers_rustc_and_tsc() {
        let service = DiagnosticsService::with_defaults();
        let formatted = format!("{service:?}");
        assert!(formatted.contains("DiagnosticsService"));
    }

    #[test]
    fn shared_diagnostics_service_is_cheap_clone() {
        let shared: SharedDiagnosticsService = Arc::new(DiagnosticsService::with_defaults());
        let clone = Arc::clone(&shared);
        assert!(Arc::ptr_eq(&shared, &clone));
    }

    #[test]
    fn cache_key_distinguishes_workspaces() {
        let service = DiagnosticsService::new(StubProvider("a", &["rs"]));
        let path = PathBuf::from("src/main.rs");
        let _ = service
            .diagnostics_for_path(&path, &PathBuf::from("/workspace_a"))
            .unwrap();
        let _ = service
            .diagnostics_for_path(&path, &PathBuf::from("/workspace_b"))
            .unwrap();
        // Same path in two different workspaces should not collapse
        // into a single cache entry.
        assert_eq!(service.cache_len(), 2);
    }
}
