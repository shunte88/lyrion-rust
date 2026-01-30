//! Lyrion Plugin System
//!
//! Provides a trait-based plugin architecture for extending Lyrion Music Server.
//! Supports both native (shared library) plugins and WASM plugins.
//!
//! # Plugin Lifecycle
//! 1. Discovery: Find plugin manifests
//! 2. Loading: Load shared library or WASM module
//! 3. Initialization: Call `init()` with plugin context
//! 4. Running: Handle requests and events
//! 5. Shutdown: Call `shutdown()` for cleanup
//!
//! # Example Native Plugin
//! ```no_run
//! use lyrion_plugins::{Plugin, PluginContext, PluginManifest};
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn manifest(&self) -> PluginManifest {
//!         PluginManifest {
//!             name: "My Plugin".to_string(),
//!             version: "1.0.0".to_string(),
//!             author: "Your Name".to_string(),
//!             description: Some("Does amazing things".to_string()),
//!         }
//!     }
//!
//!     fn init(&mut self, context: &PluginContext) -> Result<(), String> {
//!         // Initialize plugin
//!         Ok(())
//!     }
//!
//!     fn shutdown(&mut self) {
//!         // Cleanup
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod loader;
pub mod manager;
pub mod registry;
pub mod perl;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export main types for convenience
pub use manager::PluginManager;
pub use registry::PluginRegistry;

/// Plugin manifest describing plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,

    /// Semantic version (e.g., "1.0.0")
    pub version: String,

    /// Plugin author
    pub author: String,

    /// Optional description
    pub description: Option<String>,

    /// Minimum required server version
    pub min_server_version: Option<String>,

    /// Plugin dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Plugin capabilities/features
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Whether this plugin is enforced (cannot be disabled)
    #[serde(default)]
    pub enforced: bool,
}

/// Context provided to plugins for accessing server functionality
#[derive(Clone)]
pub struct PluginContext {
    /// Database connection pool
    pub db_pool: sqlx::SqlitePool,

    /// Server configuration
    pub config: PluginConfig,

    /// Plugin-specific preferences (key-value store)
    pub preferences: HashMap<String, String>,
}

/// Plugin configuration from server
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// Server version
    pub server_version: String,

    /// Server data directory
    pub data_dir: PathBuf,

    /// Plugin directory
    pub plugin_dir: PathBuf,

    /// HTTP server base URL
    pub base_url: String,
}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin error types
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitializationError(String),

    #[error("Plugin load failed: {0}")]
    LoadError(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),

    #[error("Dependency missing: {0}")]
    MissingDependency(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl From<String> for PluginError {
    fn from(s: String) -> Self {
        PluginError::InitializationError(s)
    }
}

/// Core plugin trait
///
/// All plugins must implement this trait to be loaded by the plugin manager.
pub trait Plugin: Send + Sync {
    /// Get plugin manifest
    fn manifest(&self) -> PluginManifest;

    /// Initialize plugin with server context
    ///
    /// Called once when plugin is loaded. Return `Err` to prevent plugin from loading.
    fn init(&mut self, context: &PluginContext) -> Result<(), String>;

    /// Shutdown plugin
    ///
    /// Called when plugin is unloaded or server is shutting down.
    fn shutdown(&mut self);

    /// Handle custom CLI commands
    ///
    /// Return `Some(response)` if command was handled, `None` if not recognized.
    fn handle_command(
        &mut self,
        _command: &str,
        _params: &HashMap<String, String>,
    ) -> Option<String> {
        None
    }

    /// Register HTTP routes (optional)
    ///
    /// Return list of (method, path, handler_id) tuples.
    fn http_routes(&self) -> Vec<HttpRoute> {
        vec![]
    }

    /// Handle HTTP request (optional)
    ///
    /// Called when a registered route is matched.
    fn handle_http_request(
        &mut self,
        _request: HttpRequest,
    ) -> Result<HttpResponse, String> {
        Err("HTTP handler not implemented".to_string())
    }

    /// Get plugin settings schema (optional)
    ///
    /// Return JSON schema for plugin settings UI.
    fn settings_schema(&self) -> Option<String> {
        None
    }
}

/// HTTP route registration
#[derive(Debug, Clone)]
pub struct HttpRoute {
    pub method: String,
    pub path: String,
    pub handler_id: String,
}

/// HTTP request passed to plugin
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub query: HashMap<String, String>,
}

/// HTTP response from plugin
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: body.into(),
        }
    }

    pub fn json(value: impl serde::Serialize) -> Result<Self, serde_json::Error> {
        let body = serde_json::to_vec(&value)?;
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(Self {
            status: 200,
            headers,
            body,
        })
    }

    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: message.into().into_bytes(),
        }
    }
}

/// Plugin state for tracking lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin discovered but not loaded
    Discovered,
    /// Plugin loaded successfully
    Loaded,
    /// Plugin initialization failed
    Failed,
    /// Plugin disabled by user
    Disabled,
    /// Plugin unloaded
    Unloaded,
}
