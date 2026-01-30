//! Integration tests for plugin loading and lifecycle

use lyrion_plugins::{PluginConfig, PluginContext, PluginManager};
use std::collections::HashMap;
use std::path::PathBuf;

/// Test plugin manager creation
#[test]
fn test_manager_creation() {
    let config = PluginConfig {
        server_version: "0.1.0".to_string(),
        data_dir: PathBuf::from("/tmp/lyrion-test"),
        plugin_dir: PathBuf::from("/tmp/lyrion-test/plugins"),
        base_url: "http://localhost:9000".to_string(),
    };

    let manager = PluginManager::new(config);
    assert_eq!(manager.list_plugins().len(), 0);
}

/// Test plugin registry routing
#[test]
fn test_registry_routing() {
    use lyrion_plugins::{HttpRoute, PluginRegistry};

    let mut registry = PluginRegistry::new();

    // Register routes
    let routes = vec![
        HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/test/status".to_string(),
            handler_id: "status".to_string(),
        },
        HttpRoute {
            method: "POST".to_string(),
            path: "/plugins/test/action".to_string(),
            handler_id: "action".to_string(),
        },
    ];

    registry.register_http_routes("test_plugin", routes);

    // Test exact match
    assert_eq!(
        registry.route_http_request("GET", "/plugins/test/status"),
        Some(("test_plugin", "status"))
    );

    // Test method mismatch
    assert_eq!(
        registry.route_http_request("POST", "/plugins/test/status"),
        None
    );

    // Test path mismatch
    assert_eq!(
        registry.route_http_request("GET", "/plugins/test/unknown"),
        None
    );
}

/// Test pattern matching in registry
#[test]
fn test_registry_patterns() {
    use lyrion_plugins::{HttpRoute, PluginRegistry};

    let mut registry = PluginRegistry::new();

    // Register route with parameter
    registry.register_http_routes(
        "test_plugin",
        vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/:name/status".to_string(),
            handler_id: "status".to_string(),
        }],
    );

    // Should match any name
    assert_eq!(
        registry.route_http_request("GET", "/plugins/foo/status"),
        Some(("test_plugin", "status"))
    );

    assert_eq!(
        registry.route_http_request("GET", "/plugins/bar/status"),
        Some(("test_plugin", "status"))
    );
}

/// Test wildcard matching in registry
#[test]
fn test_registry_wildcard() {
    use lyrion_plugins::{HttpRoute, PluginRegistry};

    let mut registry = PluginRegistry::new();

    // Register route with wildcard
    registry.register_http_routes(
        "test_plugin",
        vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/test/*".to_string(),
            handler_id: "catch_all".to_string(),
        }],
    );

    // Should match anything after /plugins/test/
    assert_eq!(
        registry.route_http_request("GET", "/plugins/test/anything/here"),
        Some(("test_plugin", "catch_all"))
    );

    assert_eq!(
        registry.route_http_request("GET", "/plugins/test/deeply/nested/path"),
        Some(("test_plugin", "catch_all"))
    );
}

/// Test CLI command registration
#[test]
fn test_cli_commands() {
    use lyrion_plugins::PluginRegistry;

    let mut registry = PluginRegistry::new();

    registry.register_cli_command("test_plugin", "test-command");
    registry.register_cli_command("other_plugin", "other-command");

    assert_eq!(
        registry.find_command_handler("test-command"),
        Some("test_plugin")
    );

    assert_eq!(
        registry.find_command_handler("other-command"),
        Some("other_plugin")
    );

    assert_eq!(registry.find_command_handler("unknown-command"), None);
}

/// Test plugin unregistration
#[test]
fn test_plugin_unregister() {
    use lyrion_plugins::{HttpRoute, PluginRegistry};

    let mut registry = PluginRegistry::new();

    // Register routes and commands
    registry.register_http_routes(
        "test_plugin",
        vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/test/status".to_string(),
            handler_id: "status".to_string(),
        }],
    );
    registry.register_cli_command("test_plugin", "test-command");

    // Verify registered
    assert!(registry
        .route_http_request("GET", "/plugins/test/status")
        .is_some());
    assert!(registry.find_command_handler("test-command").is_some());

    // Unregister
    registry.unregister_plugin("test_plugin");

    // Verify unregistered
    assert!(registry
        .route_http_request("GET", "/plugins/test/status")
        .is_none());
    assert!(registry.find_command_handler("test-command").is_none());
}

/// Test capabilities tracking
#[test]
fn test_capabilities() {
    use lyrion_plugins::PluginRegistry;

    let mut registry = PluginRegistry::new();

    registry.register_capabilities(
        "test_plugin",
        vec!["http".to_string(), "cli".to_string()],
    );

    assert!(registry.has_capability("test_plugin", "http"));
    assert!(registry.has_capability("test_plugin", "cli"));
    assert!(!registry.has_capability("test_plugin", "wasm"));
    assert!(!registry.has_capability("other_plugin", "http"));
}

/// Test listing registered items
#[test]
fn test_listing() {
    use lyrion_plugins::{HttpRoute, PluginRegistry};

    let mut registry = PluginRegistry::new();

    registry.register_http_routes(
        "plugin1",
        vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/plugin1/test".to_string(),
            handler_id: "test".to_string(),
        }],
    );

    registry.register_http_routes(
        "plugin2",
        vec![HttpRoute {
            method: "POST".to_string(),
            path: "/plugins/plugin2/action".to_string(),
            handler_id: "action".to_string(),
        }],
    );

    registry.register_cli_command("plugin1", "cmd1");
    registry.register_cli_command("plugin2", "cmd2");

    let http_routes = registry.list_http_routes();
    assert_eq!(http_routes.len(), 2);

    let cli_commands = registry.list_cli_commands();
    assert_eq!(cli_commands.len(), 2);
}

/// Test that duplicate route registration warns but overwrites
#[test]
fn test_duplicate_route_warning() {
    use lyrion_plugins::{HttpRoute, PluginRegistry};

    let mut registry = PluginRegistry::new();

    // Register same route twice
    registry.register_http_routes(
        "plugin1",
        vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/test/status".to_string(),
            handler_id: "handler1".to_string(),
        }],
    );

    registry.register_http_routes(
        "plugin2",
        vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/test/status".to_string(),
            handler_id: "handler2".to_string(),
        }],
    );

    // Should use last registered
    assert_eq!(
        registry.route_http_request("GET", "/plugins/test/status"),
        Some(("plugin2", "handler2"))
    );
}
