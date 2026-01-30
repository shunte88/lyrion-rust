//! Native plugin loader using libloading
//!
//! Loads plugins from shared libraries (.so, .dylib, .dll) with a stable C ABI.

use crate::{Plugin, PluginContext, PluginError, PluginManifest, PluginResult};
use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Type alias for plugin constructor function
///
/// Plugins must export a C function with this signature:
/// ```c
/// extern "C" fn create_plugin() -> *mut dyn Plugin
/// ```
type PluginConstructor = unsafe extern "C" fn() -> *mut dyn Plugin;

/// Loaded plugin wrapper
pub struct LoadedPlugin {
    /// Plugin instance (trait object)
    plugin: Box<dyn Plugin>,

    /// Loaded library (kept alive to prevent unloading)
    _library: Arc<Library>,

    /// Plugin file path
    path: PathBuf,

    /// Plugin manifest
    manifest: PluginManifest,
}

impl LoadedPlugin {
    /// Get plugin manifest
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    /// Get mutable reference to plugin
    pub fn plugin_mut(&mut self) -> &mut Box<dyn Plugin> {
        &mut self.plugin
    }

    /// Get plugin file path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Plugin loader for native shared libraries
pub struct PluginLoader {
    /// Loaded libraries (kept alive)
    _libraries: Vec<Arc<Library>>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            _libraries: Vec::new(),
        }
    }

    /// Load a plugin from a shared library file
    ///
    /// # Safety
    /// This function loads arbitrary native code from a shared library.
    /// Only load plugins from trusted sources.
    pub unsafe fn load_plugin(&mut self, path: impl AsRef<Path>) -> PluginResult<LoadedPlugin> {
        let path = path.as_ref();

        info!("Loading plugin from: {}", path.display());

        // Load the shared library
        let library = Library::new(path).map_err(|e| {
            PluginError::LoadError(format!("Failed to load library: {}", e))
        })?;

        let library = Arc::new(library);

        // Look for the plugin constructor function
        let constructor: Symbol<PluginConstructor> = library
            .get(b"create_plugin")
            .map_err(|e| {
                PluginError::LoadError(format!(
                    "Plugin does not export create_plugin function: {}",
                    e
                ))
            })?;

        // Call the constructor to get the plugin instance
        let plugin_ptr = constructor();

        if plugin_ptr.is_null() {
            return Err(PluginError::LoadError(
                "create_plugin returned null".to_string(),
            ));
        }

        // Convert raw pointer to Box<dyn Plugin>
        let plugin = Box::from_raw(plugin_ptr);

        // Get the manifest from the plugin
        let manifest = plugin.manifest();

        debug!(
            "Plugin loaded: {} v{} by {}",
            manifest.name, manifest.version, manifest.author
        );

        // Keep library alive
        self._libraries.push(library.clone());

        Ok(LoadedPlugin {
            plugin,
            _library: library,
            path: path.to_path_buf(),
            manifest,
        })
    }

    /// Initialize a loaded plugin with context
    pub fn initialize_plugin(
        plugin: &mut LoadedPlugin,
        context: &PluginContext,
    ) -> Result<(), String> {
        info!("Initializing plugin: {}", plugin.manifest.name);

        plugin.plugin.init(context)
    }

    /// Shutdown a plugin
    pub fn shutdown_plugin(plugin: &mut LoadedPlugin) {
        info!("Shutting down plugin: {}", plugin.manifest.name);
        plugin.plugin.shutdown();
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Discover plugin manifests in a directory
///
/// Looks for `plugin.toml` files in the plugins directory.
pub fn discover_plugins(plugin_dir: impl AsRef<Path>) -> PluginResult<Vec<PathBuf>> {
    let plugin_dir = plugin_dir.as_ref();

    if !plugin_dir.exists() {
        return Ok(vec![]);
    }

    let mut manifests = Vec::new();

    for entry in std::fs::read_dir(plugin_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Look for plugin.toml in subdirectory
            let manifest_path = path.join("plugin.toml");
            if manifest_path.exists() {
                manifests.push(manifest_path);
            }
        }
    }

    Ok(manifests)
}

/// Load plugin manifest from TOML file
pub fn load_manifest(path: impl AsRef<Path>) -> PluginResult<PluginManifest> {
    let content = std::fs::read_to_string(path.as_ref())?;
    let manifest: PluginManifest = toml::from_str(&content)
        .map_err(|e| PluginError::InvalidManifest(format!("Invalid TOML: {}", e)))?;

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parsing() {
        let toml = r#"
            name = "Test Plugin"
            version = "1.0.0"
            author = "Test Author"
            description = "A test plugin"
            min_server_version = "0.1.0"
            dependencies = ["plugin-a", "plugin-b"]
            capabilities = ["cli", "http"]
            enforced = false
        "#;

        let manifest: PluginManifest = toml::from_str(toml).unwrap();

        assert_eq!(manifest.name, "Test Plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.author, "Test Author");
        assert_eq!(manifest.description, Some("A test plugin".to_string()));
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.capabilities.len(), 2);
        assert!(!manifest.enforced);
    }
}
