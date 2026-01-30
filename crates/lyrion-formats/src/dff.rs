//! DFF (DSDIFF) format parser
//! Supports Sony's DSDIFF format with metadata chunks

use crate::{AudioMetadata, FormatParser};
use anyhow::{Result, bail};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub struct DffParser;

impl FormatParser for DffParser {
    fn parse(&self, path: &str) -> Result<AudioMetadata> {
        let mut file = File::open(path)?;
        let file_metadata = file.metadata()?;

        let modified_time = file_metadata
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).naive_utc());

        // Read FORM chunk header (12 bytes)
        let mut form_header = [0u8; 12];
        file.read_exact(&mut form_header)?;

        // Check magic number "FRM8"
        if &form_header[0..4] != b"FRM8" {
            bail!("Not a valid DFF file");
        }

        // Chunk size (big-endian u64 at offset 4)
        let _form_size = u64::from_be_bytes(form_header[4..12].try_into()?);

        // Check DSD identifier "DSD "
        let mut dsd_id = [0u8; 4];
        file.read_exact(&mut dsd_id)?;
        if &dsd_id != b"DSD " {
            bail!("Not a valid DSDIFF file");
        }

        let mut sample_rate = 0u32;
        let mut channels = 0u16;
        let mut sample_count = 0u64;
        let mut title = None;
        let mut artist = None;
        let mut _compression_type = None;

        // Read chunks
        loop {
            let mut chunk_header = [0u8; 12];
            if file.read_exact(&mut chunk_header).is_err() {
                break; // End of file
            }

            let chunk_id = &chunk_header[0..4];
            let chunk_size = u64::from_be_bytes(chunk_header[4..12].try_into()?);

            match chunk_id {
                b"FVER" => {
                    // Format version chunk - skip
                    file.seek(SeekFrom::Current(chunk_size as i64))?;
                }
                b"PROP" => {
                    // Property chunk - contains format info
                    let mut prop_type = [0u8; 4];
                    file.read_exact(&mut prop_type)?;

                    if &prop_type == b"SND " {
                        // Sound property - parse format chunks within
                        let prop_end = file.stream_position()? + chunk_size - 4;

                        while file.stream_position()? < prop_end {
                            let mut sub_chunk = [0u8; 12];
                            if file.read_exact(&mut sub_chunk).is_err() {
                                break;
                            }

                            let sub_id = &sub_chunk[0..4];
                            let sub_size = u64::from_be_bytes(sub_chunk[4..12].try_into()?);

                            match sub_id {
                                b"FS  " => {
                                    // Sample rate (big-endian u32)
                                    let mut rate_bytes = [0u8; 4];
                                    file.read_exact(&mut rate_bytes)?;
                                    sample_rate = u32::from_be_bytes(rate_bytes);
                                    file.seek(SeekFrom::Current((sub_size - 4) as i64))?;
                                }
                                b"CHNL" => {
                                    // Channel info (big-endian u16)
                                    let mut chan_bytes = [0u8; 2];
                                    file.read_exact(&mut chan_bytes)?;
                                    channels = u16::from_be_bytes(chan_bytes);
                                    file.seek(SeekFrom::Current((sub_size - 2) as i64))?;
                                }
                                b"CMPR" => {
                                    // Compression type
                                    let mut cmpr_bytes = [0u8; 4];
                                    file.read_exact(&mut cmpr_bytes)?;
                                    _compression_type = Some(cmpr_bytes);
                                    file.seek(SeekFrom::Current((sub_size - 4) as i64))?;
                                }
                                _ => {
                                    file.seek(SeekFrom::Current(sub_size as i64))?;
                                }
                            }
                        }
                    } else {
                        file.seek(SeekFrom::Current((chunk_size - 4) as i64))?;
                    }
                }
                b"DSD " => {
                    // DSD sound data chunk - contains sample count
                    sample_count = chunk_size; // Sample count in bytes
                    file.seek(SeekFrom::Current(chunk_size as i64))?;
                }
                b"DIIN" | b"DITI" => {
                    // Edited master information / Title
                    let mut text_data = vec![0u8; chunk_size.min(1024) as usize];
                    file.read_exact(&mut text_data)?;

                    // Try to extract text (might be in various encodings)
                    if let Ok(text) = String::from_utf8(text_data.clone()) {
                        let text = text.trim_end_matches('\0').to_string();
                        if chunk_id == b"DITI" && !text.is_empty() {
                            title = Some(text);
                        }
                    }

                    if chunk_size > 1024 {
                        file.seek(SeekFrom::Current((chunk_size - 1024) as i64))?;
                    }
                }
                b"DIAR" => {
                    // Artist
                    let mut text_data = vec![0u8; chunk_size.min(1024) as usize];
                    file.read_exact(&mut text_data)?;

                    if let Ok(text) = String::from_utf8(text_data) {
                        let text = text.trim_end_matches('\0').to_string();
                        if !text.is_empty() {
                            artist = Some(text);
                        }
                    }

                    if chunk_size > 1024 {
                        file.seek(SeekFrom::Current((chunk_size - 1024) as i64))?;
                    }
                }
                _ => {
                    // Unknown chunk - skip
                    file.seek(SeekFrom::Current(chunk_size as i64))?;
                }
            }
        }

        // Calculate duration
        let duration_secs = if sample_rate > 0 && channels > 0 {
            (sample_count as f64) / (sample_rate as f64 * channels as f64)
        } else {
            0.0
        };
        let duration_ms = (duration_secs * 1000.0) as u64;

        // Determine DSD rate
        let dsd_rate = if sample_rate > 0 {
            sample_rate / 44100
        } else {
            0
        };

        let metadata = AudioMetadata {
            title,
            artist,
            album: None,
            album_artist: None,
            genre: None,
            year: None,
            track_number: None,
            disc_number: None,
            duration_ms: Some(duration_ms),
            bitrate: Some((sample_rate * channels as u32) / 1000), // Approximate bitrate in kbps
            sample_rate: Some(sample_rate),
            channels: Some(channels as u8),
            file_size: file_metadata.len(),
            format: format!("dff:dsd{}", dsd_rate),
            artwork: None,
            modified_time,
        };

        Ok(metadata)
    }

    fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("dff")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dff_parser_supports() {
        let parser = DffParser;
        assert!(parser.supports_extension("dff"));
        assert!(parser.supports_extension("DFF"));
        assert!(!parser.supports_extension("dsf"));
    }
}
