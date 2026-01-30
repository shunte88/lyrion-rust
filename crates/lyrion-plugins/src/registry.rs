//! Plugin Registry
//!
//! Central registry for plugin routes, commands, and capabilities.
//! Routes HTTP requests and CLI commands to the appropriate plugin.

use crate::HttpRoute;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Plugin registry for routing and dispatch
pub struct PluginRegistry {
    /// HTTP routes: (method, path) -> (plugin_name, handler_id)
    http_routes: HashMap<(String, String), (String, String)>,

    /// CLI commands: command -> plugin_name
    cli_commands: HashMap<String, String>,

    /// Plugin capabilities: plugin_name -> capabilities
    capabilities: HashMap<String, Vec<String>>,
}

impl PluginRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            http_routes: HashMap::new(),
            cli_commands: HashMap::new(),
            capabilities: HashMap::new(),
        }
    }

    /// Register HTTP routes for a plugin
    pub fn register_http_routes(&mut self, plugin_name: &str, routes: Vec<HttpRoute>) {
        for route in routes {
            let key = (route.method.clone(), route.path.clone());
            let value = (plugin_name.to_string(), route.handler_id.clone());

            if self.http_routes.contains_key(&key) {
                warn!(
                    "HTTP route {} {} already registered, overwriting",
                    route.method, route.path
                );
            }

            self.http_routes.insert(key, value);

            debug!(
                "Registered HTTP route: {} {} -> {}::{}",
                route.method, route.path, plugin_name, route.handler_id
            );
        }
    }

    /// Find the plugin and handler for an HTTP request
    pub fn route_http_request(
        &self,
        method: &str,
        path: &str,
    ) -> Option<(&str, &str)> {
        // Try exact match first
        if let Some((plugin_name, handler_id)) = self.http_routes.get(&(method.to_string(), path.to_string())) {
            return Some((plugin_name.as_str(), handler_id.as_str()));
        }

        // Try pattern matching (e.g., /plugins/:name/*)
        for ((route_method, route_path), (plugin_name, handler_id)) in &self.http_routes {
            if route_method == method && Self::matches_pattern(route_path, path) {
                return Some((plugin_name.as_str(), handler_id.as_str()));
            }
        }

        None
    }

    /// Check if a path matches a route pattern
    fn matches_pattern(pattern: &str, path: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            // Check for wildcard at end
            if let Some(last) = pattern_parts.last() {
                if *last == "*" && path_parts.len() >= pattern_parts.len() - 1 {
                    // Wildcard match
                    return Self::matches_pattern_parts(&pattern_parts[..pattern_parts.len() - 1], &path_parts);
                }
            }
            return false;
        }

        Self::matches_pattern_parts(&pattern_parts, &path_parts)
    }

    /// Match pattern parts (handles :param syntax)
    fn matches_pattern_parts(pattern_parts: &[&str], path_parts: &[&str]) -> bool {
        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if pattern_part.starts_with(':') {
                // Parameter, always matches
                continue;
            } else if *pattern_part != *path_part {
                return false;
            }
        }
        true
    }

    /// Register a CLI command for a plugin
    pub fn register_cli_command(&mut self, plugin_name: &str, command: &str) {
        if self.cli_commands.contains_key(command) {
            warn!("CLI command {} already registered, overwriting", command);
        }

        self.cli_commands
            .insert(command.to_string(), plugin_name.to_string());

        debug!("Registered CLI command: {} -> {}", command, plugin_name);
    }

    /// Find the plugin for a CLI command
    pub fn find_command_handler(&self, command: &str) -> Option<&str> {
        self.cli_commands.get(command).map(|s| s.as_str())
    }

    /// Register plugin capabilities
    pub fn register_capabilities(&mut self, plugin_name: &str, capabilities: Vec<String>) {
        self.capabilities
            .insert(plugin_name.to_string(), capabilities);
    }

    /// Get plugin capabilities
    pub fn get_capabilities(&self, plugin_name: &str) -> Option<&[String]> {
        self.capabilities.get(plugin_name).map(|v| v.as_slice())
    }

    /// Check if a plugin has a capability
    pub fn has_capability(&self, plugin_name: &str, capability: &str) -> bool {
        self.capabilities
            .get(plugin_name)
            .map(|caps| caps.contains(&capability.to_string()))
            .unwrap_or(false)
    }

    /// Unregister all routes and commands for a plugin
    pub fn unregister_plugin(&mut self, plugin_name: &str) {
        // Remove HTTP routes
        self.http_routes
            .retain(|_, (name, _)| name != plugin_name);

        // Remove CLI commands
        self.cli_commands.retain(|_, name| name != plugin_name);

        // Remove capabilities
        self.capabilities.remove(plugin_name);

        debug!("Unregistered all routes for plugin: {}", plugin_name);
    }

    /// List all registered HTTP routes
    pub fn list_http_routes(&self) -> Vec<(String, String, String)> {
        self.http_routes
            .iter()
            .map(|((method, path), (plugin, _))| {
                (method.clone(), path.clone(), plugin.clone())
            })
            .collect()
    }

    /// List all registered CLI commands
    pub fn list_cli_commands(&self) -> Vec<(String, String)> {
        self.cli_commands
            .iter()
            .map(|(cmd, plugin)| (cmd.clone(), plugin.clone()))
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_route_registration() {
        let mut registry = PluginRegistry::new();

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

        assert_eq!(
            registry.route_http_request("GET", "/plugins/test/status"),
            Some(("test_plugin", "status"))
        );

        assert_eq!(
            registry.route_http_request("POST", "/plugins/test/action"),
            Some(("test_plugin", "action"))
        );

        assert_eq!(registry.route_http_request("GET", "/plugins/test/unknown"), None);
    }

    #[test]
    fn test_pattern_matching() {
        let mut registry = PluginRegistry::new();

        let routes = vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/:name/status".to_string(),
            handler_id: "status".to_string(),
        }];

        registry.register_http_routes("test_plugin", routes);

        assert_eq!(
            registry.route_http_request("GET", "/plugins/foo/status"),
            Some(("test_plugin", "status"))
        );

        assert_eq!(
            registry.route_http_request("GET", "/plugins/bar/status"),
            Some(("test_plugin", "status"))
        );
    }

    #[test]
    fn test_wildcard_matching() {
        let mut registry = PluginRegistry::new();

        let routes = vec![HttpRoute {
            method: "GET".to_string(),
            path: "/plugins/test/*".to_string(),
            handler_id: "catch_all".to_string(),
        }];

        registry.register_http_routes("test_plugin", routes);

        assert_eq!(
            registry.route_http_request("GET", "/plugins/test/anything/here"),
            Some(("test_plugin", "catch_all"))
        );
    }

    #[test]
    fn test_cli_command_registration() {
        let mut registry = PluginRegistry::new();

        registry.register_cli_command("test_plugin", "test-command");

        assert_eq!(
            registry.find_command_handler("test-command"),
            Some("test_plugin")
        );

        assert_eq!(registry.find_command_handler("unknown"), None);
    }

    #[test]
    fn test_capabilities() {
        let mut registry = PluginRegistry::new();

        registry.register_capabilities(
            "test_plugin",
            vec!["http".to_string(), "cli".to_string()],
        );

        assert!(registry.has_capability("test_plugin", "http"));
        assert!(registry.has_capability("test_plugin", "cli"));
        assert!(!registry.has_capability("test_plugin", "wasm"));
    }

    #[test]
    fn test_unregister_plugin() {
        let mut registry = PluginRegistry::new();

        registry.register_http_routes(
            "test_plugin",
            vec![HttpRoute {
                method: "GET".to_string(),
                path: "/test".to_string(),
                handler_id: "handler".to_string(),
            }],
        );

        registry.register_cli_command("test_plugin", "test-cmd");

        registry.unregister_plugin("test_plugin");

        assert_eq!(registry.route_http_request("GET", "/test"), None);
        assert_eq!(registry.find_command_handler("test-cmd"), None);
    }
}
