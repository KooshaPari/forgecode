use std::path::PathBuf;
use std::sync::LazyLock;

use config::ConfigBuilder;
use config::builder::DefaultState;

use crate::ForgeConfig;
use crate::legacy::LegacyConfig;

/// Loads all `.env` files found while walking up from the current working
/// directory to the root, with priority given to closer (lower) directories.
/// Executed at most once per process.
static LOAD_DOT_ENV: LazyLock<()> = LazyLock::new(|| {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut paths = vec![];
    let mut current = PathBuf::new();

    for component in cwd.components() {
        current.push(component);
        paths.push(current.clone());
    }

    paths.reverse();

    for path in paths {
        let env_file = path.join(".env");
        if env_file.is_file() {
            dotenvy::from_path(&env_file).ok();
        }
    }
});

/// Caches base-path resolution for the process lifetime.
static BASE_PATH: LazyLock<PathBuf> = LazyLock::new(ConfigReader::resolve_base_path);

/// Merges [`ForgeConfig`] from layered sources using a builder pattern.
#[derive(Default)]
pub struct ConfigReader {
    builder: ConfigBuilder<DefaultState>,
}

impl ConfigReader {
    /// Returns the path to the legacy JSON config file
    /// (`~/.forge/.config.json`).
    pub fn config_legacy_path() -> PathBuf {
        Self::base_path().join(".config.json")
    }

    /// Returns the path to the primary TOML config file
    /// (`~/.forge/.forge.toml`).
    pub fn config_path() -> PathBuf {
        Self::base_path().join(".forge.toml")
    }

    /// Returns the base directory for all Forge config files.
    ///
    /// Resolution order:
    /// 1. `FORGE_CONFIG` environment variable, if set.
    /// 2. For the canonical `helioslite` binary (Gate 5):
    ///    - `~/.helioslite` (canonical data dir), if that directory exists.
    ///    - `~/.forge` (legacy), if that directory exists — the data is
    ///      honored and read in place and is never auto-migrated.
    ///    - `~/.helioslite` as the default for fresh installs.
    /// 3. For the legacy `forge` / `forge-dev` binaries:
    ///    - `~/forge` (historical legacy path), if that directory exists.
    ///    - `~/.forge` as the default path.
    pub fn base_path() -> PathBuf {
        BASE_PATH.clone()
    }

    fn resolve_base_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self::resolve_base_path_for(home, Self::is_helioslite_binary())
    }

    /// Returns the executable's file stem (e.g. `helioslite`, `forge`) using
    /// the same argv[0] detection the CLI name uses in `forge_main::main`.
    fn executable_name() -> Option<String> {
        std::env::args_os().next().and_then(|arg| {
            PathBuf::from(arg)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
    }

    /// Returns `true` when the running binary is the canonical `helioslite`.
    ///
    /// The legacy `forge` / `forge-dev` aliases keep the historical
    /// `~/.forge` resolution so they never collide with the canonical data
    /// dir.
    fn is_helioslite_binary() -> bool {
        Self::executable_name()
            .map(|name| name.eq_ignore_ascii_case("helioslite"))
            .unwrap_or(false)
    }

    /// Resolves the base path for a given home directory and binary identity.
    ///
    /// Kept as a pure helper so both resolution branches are unit-testable.
    fn resolve_base_path_for(home: PathBuf, canonical_binary: bool) -> PathBuf {
        if let Ok(path) = std::env::var("FORGE_CONFIG") {
            return PathBuf::from(path);
        }

        if canonical_binary {
            // Canonical heliosLite data dir. Once it exists it wins; otherwise
            // the legacy ~/.forge data is honored and read in place (never
            // auto-migrated), so existing installs are not disrupted.
            let canonical = home.join(".helioslite");
            if canonical.exists() {
                tracing::info!("Using canonical heliosLite path");
                return canonical;
            }

            let legacy = home.join(".forge");
            if legacy.exists() {
                tracing::info!("Using legacy heliosLite path");
                return legacy;
            }

            tracing::info!("Using canonical heliosLite path (new)");
            return canonical;
        }

        // Legacy forge/forge-dev binaries keep the historical resolution:
        // prefer ~/forge when present, otherwise default to ~/.forge.
        let legacy = home.join("forge");
        if legacy.exists() {
            tracing::info!("Using legacy path");
            return legacy;
        }

        tracing::info!("Using new path");
        home.join(".forge")
    }

    /// Adds the provided TOML string as a config source without touching the
    /// filesystem.
    pub fn read_toml(mut self, contents: &str) -> Self {
        self.builder = self
            .builder
            .add_source(config::File::from_str(contents, config::FileFormat::Toml));

        self
    }

    /// Adds the embedded default config (`../.forge.toml`) as a source.
    pub fn read_defaults(self) -> Self {
        let defaults = include_str!("../.forge.toml");

        self.read_toml(defaults)
    }

    /// Adds `FORGE_`-prefixed environment variables as a config source.
    pub fn read_env(mut self) -> Self {
        self.builder = self.builder.add_source(
            config::Environment::with_prefix("FORGE")
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true)
                .list_separator(",")
                .with_list_parse_key("retry.status_codes")
                .with_list_parse_key("http.root_cert_paths"),
        );

        self
    }

    /// Builds and deserializes all accumulated sources into a [`ForgeConfig`].
    ///
    /// Triggers `.env` file loading (at most once per process) by walking up
    /// the directory tree from the current working directory, with closer
    /// directories taking priority.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be built or deserialized.
    pub fn build(self) -> crate::Result<ForgeConfig> {
        *LOAD_DOT_ENV;
        let config = self.builder.build()?;
        Ok(config.try_deserialize::<ForgeConfig>()?)
    }

    /// Adds `~/.forge/.forge.toml` as a config source, silently skipping if
    /// absent.
    pub fn read_global(mut self) -> Self {
        let path = Self::config_path();
        self.builder = self
            .builder
            .add_source(config::File::from(path).required(false));
        self
    }

    /// Reads `~/.forge/.config.json` (legacy format) and adds it as a source,
    /// silently skipping errors.
    pub fn read_legacy(self) -> Self {
        let content = LegacyConfig::read(&Self::config_legacy_path());
        if let Ok(content) = content {
            self.read_toml(&content)
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::ModelConfig;

    /// Serializes tests that mutate environment variables to prevent races.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Holds env vars set for a test's duration and removes them on drop, while
    /// holding [`ENV_MUTEX`].
    struct EnvGuard {
        keys: Vec<&'static str>,
        _lock: MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        /// Acquires [`ENV_MUTEX`], sets each `(key, value)` pair in the
        /// environment, and removes each key in `remove` if present. All
        /// set keys are cleaned up on drop.
        #[must_use]
        fn set(pairs: &[(&'static str, &str)]) -> Self {
            Self::set_and_remove(pairs, &[])
        }

        /// Like [`set`] but also removes the listed keys before the test runs.
        #[must_use]
        fn set_and_remove(pairs: &[(&'static str, &str)], remove: &[&'static str]) -> Self {
            let lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
            let keys = pairs.iter().map(|(k, _)| *k).collect();
            for key in remove {
                unsafe { std::env::remove_var(key) };
            }
            for (key, value) in pairs {
                unsafe { std::env::set_var(key, value) };
            }
            Self { keys, _lock: lock }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for key in &self.keys {
                unsafe { std::env::remove_var(key) };
            }
        }
    }

    #[test]
    fn test_base_path_uses_forge_config_env_var() {
        let _guard = EnvGuard::set(&[("FORGE_CONFIG", "/custom/forge/dir")]);
        let actual = ConfigReader::resolve_base_path();
        let expected = PathBuf::from("/custom/forge/dir");
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_base_path_falls_back_to_home_dir_when_env_var_absent() {
        // Hold the env mutex and ensure FORGE_CONFIG is absent so this test
        // cannot race with test_base_path_uses_forge_config_env_var.
        let _guard = EnvGuard::set_and_remove(&[], &["FORGE_CONFIG"]);

        let actual = ConfigReader::resolve_base_path();
        // Without FORGE_CONFIG set the path must be either "forge" (legacy,
        // preferred when ~/forge exists) or ".forge" (default new path).
        let name = actual.file_name().unwrap();
        assert!(
            name == "forge" || name == ".forge",
            "Expected base_path to end with 'forge' or '.forge', got: {:?}",
            name
        );
    }

    #[test]
    fn test_base_path_canonical_binary_defaults_to_helioslite() {
        let _guard = EnvGuard::set_and_remove(&[], &["FORGE_CONFIG"]);
        let home = PathBuf::from("/home/nonexistent-user");
        let actual = ConfigReader::resolve_base_path_for(home.clone(), true);
        assert_eq!(actual, home.join(".helioslite"));
    }

    #[test]
    fn test_base_path_canonical_binary_honors_legacy_forge_dir() {
        let _guard = EnvGuard::set_and_remove(&[], &["FORGE_CONFIG"]);
        let home = std::env::temp_dir().join(format!("hl-gate5-legacy-{}", std::process::id()));
        std::fs::create_dir_all(home.join(".forge")).unwrap();
        let actual = ConfigReader::resolve_base_path_for(home.clone(), true);
        std::fs::remove_dir_all(&home).ok();
        assert_eq!(actual, home.join(".forge"));
    }

    #[test]
    fn test_base_path_canonical_binary_prefers_existing_helioslite_dir() {
        let _guard = EnvGuard::set_and_remove(&[], &["FORGE_CONFIG"]);
        let home = std::env::temp_dir().join(format!("hl-gate5-canon-{}", std::process::id()));
        std::fs::create_dir_all(home.join(".helioslite")).unwrap();
        std::fs::create_dir_all(home.join(".forge")).unwrap();
        let actual = ConfigReader::resolve_base_path_for(home.clone(), true);
        std::fs::remove_dir_all(&home).ok();
        assert_eq!(actual, home.join(".helioslite"));
    }

    #[test]
    fn test_base_path_legacy_binary_defaults_to_dot_forge() {
        let _guard = EnvGuard::set_and_remove(&[], &["FORGE_CONFIG"]);
        let home = PathBuf::from("/home/nonexistent-user");
        let actual = ConfigReader::resolve_base_path_for(home.clone(), false);
        assert_eq!(actual, home.join(".forge"));
    }

    #[test]
    fn test_read_parses_without_error() {
        let actual = ConfigReader::default().read_defaults().build();
        assert!(actual.is_ok(), "read() failed: {:?}", actual.err());
    }

    #[test]
    fn test_legacy_layer_does_not_overwrite_defaults() {
        // Simulate what `read_legacy` does: serialize a ForgeConfig that only
        // carries session/commit/suggest (all other fields are None) and layer
        // it on top of the embedded defaults. The default values must survive.
        let legacy = ForgeConfig {
            session: Some(ModelConfig {
                provider_id: "anthropic".to_string(),
                model_id: "claude-3".to_string(),
            }),
            ..Default::default()
        };
        let legacy_toml = toml_edit::ser::to_string_pretty(&legacy).unwrap();

        let actual = ConfigReader::default()
            // Read legacy first and then defaults
            .read_toml(&legacy_toml)
            .read_defaults()
            .build()
            .unwrap();

        // Session should come from the legacy layer
        assert_eq!(
            actual.session,
            Some(ModelConfig {
                provider_id: "anthropic".to_string(),
                model_id: "claude-3".to_string(),
            })
        );

        // Default values from .forge.toml must be retained, not reset to zero
        assert_eq!(actual.max_parallel_file_reads, 64);
        assert_eq!(actual.max_read_lines, 2000);
        assert_eq!(actual.tool_timeout_secs, 300);
        assert_eq!(actual.max_search_lines, 1000);
        assert_eq!(actual.tool_supported, true);
    }

    #[test]
    fn test_read_session_from_env_vars() {
        let _guard = EnvGuard::set(&[
            ("FORGE_SESSION__PROVIDER_ID", "fake-provider"),
            ("FORGE_SESSION__MODEL_ID", "fake-model"),
        ]);

        let actual = ConfigReader::default()
            .read_defaults()
            .read_env()
            .build()
            .unwrap();

        let expected = Some(ModelConfig {
            provider_id: "fake-provider".to_string(),
            model_id: "fake-model".to_string(),
        });
        assert_eq!(actual.session, expected);
    }

    #[test]
    fn test_use_forge_committer_defaults_to_true() {
        let actual = ConfigReader::default().read_defaults().build().unwrap();

        assert_eq!(actual.use_forge_committer, true);
    }

    #[test]
    fn test_use_forge_committer_can_be_disabled() {
        let toml = "use_forge_committer = false\n";

        let actual = ConfigReader::default()
            .read_defaults()
            .read_toml(toml)
            .build()
            .unwrap();

        assert_eq!(actual.use_forge_committer, false);
    }
}
