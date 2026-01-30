//! M4A/AAC/ALAC format parser using lofty
//! Supports AAC (lossy) and ALAC (Apple Lossless) codecs in M4A container

use crate::{AudioMetadata, FormatParser};
use anyhow::Result;
use lofty::prelude::*;
use lofty::probe::Probe;
use std::fs;

pub struct M4aParser;

impl FormatParser for M4aParser {
    fn parse(&self, path: &str) -> Result<AudioMetadata> {
        let tagged_file = Probe::open(path)?.read()?;

        let properties = tagged_file.properties();
        let tag = tagged_file.primary_tag().or(tagged_file.first_tag());

        let file_metadata = fs::metadata(path)?;
        let modified_time = file_metadata
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).naive_utc());

        let mut metadata = AudioMetadata {
            title: tag.and_then(|t| t.title().map(|s| s.to_string())),
            artist: tag.and_then(|t| t.artist().map(|s| s.to_string())),
            album: tag.and_then(|t| t.album().map(|s| s.to_string())),
            album_artist: tag.and_then(|t| {
                // Try to get album artist from various tags
                t.get_string(&lofty::tag::ItemKey::AlbumArtist)
                    .map(|s| s.to_string())
            }),
            genre: tag.and_then(|t| t.genre().map(|s| s.to_string())),
            year: tag.and_then(|t| t.year()),
            track_number: tag.and_then(|t| t.track()),
            disc_number: tag.and_then(|t| t.disk()),
            duration_ms: Some(properties.duration().as_millis() as u64),
            bitrate: properties.audio_bitrate().map(|b| b as u32),
            sample_rate: properties.sample_rate(),
            channels: properties.channels().map(|c| c as u8),
            file_size: file_metadata.len(),
            format: "m4a".to_string(), // Note: Can contain AAC or ALAC codec
            artwork: None,
            modified_time,
        };

        // Extract artwork
        if let Some(tag) = tag {
            if let Some(picture) = tag.pictures().first() {
                metadata.artwork = Some(picture.data().to_vec());
            }
        }

        Ok(metadata)
    }

    fn supports_extension(&self, ext: &str) -> bool {
        matches!(ext.to_lowercase().as_str(), "m4a" | "aac" | "mp4" | "alac")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m4a_parser_supports() {
        let parser = M4aParser;
        assert!(parser.supports_extension("m4a"));
        assert!(parser.supports_extension("aac"));
        assert!(parser.supports_extension("mp4"));
        assert!(parser.supports_extension("alac"));
        assert!(parser.supports_extension("M4A"));
        assert!(!parser.supports_extension("mp3"));
    }
}
