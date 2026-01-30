# Lyrion Plugin System

A robust plugin architecture for Lyrion Music Server supporting native (shared library) plugins with optional WASM support.

## Architecture Overview

The plugin system consists of:

1. **Plugin Trait**: Core interface all plugins must implement
2. **Plugin Loader**: Loads native shared libraries via libloading
3. **Plugin Manager**: Manages lifecycle, dependencies, and state
4. **Plugin Registry**: Routes HTTP requests and CLI commands to plugins

## Plugin Lifecycle

```
Discovery → Loading → Initialization → Running → Shutdown
```

1. **Discovery**: Find plugin.toml manifests in plugin directory
2. **Loading**: Load shared library (.so/.dylib/.dll)
3. **Initialization**: Call `init()` with PluginContext
4. **Running**: Handle commands and HTTP requests
5. **Shutdown**: Call `shutdown()` for cleanup

## Creating a Plugin

### 1. Directory Structure

```
plugins/your-plugin/
├── Cargo.toml
├── plugin.toml
└── src/
    └── lib.rs
```

### 2. Cargo.toml

```toml
[workspace]  # Important: prevents workspace conflicts

[package]
name = "lyrion-plugin-yourplugin"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Build as shared library

[dependencies]
lyrion-plugins = { path = "../../crates/lyrion-plugins" }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
serde = "1.0"
serde_json = "1.0"
tokio = { version = "1.40", features = ["full"] }
```

### 3. plugin.toml

```toml
[plugin]
name = "YourPlugin"
version = "1.0.0"
author = "Your Name"
description = "Brief description of what your plugin does"
min_server_version = "0.1.0"

[dependencies]
# List plugin dependencies (other plugin names)

[capabilities]
capabilities = ["cli", "http"]

[config]
enforced = false
```

### 4. Plugin Implementation

```rust
use lyrion_plugins::{
    HttpRequest, HttpResponse, HttpRoute, Plugin, PluginContext,
    PluginManifest,
};
use std::collections::HashMap;

/// Your plugin state
pub struct YourPlugin {
    db_pool: Option<sqlx::SqlitePool>,
    // Add your state here
}

impl YourPlugin {
    pub fn new() -> Self {
        Self {
            db_pool: None,
        }
    }
}

impl Plugin for YourPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "YourPlugin".to_string(),
            version: "1.0.0".to_string(),
            author: "Your Name".to_string(),
            description: Some("Your plugin description".to_string()),
            min_server_version: Some("0.1.0".to_string()),
            dependencies: vec![],
            capabilities: vec!["cli".to_string(), "http".to_string()],
            enforced: false,
        }
    }

    fn init(&mut self, context: &PluginContext) -> Result<(), String> {
        // Store database pool
        self.db_pool = Some(context.db_pool.clone());

        // Initialize your plugin

        Ok(())
    }

    fn shutdown(&mut self) {
        // Cleanup resources
        self.db_pool = None;
    }

    fn handle_command(
        &mut self,
        command: &str,
        params: &HashMap<String, String>,
    ) -> Option<String> {
        if command == "yourcommand" {
            // Handle command
            Some("Command executed".to_string())
        } else {
            None
        }
    }

    fn http_routes(&self) -> Vec<HttpRoute> {
        vec![
            HttpRoute {
                method: "GET".to_string(),
                path: "/plugins/yourplugin/endpoint".to_string(),
                handler_id: "main".to_string(),
            },
        ]
    }

    fn handle_http_request(
        &mut self,
        request: HttpRequest
    ) -> Result<HttpResponse, String> {
        // Handle HTTP request
        HttpResponse::json(serde_json::json!({
            "status": "ok"
        })).map_err(|e| e.to_string())
    }

    fn settings_schema(&self) -> Option<String> {
        Some(r#"{
            "type": "object",
            "properties": {
                "setting1": {
                    "type": "string",
                    "title": "Setting 1"
                }
            }
        }"#.to_string())
    }
}

/// Plugin constructor - REQUIRED
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(YourPlugin::new());
    Box::into_raw(plugin)
}
```

### 5. Build Plugin

```bash
cd plugins/yourplugin
cargo build --release
```

The compiled shared library will be at:
```
target/release/liblyrion_plugin_yourplugin.so  # Linux
target/release/liblyrion_plugin_yourplugin.dylib  # macOS
target/release/lyrion_plugin_yourplugin.dll  # Windows
```

## Using the Plugin System

### Server Integration

```rust
use lyrion_plugins::{PluginManager, PluginContext, PluginConfig};

// Create plugin configuration
let plugin_config = PluginConfig {
    server_version: "0.1.0".to_string(),
    data_dir: PathBuf::from("/path/to/data"),
    plugin_dir: PathBuf::from("/path/to/plugins"),
    base_url: "http://localhost:9000".to_string(),
};

// Create plugin manager
let mut plugin_manager = PluginManager::new(plugin_config);

// Create plugin context
let plugin_context = PluginContext {
    db_pool: db_pool.clone(),
    config: plugin_config.clone(),
    preferences: HashMap::new(),
};

// Discover plugins
plugin_manager.discover()?;

// Load all plugins (unsafe - only load trusted plugins)
unsafe {
    plugin_manager.load_all(&plugin_context)?;
}

// Get plugin registry for routing
let registry = plugin_manager.registry();

// Route HTTP request
if let Some((plugin_name, handler_id)) =
    registry.route_http_request("GET", "/plugins/randomplay/tracks") {
    // Get plugin and handle request
    if let Some(plugin) = plugin_manager.get_plugin_mut(plugin_name) {
        let response = plugin.plugin_mut().handle_http_request(request)?;
        // Send response
    }
}

// Handle CLI command
if let Some(plugin_name) = registry.find_command_handler("randomplay") {
    if let Some(plugin) = plugin_manager.get_plugin_mut(plugin_name) {
        let result = plugin.plugin_mut().handle_command("randomplay", &params);
        // Handle result
    }
}

// Shutdown all plugins
plugin_manager.shutdown_all();
```

## Plugin API Reference

### Plugin Trait

```rust
pub trait Plugin: Send + Sync {
    /// Get plugin manifest (name, version, dependencies)
    fn manifest(&self) -> PluginManifest;

    /// Initialize plugin with server context
    fn init(&mut self, context: &PluginContext) -> Result<(), String>;

    /// Shutdown plugin
    fn shutdown(&mut self);

    /// Handle CLI commands
    fn handle_command(
        &mut self,
        command: &str,
        params: &HashMap<String, String>
    ) -> Option<String>;

    /// Register HTTP routes
    fn http_routes(&self) -> Vec<HttpRoute>;

    /// Handle HTTP requests
    fn handle_http_request(
        &mut self,
        request: HttpRequest
    ) -> Result<HttpResponse, String>;

    /// Get plugin settings schema (JSON Schema)
    fn settings_schema(&self) -> Option<String>;
}
```

### PluginContext

```rust
pub struct PluginContext {
    /// SQLite connection pool
    pub db_pool: sqlx::SqlitePool,

    /// Plugin configuration from server
    pub config: PluginConfig,

    /// Plugin-specific preferences (key-value store)
    pub preferences: HashMap<String, String>,
}
```

### HttpRoute

```rust
pub struct HttpRoute {
    /// HTTP method (GET, POST, etc.)
    pub method: String,

    /// Route path (supports :param and * wildcards)
    pub path: String,

    /// Handler identifier
    pub handler_id: String,
}
```

Examples:
- `/plugins/myplugin/exact` - Exact match
- `/plugins/:name/status` - Matches `/plugins/foo/status`
- `/plugins/myplugin/*` - Matches `/plugins/myplugin/anything/here`

### HttpRequest

```rust
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub query: HashMap<String, String>,
}
```

### HttpResponse

```rust
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Create 200 OK response
    pub fn ok(body: impl Into<Vec<u8>>) -> Self;

    /// Create JSON response
    pub fn json(value: impl Serialize) -> Result<Self, serde_json::Error>;

    /// Create error response
    pub fn error(status: u16, message: impl Into<String>) -> Self;
}
```

## Example: RandomPlay Plugin

The RandomPlay plugin demonstrates all plugin features:

**Features**:
- Random track selection from database
- Multiple mix modes (tracks, albums, artists, years)
- CLI commands: `randomplay tracks`, `randomplay albums`
- HTTP endpoints:
  - `GET /plugins/randomplay/tracks?count=20`
  - `GET /plugins/randomplay/albums?count=5`
  - `GET /plugins/randomplay/artists?count=10`
  - `GET /plugins/randomplay/years?count=10`
- Settings schema for configuration

**Location**: `plugins/randomplay/`

**Build**:
```bash
cd plugins/randomplay
cargo build --release
```

**Usage**:
```bash
# CLI
randomplay --mode tracks --count 20

# HTTP
curl http://localhost:9000/plugins/randomplay/tracks?count=20
```

## Dependencies

Plugins can declare dependencies on other plugins. The PluginManager resolves dependencies and loads plugins in topological order.

```toml
[dependencies]
dependencies = ["PluginA", "PluginB"]
```

If dependencies are missing, the plugin will not load.

## Security

**IMPORTANT**: The plugin system uses `unsafe` code to load dynamic libraries. Only load trusted plugins from verified sources.

Plugins have full access to:
- Database
- File system
- Network
- All Rust standard library functions

Consider implementing:
- Plugin signing/verification
- Sandboxing (WASM plugins provide this)
- Permission system
- Code review process

## WASM Plugins (Optional)

WASM plugins provide sandboxed execution with limited capabilities.

Enable with feature flag:
```toml
[dependencies]
lyrion-plugins = { path = "...", features = ["wasm"] }
```

See `src/wasm.rs` for WASM implementation (when available).

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest() {
        let plugin = YourPlugin::new();
        let manifest = plugin.manifest();
        assert_eq!(manifest.name, "YourPlugin");
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_plugin_loading() {
    let plugin_config = PluginConfig { /* ... */ };
    let mut manager = PluginManager::new(plugin_config);

    unsafe {
        let plugin_name = manager.load_plugin(&manifest_path, &context)?;
        assert_eq!(plugin_name, "YourPlugin");
    }
}
```

## Troubleshooting

### Plugin Not Loading

1. Check `plugin.toml` syntax
2. Verify shared library was built: `ls target/release/*.so`
3. Check server logs for error messages
4. Verify dependencies are loaded first

### Symbol Not Found Error

Ensure `create_plugin` function has correct signature:
```rust
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin
```

### Runtime Errors

Use `Handle::current()` to access async runtime from sync plugin methods:
```rust
let result = tokio::runtime::Handle::current().block_on(async {
    // async code here
});
```

## Best Practices

1. **Error Handling**: Return detailed error messages from `init()`
2. **Resource Cleanup**: Free resources in `shutdown()`
3. **Database Queries**: Use parameterized queries to prevent SQL injection
4. **Async Code**: Use `Handle::current().block_on()` for async operations
5. **Logging**: Use `tracing` crate for structured logging
6. **Testing**: Write unit tests for core logic
7. **Documentation**: Document your plugin's commands and endpoints

## Future Enhancements

- [ ] Plugin marketplace/repository
- [ ] Hot reload support
- [ ] Plugin sandboxing/permissions
- [ ] WASM plugin support
- [ ] Plugin inter-communication
- [ ] Plugin UI framework
- [ ] Automatic plugin updates

## License

GPL-2.0 - Same as Lyrion Music Server
