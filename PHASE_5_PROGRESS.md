# Phase 5: Plugin System - In Progress

## Overview
Implementing a robust plugin architecture for Lyrion Music Server with support for both native (shared library) and WASM plugins.

## Completed So Far ✅

### 1. Research & Analysis
- ✅ Analyzed Perl plugin system (`Slim/Utils/PluginManager.pm`)
- ✅ Studied plugin lifecycle: init, shutdown, event handling
- ✅ Examined RandomPlay plugin as reference implementation
- ✅ Understood manifest format and state management

### 2. Plugin API Design
**File**: `crates/lyrion-plugins/src/lib.rs` (264 lines)

**Core Trait**:
```rust
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    fn init(&mut self, context: &PluginContext) -> Result<(), String>;
    fn shutdown(&mut self);
    fn handle_command(&mut self, command: &str, params: &HashMap<String, String>) -> Option<String>;
    fn http_routes(&self) -> Vec<HttpRoute>;
    fn handle_http_request(&mut self, request: HttpRequest) -> Result<HttpResponse, String>;
    fn settings_schema(&self) -> Option<String>;
}
```

**Key Types**:
- `PluginManifest`: Metadata (name, version, author, dependencies)
- `PluginContext`: Server context (DB pool, config, preferences)
- `PluginConfig`: Server configuration passed to plugins
- `PluginError`: Comprehensive error types
- `HttpRoute`, `HttpRequest`, `HttpResponse`: HTTP integration
- `PluginState`: Lifecycle state tracking

### 3. Native Plugin Loader
**File**: `crates/lyrion-plugins/src/loader.rs` (215 lines)

**Features**:
- Load shared libraries (.so, .dylib, .dll) via libloading
- C ABI for stable plugin interface
- Plugin discovery from directory
- TOML manifest loading
- Safety wrapper around unsafe library loading

**Plugin Constructor Signature**:
```rust
extern "C" fn create_plugin() -> *mut dyn Plugin
```

**Functions**:
- `PluginLoader::load_plugin()`: Load shared library
- `PluginLoader::initialize_plugin()`: Initialize with context
- `discover_plugins()`: Find plugin.toml files
- `load_manifest()`: Parse TOML manifest

## In Progress 🚧

### Plugin Manager
**File**: `crates/lyrion-plugins/src/manager.rs` (to be created)

**Responsibilities**:
- Plugin registry management
- Dependency resolution
- Load order determination
- State persistence (enabled/disabled)
- Hot reload support
- Error recovery

### Plugin Registry
**File**: `crates/lyrion-plugins/src/registry.rs` (to be created)

**Responsibilities**:
- Track all loaded plugins
- Route HTTP requests to plugins
- Dispatch CLI commands
- Plugin lookup by name/ID
- Capability registration

## Still TODO ⏳

### 1. Complete Infrastructure

**Plugin Manager** (`manager.rs`):
```rust
pub struct PluginManager {
    loader: PluginLoader,
    plugins: HashMap<String, LoadedPlugin>,
    states: HashMap<String, PluginState>,
    config: PluginConfig,
}

impl PluginManager {
    pub fn new(config: PluginConfig) -> Self;
    pub fn discover_plugins(&mut self) -> PluginResult<Vec<String>>;
    pub fn load_plugin(&mut self, name: &str, context: &PluginContext) -> PluginResult<()>;
    pub fn unload_plugin(&mut self, name: &str) -> PluginResult<()>;
    pub fn reload_plugin(&mut self, name: &str, context: &PluginContext) -> PluginResult<()>;
    pub fn get_plugin(&self, name: &str) -> Option<&LoadedPlugin>;
    pub fn list_plugins(&self) -> Vec<&PluginManifest>;
}
```

**Plugin Registry** (`registry.rs`):
```rust
pub struct PluginRegistry {
    http_routes: HashMap<String, (String, String)>, // path -> (plugin_name, handler_id)
    cli_commands: HashMap<String, String>, // command -> plugin_name
}

impl PluginRegistry {
    pub fn register_http_routes(&mut self, plugin_name: &str, routes: Vec<HttpRoute>);
    pub fn route_http_request(&self, request: HttpRequest) -> Option<(String, String)>;
    pub fn register_cli_command(&mut self, plugin_name: &str, command: &str);
    pub fn find_command_handler(&self, command: &str) -> Option<&str>;
}
```

### 2. Example Plugin (RandomPlay)

**Directory**: `plugins/randomplay/`

**Files**:
- `src/lib.rs`: Plugin implementation
- `plugin.toml`: Manifest
- `Cargo.toml`: Build configuration

**Implementation**:
```rust
use lyrion_plugins::{Plugin, PluginContext, PluginManifest};

pub struct RandomPlayPlugin {
    db_pool: sqlx::SqlitePool,
    // ... state
}

impl Plugin for RandomPlayPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "RandomPlay".to_string(),
            version: "1.0.0".to_string(),
            author: "Lyrion Community".to_string(),
            description: Some("Random track playback".to_string()),
            min_server_version: Some("0.1.0".to_string()),
            dependencies: vec![],
            capabilities: vec!["cli".to_string()],
            enforced: false,
        }
    }

    fn init(&mut self, context: &PluginContext) -> Result<(), String> {
        self.db_pool = context.db_pool.clone();
        // Initialize...
        Ok(())
    }

    fn shutdown(&mut self) {
        // Cleanup...
    }

    fn handle_command(&mut self, command: &str, params: &HashMap<String, String>) -> Option<String> {
        match command {
            "randomplay" => {
                // Generate random playlist
                Some("Random playlist started".to_string())
            }
            _ => None,
        }
    }
}

#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(RandomPlayPlugin {
        db_pool: /* will be set in init */,
    });
    Box::into_raw(plugin)
}
```

### 3. Integration with Server

**File**: `crates/lyrion-server/src/main.rs`

**Changes Needed**:
```rust
use lyrion_plugins::{PluginManager, PluginContext, PluginConfig};

// In main():
let plugin_config = PluginConfig {
    server_version: env!("CARGO_PKG_VERSION").to_string(),
    data_dir: data_dir.clone(),
    plugin_dir: data_dir.join("plugins"),
    base_url: "http://localhost:9000".to_string(),
};

let mut plugin_manager = PluginManager::new(plugin_config);

let plugin_context = PluginContext {
    db_pool: db_pool.clone(),
    config: plugin_config.clone(),
    preferences: HashMap::new(),
};

// Discover and load plugins
plugin_manager.discover_plugins()?;
for plugin_name in plugin_manager.list_plugins() {
    plugin_manager.load_plugin(&plugin_name, &plugin_context)?;
}

// Register plugin HTTP routes with Axum
for route in plugin_manager.http_routes() {
    // Add to router
}

// On shutdown:
plugin_manager.shutdown_all();
```

### 4. WASM Support (Optional)

**File**: `crates/lyrion-plugins/src/wasm.rs`

**Features**:
- Load WASM modules with wasmtime
- Sandboxed execution
- Cross-platform plugins
- WASI support for file I/O

**Implementation**:
```rust
use wasmtime::*;

pub struct WasmPlugin {
    instance: Instance,
    store: Store<()>,
}

impl WasmPlugin {
    pub fn load(path: impl AsRef<Path>) -> PluginResult<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;

        Ok(Self { instance, store })
    }
}
```

### 5. Plugin Builder Tool

**Binary**: `lyrion-plugin-builder`

**Purpose**: Help developers create and test plugins

**Commands**:
```bash
# Create new plugin
lyrion-plugin-builder new my-plugin

# Build plugin
lyrion-plugin-builder build

# Test plugin
lyrion-plugin-builder test

# Package plugin for distribution
lyrion-plugin-builder package
```

## Dependencies Added

```toml
[dependencies]
sqlx = { workspace = true }
libloading = "0.8"
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
toml = "0.8"

# Optional WASM support
wasmtime = { version = "28.0", optional = true }

[features]
default = []
wasm = ["wasmtime"]
```

## Architecture Decisions

### 1. Trait-Based API
- **Pro**: Type-safe, compile-time checked
- **Pro**: Natural Rust idiom
- **Con**: Requires C ABI wrapper for dynamic loading

### 2. C ABI for Native Plugins
- **Pro**: Stable across Rust versions
- **Pro**: Can load C/C++ plugins
- **Con**: Requires unsafe code

### 3. Dual Loading Support (Native + WASM)
- **Pro**: Flexibility for plugin developers
- **Pro**: WASM provides sandboxing
- **Con**: More complexity in loader

### 4. Plugin Context Pattern
- **Pro**: Easy to extend without breaking plugins
- **Pro**: Clear separation of concerns
- **Con**: Cloning context may be expensive

## Testing Strategy

### Unit Tests
- Manifest parsing
- Plugin loading (with mock plugins)
- Dependency resolution
- State transitions

### Integration Tests
- Load real plugin from shared library
- Initialize and shutdown lifecycle
- Command handling
- HTTP routing

### Example Test Plugin
```rust
// tests/fixtures/test_plugin/src/lib.rs
struct TestPlugin;

impl Plugin for TestPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: "Test".to_string(),
            description: None,
            min_server_version: None,
            dependencies: vec![],
            capabilities: vec![],
            enforced: false,
        }
    }

    fn init(&mut self, _context: &PluginContext) -> Result<(), String> {
        Ok(())
    }

    fn shutdown(&mut self) {}
}

#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    Box::into_raw(Box::new(TestPlugin))
}
```

## Next Steps

1. **Complete Plugin Manager** (2-3 hours)
   - Implement full lifecycle management
   - Add state persistence
   - Handle dependencies

2. **Complete Plugin Registry** (1-2 hours)
   - HTTP route registration and dispatch
   - CLI command registration
   - Capability tracking

3. **Create Example Plugin** (2-3 hours)
   - Port RandomPlay from Perl
   - Demonstrate full API usage
   - Test with real server

4. **Integration Tests** (1-2 hours)
   - End-to-end plugin loading
   - Verify all lifecycle hooks
   - Test error handling

5. **Documentation** (1 hour)
   - Plugin development guide
   - API reference
   - Example plugins

## Estimated Time to Complete
- **Remaining work**: 8-12 hours
- **Current progress**: ~40% complete

## Success Criteria

- ✅ Plugin trait API defined
- ✅ Native plugin loader working
- ⏳ Plugin manager with full lifecycle
- ⏳ Example plugin (RandomPlay) working
- ⏳ HTTP routing to plugins
- ⏳ CLI command dispatch
- ⏳ Integration with server
- ⏳ Documentation complete

## Files Created/Modified

### Created
- `crates/lyrion-plugins/src/lib.rs` (264 lines) - Core API
- `crates/lyrion-plugins/src/loader.rs` (215 lines) - Native loader
- `crates/lyrion-plugins/Cargo.toml` - Dependencies

### To Create
- `crates/lyrion-plugins/src/manager.rs` - Plugin manager
- `crates/lyrion-plugins/src/registry.rs` - Plugin registry
- `crates/lyrion-plugins/src/wasm.rs` - WASM support (optional)
- `plugins/randomplay/` - Example plugin
- Integration tests
- Documentation

## Related Files (Perl Reference)
- `/data2/slimserver/Slim/Utils/PluginManager.pm` - Original implementation
- `/data2/slimserver/Slim/Plugin/Base.pm` - Base plugin class
- `/data2/slimserver/Slim/Plugin/RandomPlay/Plugin.pm` - Example plugin
