//! Plugin Manager
//!
//! Manages the lifecycle of all plugins, handles loading, initialization,
//! dependency resolution, and state persistence.

use crate::{
    loader::{discover_plugins, load_manifest, LoadedPlugin, PluginLoader},
    registry::PluginRegistry,
    Plugin, PluginConfig, PluginContext, PluginError, PluginManifest, PluginResult, PluginState,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// Plugin manager coordinating all plugin operations
pub struct PluginManager {
    /// Plugin loader for native plugins
    loader: PluginLoader,

    /// Registry for HTTP routes and CLI commands
    registry: PluginRegistry,

    /// Loaded plugins by name
    plugins: HashMap<String, LoadedPlugin>,

    /// Plugin states (for persistence)
    states: HashMap<String, PluginState>,

    /// Plugin configuration
    config: PluginConfig,

    /// Discovered but not loaded plugins
    discovered: Vec<PathBuf>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: PluginConfig) -> Self {
        Self {
            loader: PluginLoader::new(),
            registry: PluginRegistry::new(),
            plugins: HashMap::new(),
            states: HashMap::new(),
            config,
            discovered: Vec::new(),
        }
    }

    /// Discover all available plugins
    pub fn discover(&mut self) -> PluginResult<Vec<String>> {
        info!("Discovering plugins in: {}", self.config.plugin_dir.display());

        self.discovered = discover_plugins(&self.config.plugin_dir)?;

        let names: Vec<String> = self
            .discovered
            .iter()
            .filter_map(|path| {
                match load_manifest(path) {
                    Ok(manifest) => Some(manifest.name),
                    Err(e) => {
                        error!("Failed to load manifest from {:?}: {}", path, e);
                        None
                    }
                }
            })
            .collect();

        info!("Discovered {} plugins: {:?}", names.len(), names);

        Ok(names)
    }

    /// Load a plugin by manifest path
    ///
    /// # Safety
    /// Loads native code from shared library. Only load trusted plugins.
    pub unsafe fn load_plugin(
        &mut self,
        manifest_path: &PathBuf,
        context: &PluginContext,
    ) -> PluginResult<String> {
        let manifest = load_manifest(manifest_path)?;
        let plugin_name = manifest.name.clone();

        // Check if already loaded
        if self.plugins.contains_key(&plugin_name) {
            return Err(PluginError::AlreadyLoaded(plugin_name));
        }

        // Check dependencies
        for dep in &manifest.dependencies {
            if !self.plugins.contains_key(dep) {
                return Err(PluginError::MissingDependency(format!(
                    "Plugin {} requires {}",
                    plugin_name, dep
                )));
            }
        }

        // Find the shared library
        let plugin_dir = manifest_path.parent().ok_or_else(|| {
            PluginError::LoadError("Invalid manifest path".to_string())
        })?;

        let lib_name = self.find_shared_library(plugin_dir)?;

        // Load the plugin
        let mut plugin = self.loader.load_plugin(&lib_name)?;

        // Initialize
        PluginLoader::initialize_plugin(&mut plugin, context)?;

        // Register HTTP routes
        let routes = plugin.plugin_mut().http_routes();
        self.registry
            .register_http_routes(&plugin_name, routes);

        // Mark as loaded
        self.states.insert(plugin_name.clone(), PluginState::Loaded);

        // Store plugin
        self.plugins.insert(plugin_name.clone(), plugin);

        info!("Successfully loaded plugin: {}", plugin_name);

        Ok(plugin_name)
    }

    /// Find shared library in plugin directory
    fn find_shared_library(&self, plugin_dir: &std::path::Path) -> PluginResult<PathBuf> {
        // Look for .so, .dylib, or .dll
        let extensions = if cfg!(target_os = "linux") {
            vec!["so"]
        } else if cfg!(target_os = "macos") {
            vec!["dylib"]
        } else if cfg!(target_os = "windows") {
            vec!["dll"]
        } else {
            vec!["so", "dylib", "dll"]
        };

        for entry in std::fs::read_dir(plugin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_string_lossy().as_ref()) {
                    return Ok(path);
                }
            }
        }

        Err(PluginError::LoadError(format!(
            "No shared library found in {}",
            plugin_dir.display()
        )))
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> PluginResult<()> {
        let mut plugin = self
            .plugins
            .remove(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // Shutdown plugin
        PluginLoader::shutdown_plugin(&mut plugin);

        // Unregister routes
        self.registry.unregister_plugin(name);

        // Update state
        self.states.insert(name.to_string(), PluginState::Unloaded);

        info!("Unloaded plugin: {}", name);

        Ok(())
    }

    /// Reload a plugin
    pub unsafe fn reload_plugin(
        &mut self,
        name: &str,
        context: &PluginContext,
    ) -> PluginResult<()> {
        // Find manifest path
        let manifest_path = self
            .discovered
            .iter()
            .find(|path| {
                if let Ok(manifest) = load_manifest(path) {
                    manifest.name == name
                } else {
                    false
                }
            })
            .cloned()
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // Unload if loaded
        if self.plugins.contains_key(name) {
            self.unload_plugin(name)?;
        }

        // Reload
        self.load_plugin(&manifest_path, context)?;

        info!("Reloaded plugin: {}", name);

        Ok(())
    }

    /// Get a loaded plugin
    pub fn get_plugin(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    /// Get a mutable loaded plugin
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut LoadedPlugin> {
        self.plugins.get_mut(name)
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<&PluginManifest> {
        self.plugins.values().map(|p| p.manifest()).collect()
    }

    /// Get plugin registry for routing
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Get mutable plugin registry
    pub fn registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.registry
    }

    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) {
        info!("Shutting down all plugins");

        let plugin_names: Vec<String> = self.plugins.keys().cloned().collect();

        for name in plugin_names {
            if let Err(e) = self.unload_plugin(&name) {
                error!("Failed to unload plugin {}: {}", name, e);
            }
        }
    }

    /// Get plugin state
    pub fn get_state(&self, name: &str) -> Option<PluginState> {
        self.states.get(name).copied()
    }

    /// Load all discovered plugins
    pub unsafe fn load_all(&mut self, context: &PluginContext) -> PluginResult<Vec<String>> {
        let mut loaded = Vec::new();
        let manifest_paths = self.discovered.clone();

        // Sort by dependencies (simple topological sort)
        let sorted_paths = self.sort_by_dependencies(&manifest_paths)?;

        for path in sorted_paths {
            match self.load_plugin(&path, context) {
                Ok(name) => {
                    loaded.push(name);
                }
                Err(e) => {
                    error!("Failed to load plugin from {:?}: {}", path, e);
                }
            }
        }

        info!("Loaded {} plugins", loaded.len());

        Ok(loaded)
    }

    /// Sort plugins by dependencies (topological sort)
    fn sort_by_dependencies(&self, paths: &[PathBuf]) -> PluginResult<Vec<PathBuf>> {
        // Simple approach: load plugins with no dependencies first
        let mut sorted = Vec::new();
        let mut remaining: Vec<_> = paths.to_vec();

        while !remaining.is_empty() {
            let mut made_progress = false;

            let mut i = 0;
            while i < remaining.len() {
                let manifest = load_manifest(&remaining[i])?;

                // Check if all dependencies are satisfied
                let deps_satisfied = manifest.dependencies.iter().all(|dep| {
                    sorted.iter().any(|path| {
                        if let Ok(m) = load_manifest(path) {
                            m.name == *dep
                        } else {
                            false
                        }
                    })
                });

                if deps_satisfied {
                    sorted.push(remaining.remove(i));
                    made_progress = true;
                } else {
                    i += 1;
                }
            }

            if !made_progress && !remaining.is_empty() {
                // Circular dependency or missing dependency
                warn!("Circular or missing dependencies detected, loading remaining plugins anyway");
                sorted.extend(remaining);
                break;
            }
        }

        Ok(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let config = PluginConfig {
            server_version: "0.1.0".to_string(),
            data_dir: PathBuf::from("/tmp/data"),
            plugin_dir: PathBuf::from("/tmp/plugins"),
            base_url: "http://localhost:9000".to_string(),
        };

        let manager = PluginManager::new(config);
        assert_eq!(manager.plugins.len(), 0);
        assert_eq!(manager.list_plugins().len(), 0);
    }
}
