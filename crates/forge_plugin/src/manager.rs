use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::{PluginConfig, PluginMetadata};

/// Manages the lifecycle of plugins.
pub struct PluginManager {
    config_dir: PathBuf,
    registry: PluginRegistry,
}

impl PluginManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir, registry: PluginRegistry::new() }
    }

    /// Load plugins from the configuration directory.
    /// Expects JSON files in the directory.
    pub async fn load_plugins(&mut self) -> Result<()> {
        if !self.config_dir.exists() {
            tracing::info!(
                "Plugin config directory does not exist, creating: {:?}",
                self.config_dir
            );
            fs::create_dir_all(&self.config_dir).await?;
            return Ok(());
        }

        let mut entries = fs::read_dir(&self.config_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                match self.load_plugin_from_file(&path).await {
                    Ok(config) => {
                        if config.enabled {
                            tracing::info!(
                                "Loaded plugin config: {} v{}",
                                config.name,
                                config.version
                            );
                            self.registry.add_config(config);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to load plugin from {:?}: {}", path, e);
                    }
                }
            }
        }
        Ok(())
    }

    async fn load_plugin_from_file(&self, path: &Path) -> Result<PluginConfig> {
        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read plugin config: {:?}", path))?;

        let config: PluginConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse plugin config: {:?}", path))?;

        Ok(config)
    }

    /// Initialize all loaded plugins.
    pub async fn init_all(&mut self) -> Result<()> {
        // Note: In a real dynamic loading scenario, we would load .so/.dll here
        // based on config.path. For now, we manage the registered instances.
        tracing::info!("Initializing {} plugins...", self.registry.len());
        // If we had actual plugin instances:
        // for plugin in self.registry.plugins_mut() {
        //     plugin.init().await?;
        // }
        Ok(())
    }

    /// Shutdown all loaded plugins.
    pub async fn shutdown_all(&mut self) -> Result<()> {
        tracing::info!("Shutting down plugins...");
        // for plugin in self.registry.plugins_mut().rev() {
        //     plugin.shutdown().await?;
        // }
        Ok(())
    }

    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }
}

/// Registry for managing plugin metadata and instances.
pub struct PluginRegistry {
    configs: Vec<PluginConfig>,
    // In a real implementation, this would hold loaded Box<dyn Plugin> instances
    // plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { configs: Vec::new() }
    }

    pub fn add_config(&mut self, config: PluginConfig) {
        self.configs.push(config);
    }

    pub fn len(&self) -> usize {
        self.configs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    pub fn get_metadata(&self) -> Vec<PluginMetadata> {
        self.configs
            .iter()
            .map(|c| PluginMetadata {
                name: c.name.clone(),
                version: c.version.clone(),
                enabled: c.enabled,
            })
            .collect()
    }

    pub fn get_config(&self, name: &str) -> Option<&PluginConfig> {
        self.configs.iter().find(|c| c.name == name)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
