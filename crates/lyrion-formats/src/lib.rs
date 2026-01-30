//! Audio format parsers and metadata extraction

use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

pub mod mp3;
pub mod flac;
pub mod wav;
pub mod aiff;
pub mod m4a;
pub mod ogg;
pub mod opus;
pub mod wma;
pub mod ape;
pub mod wv;
pub mod dsf;
pub mod dff;

/// Audio metadata extracted from files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration_ms: Option<u64>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub file_size: u64,
    pub format: String,
    pub artwork: Option<Vec<u8>>,
    pub modified_time: Option<NaiveDateTime>,
}

/// Trait for format-specific parsers
pub trait FormatParser {
    fn parse(&self, path: &str) -> Result<AudioMetadata>;
    fn supports_extension(&self, ext: &str) -> bool;
}

/// Get parser for file extension
pub fn get_parser(ext: &str) -> Option<Box<dyn FormatParser>> {
    match ext.to_lowercase().as_str() {
        "mp3" => Some(Box::new(mp3::Mp3Parser)),
        "flac" => Some(Box::new(flac::FlacParser)),
        "wav" => Some(Box::new(wav::WavParser)),
        "aiff" | "aif" | "aifc" => Some(Box::new(aiff::AiffParser)),
        "m4a" | "aac" | "mp4" | "alac" => Some(Box::new(m4a::M4aParser)),
        "ogg" => Some(Box::new(ogg::OggParser)),
        "opus" => Some(Box::new(opus::OpusParser)),
        "wma" => Some(Box::new(wma::WmaParser)),
        "ape" => Some(Box::new(ape::ApeParser)),
        "wv" => Some(Box::new(wv::WavPackParser)),
        "dsf" => Some(Box::new(dsf::DsfParser)),
        "dff" => Some(Box::new(dff::DffParser)),
        _ => None,
    }
}

/// Parse audio file metadata
pub fn parse_file(path: &str) -> Result<AudioMetadata> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let parser = get_parser(ext)
        .ok_or_else(|| anyhow::anyhow!("Unsupported format: {}", ext))?;

    parser.parse(path)
}
