//! WMA (Windows Media Audio) format parser using lofty

use crate::{AudioMetadata, FormatParser};
use anyhow::Result;
use lofty::prelude::*;
use lofty::probe::Probe;
use std::fs;

pub struct WmaParser;

impl FormatParser for WmaParser {
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
            format: "wma".to_string(),
            artwork: None,
            modified_time,
            composer: tag.and_then(|t| t.get_string(&lofty::tag::ItemKey::Composer).map(|s| s.to_string())),
            conductor: tag.and_then(|t| t.get_string(&lofty::tag::ItemKey::Conductor).map(|s| s.to_string())),
            bpm: tag.and_then(|t| t.get_string(&lofty::tag::ItemKey::Bpm).and_then(|s| s.parse::<u16>().ok())),
            lyrics: tag.and_then(|t| t.get_string(&lofty::tag::ItemKey::Lyrics).map(|s| s.to_string())),
            musicbrainz_id: tag.and_then(|t| t.get_string(&lofty::tag::ItemKey::MusicBrainzRecordingId).map(|s| s.to_string())),
            replay_gain: tag.and_then(|t| t.get_string(&lofty::tag::ItemKey::ReplayGainTrackGain).and_then(|s| s.trim_end_matches(" dB").parse::<f32>().ok())),
            replay_peak: tag.and_then(|t| t.get_string(&lofty::tag::ItemKey::ReplayGainTrackPeak).and_then(|s| s.parse::<f32>().ok())),
        };

        // Extract artwork from ASF extended content
        if let Some(tag) = tag {
            if let Some(picture) = tag.pictures().first() {
                metadata.artwork = Some(picture.data().to_vec());
            }
        }

        Ok(metadata)
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("wma")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wma_parser_supports() {
        let parser = WmaParser;
        assert!(parser.supports_extension("wma"));
        assert!(parser.supports_extension("WMA"));
        assert!(!parser.supports_extension("mp3"));
    }
}
