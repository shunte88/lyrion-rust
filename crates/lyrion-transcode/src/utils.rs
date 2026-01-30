//! Transcoding utility functions

use std::path::Path;

/// Determine if transcoding is needed
pub fn needs_transcoding(file_format: &str, target_format: &str) -> bool {
    file_format != target_format
}

/// Get file format from extension
pub fn get_format_from_path(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_format_from_path() {
        assert_eq!(get_format_from_path("/music/test.mp3"), Some("mp3".to_string()));
        assert_eq!(get_format_from_path("/music/test.flac"), Some("flac".to_string()));
        assert_eq!(get_format_from_path("/music/test"), None);
    }

    #[test]
    fn test_needs_transcoding() {
        assert!(!needs_transcoding("mp3", "mp3"));
        assert!(needs_transcoding("flac", "mp3"));
    }
}
