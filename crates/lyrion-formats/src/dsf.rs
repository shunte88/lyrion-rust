//! DSF (DSD Stream File) format parser
//! Supports DSD64, DSD128, DSD256 with ID3v2 tags

use crate::{AudioMetadata, FormatParser};
use anyhow::{Result, bail};
use lofty::prelude::*;
use lofty::probe::Probe;
use std::fs::File;
use std::io::Read;

pub struct DsfParser;

impl FormatParser for DsfParser {
    fn parse(&self, path: &str) -> Result<AudioMetadata> {
        let mut file = File::open(path)?;
        let file_metadata = file.metadata()?;

        let modified_time = file_metadata
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).naive_utc());

        // Read DSF header (28 bytes)
        let mut header = [0u8; 28];
        file.read_exact(&mut header)?;

        // Check magic number "DSD "
        if &header[0..4] != b"DSD " {
            bail!("Not a valid DSF file");
        }

        // Read chunk size (little-endian u64 at offset 4)
        let _chunk_size = u64::from_le_bytes(header[4..12].try_into()?);

        // Total file size (little-endian u64 at offset 12)
        let _file_size = u64::from_le_bytes(header[12..20].try_into()?);

        // Metadata chunk pointer (little-endian u64 at offset 20)
        let _metadata_offset = u64::from_le_bytes(header[20..28].try_into()?);

        // Read fmt chunk (52 bytes)
        let mut fmt_chunk = [0u8; 52];
        file.read_exact(&mut fmt_chunk)?;

        // Check format chunk ID "fmt "
        if &fmt_chunk[0..4] != b"fmt " {
            bail!("Invalid DSF format chunk");
        }

        // Parse format properties
        let _format_version = u32::from_le_bytes(fmt_chunk[12..16].try_into()?);
        let _format_id = u32::from_le_bytes(fmt_chunk[16..20].try_into()?); // 0 = DSD raw
        let channel_type = u32::from_le_bytes(fmt_chunk[20..24].try_into()?);
        let channel_num = u32::from_le_bytes(fmt_chunk[24..28].try_into()?);
        let sampling_frequency = u32::from_le_bytes(fmt_chunk[28..32].try_into()?); // Hz
        let _bits_per_sample = u32::from_le_bytes(fmt_chunk[32..36].try_into()?); // Always 1 for DSD
        let sample_count = u64::from_le_bytes(fmt_chunk[36..44].try_into()?);
        let _block_size_per_channel = u32::from_le_bytes(fmt_chunk[44..48].try_into()?);

        // Calculate duration
        let duration_secs = if sampling_frequency > 0 {
            sample_count as f64 / sampling_frequency as f64
        } else {
            0.0
        };
        let duration_ms = (duration_secs * 1000.0) as u64;

        // Determine DSD rate (DSD64 = 2.8224 MHz, DSD128 = 5.6448 MHz, etc.)
        let dsd_rate = sampling_frequency / 44100; // Multiples of 44.1 kHz

        // Determine channels from channel_type
        let channels = match channel_type {
            1 => 1,  // Mono
            2 => 2,  // Stereo
            3 => 3,  // 3 channels
            4 => 4,  // Quad
            5 => 4,  // 4 channels
            6 => 5,  // 5 channels
            7 => 6,  // 5.1
            _ => channel_num,
        } as u8;

        let mut metadata = AudioMetadata {
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            genre: None,
            year: None,
            track_number: None,
            disc_number: None,
            duration_ms: Some(duration_ms),
            bitrate: Some((sampling_frequency * channel_num) / 1000), // Approximate bitrate in kbps
            sample_rate: Some(sampling_frequency),
            channels: Some(channels),
            file_size: file_metadata.len(),
            format: format!("dsf:dsd{}", dsd_rate),
            artwork: None,
            modified_time,
        };

        // Try to read ID3v2 metadata if present
        // Lofty can read DSF files with ID3v2 tags directly
        if let Ok(tagged_file) = Probe::open(path) {
            if let Ok(tagged_file) = tagged_file.read() {
                if let Some(tag) = tagged_file.primary_tag().or(tagged_file.first_tag()) {
                    metadata.title = tag.title().map(|s| s.to_string());
                    metadata.artist = tag.artist().map(|s| s.to_string());
                    metadata.album = tag.album().map(|s| s.to_string());
                    metadata.genre = tag.genre().map(|s| s.to_string());
                    metadata.year = tag.year();
                    metadata.track_number = tag.track();
                    metadata.disc_number = tag.disk();

                    // Extract artwork
                    if let Some(picture) = tag.pictures().first() {
                        metadata.artwork = Some(picture.data().to_vec());
                    }
                }
            }
        }

        Ok(metadata)
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("dsf")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsf_parser_supports() {
        let parser = DsfParser;
        assert!(parser.supports_extension("dsf"));
        assert!(parser.supports_extension("DSF"));
        assert!(!parser.supports_extension("dff"));
    }
}
