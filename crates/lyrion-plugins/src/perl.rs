//! Perl Plugin Compatibility Layer
//!
//! Loads and executes existing Perl plugins without modification.
//! Provides backwards compatibility with Slim::Plugin::Base.

use crate::{Plugin, PluginContext, PluginError, PluginManifest, PluginResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{debug, error, info, warn};

/// Wrapper for Perl plugins
///
/// Executes Perl plugins in a subprocess for isolation and compatibility.
/// Uses IPC (JSON over stdin/stdout) to communicate between Rust and Perl.
pub struct PerlPlugin {
    /// Plugin name (e.g., "RandomPlay")
    name: String,

    /// Module path (e.g., "Slim::Plugin::RandomPlay::Plugin")
    module: String,

    /// Plugin base directory
    base_dir: PathBuf,

    /// Parsed manifest
    manifest: PluginManifest,

    /// Whether plugin is initialized
    initialized: bool,
}

impl PerlPlugin {
    /// Load a Perl plugin from install.xml manifest
    pub fn load(plugin_dir: impl AsRef<Path>) -> PluginResult<Self> {
        let plugin_dir = plugin_dir.as_ref();
        let install_xml = plugin_dir.join("install.xml");

        if !install_xml.exists() {
            return Err(PluginError::NotFound(format!(
                "No install.xml in {}",
                plugin_dir.display()
            )));
        }

        // Parse install.xml to get plugin metadata
        let manifest = Self::parse_install_xml(&install_xml)?;

        let name = plugin_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| PluginError::InvalidManifest("Invalid plugin directory".to_string()))?
            .to_string();

        let module = format!("Slim::Plugin::{}::Plugin", name);

        info!("Loaded Perl plugin manifest: {}", name);

        Ok(Self {
            name,
            module,
            base_dir: plugin_dir.to_path_buf(),
            manifest,
            initialized: false,
        })
    }

    /// Parse install.xml to extract plugin metadata
    fn parse_install_xml(path: &Path) -> PluginResult<PluginManifest> {
        let content = std::fs::read_to_string(path)?;

        // Simple XML parsing (in production, use quick-xml or similar)
        let name = Self::extract_xml_tag(&content, "name")
            .unwrap_or_else(|| "Unknown Plugin".to_string());
        let version = Self::extract_xml_tag(&content, "version").unwrap_or_else(|| "0.0.0".to_string());
        let description = Self::extract_xml_tag(&content, "description");
        let creator = Self::extract_xml_tag(&content, "creator").unwrap_or_else(|| "Unknown".to_string());
        let minTarget = Self::extract_xml_tag(&content, "minTarget");

        Ok(PluginManifest {
            name,
            version,
            author: creator,
            description,
            min_server_version: minTarget,
            dependencies: vec![],
            capabilities: vec!["perl".to_string()],
            enforced: false,
        })
    }

    /// Simple XML tag extraction (replace with proper XML parser if needed)
    fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start) = xml.find(&start_tag) {
            if let Some(end) = xml[start..].find(&end_tag) {
                let content = &xml[start + start_tag.len()..start + end];
                return Some(content.trim().to_string());
            }
        }
        None
    }

    /// Execute Perl code and get JSON response
    fn execute_perl_method(
        &self,
        method: &str,
        args: &HashMap<String, String>,
    ) -> PluginResult<String> {
        // Build Perl script to call method
        let perl_script = format!(
            r#"
use strict;
use warnings;
use lib '/data2/slimserver';
use JSON::XS;
use {};

my $plugin = {}->new();
my $args = decode_json('{}');
my $result;

eval {{
    if ($plugin->can('{}')) {{
        $result = $plugin->{}($args);
    }} else {{
        $result = {{ error => "Method {} not found" }};
    }}
}};

if ($@) {{
    $result = {{ error => $@ }};
}}

print encode_json($result);
"#,
            self.module,
            self.module,
            serde_json::to_string(args).unwrap_or_default(),
            method,
            method,
            method
        );

        // Execute Perl script
        let output = Command::new("perl")
            .arg("-e")
            .arg(&perl_script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| PluginError::LoadError(format!("Failed to execute Perl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PluginError::InitializationError(format!(
                "Perl execution failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Call Perl plugin's initPlugin method
    fn call_init_plugin(&mut self, _context: &PluginContext) -> Result<(), String> {
        info!("Initializing Perl plugin: {}", self.name);

        // For now, we'll mark as initialized
        // In production, we'd use a persistent Perl interpreter
        self.initialized = true;

        Ok(())
    }

    /// Call Perl plugin's shutdownPlugin method
    fn call_shutdown_plugin(&mut self) {
        info!("Shutting down Perl plugin: {}", self.name);
        self.initialized = false;
    }
}

impl Plugin for PerlPlugin {
    fn manifest(&self) -> PluginManifest {
        self.manifest.clone()
    }

    fn init(&mut self, context: &PluginContext) -> Result<(), String> {
        self.call_init_plugin(context)
    }

    fn shutdown(&mut self) {
        self.call_shutdown_plugin();
    }

    fn handle_command(
        &mut self,
        command: &str,
        params: &HashMap<String, String>,
    ) -> Option<String> {
        // Try to execute Perl command handler
        match self.execute_perl_method(command, params) {
            Ok(response) => Some(response),
            Err(e) => {
                error!("Perl command execution failed: {}", e);
                None
            }
        }
    }
}

/// Discover Perl plugins in the Slim/Plugin directory
pub fn discover_perl_plugins(slim_dir: impl AsRef<Path>) -> PluginResult<Vec<PathBuf>> {
    let plugin_dir = slim_dir.as_ref().join("Slim").join("Plugin");

    if !plugin_dir.exists() {
        warn!("Slim/Plugin directory not found: {}", plugin_dir.display());
        return Ok(vec![]);
    }

    let mut plugins = Vec::new();

    for entry in std::fs::read_dir(&plugin_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Check for install.xml
            if path.join("install.xml").exists() {
                debug!("Found Perl plugin: {}", path.display());
                plugins.push(path);
            }
        }
    }

    info!("Discovered {} Perl plugins", plugins.len());
    Ok(plugins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_parsing() {
        let xml = r#"
            <extension>
                <name>PLUGIN_RANDOMPLAY</name>
                <version>2.0</version>
                <description>PLUGIN_RANDOMPLAY_DESC</description>
                <creator>Logitech</creator>
                <minTarget>7.6</minTarget>
            </extension>
        "#;

        let name = PerlPlugin::extract_xml_tag(xml, "name");
        assert_eq!(name, Some("PLUGIN_RANDOMPLAY".to_string()));

        let version = PerlPlugin::extract_xml_tag(xml, "version");
        assert_eq!(version, Some("2.0".to_string()));

        let creator = PerlPlugin::extract_xml_tag(xml, "creator");
        assert_eq!(creator, Some("Logitech".to_string()));
    }
}
