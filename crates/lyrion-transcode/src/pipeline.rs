//! Transcoding pipeline implementation

use anyhow::Result;
use std::process::{Child, Command, Stdio};

/// Transcoding pipeline
pub struct TranscodePipeline {
    processes: Vec<Child>,
}

impl TranscodePipeline {
    /// Create a new pipeline for transcoding
    pub fn new(
        input_file: &str,
        from_format: &str,
        to_format: &str,
    ) -> Result<Self> {
        let commands = get_transcode_commands(from_format, to_format)?;

        let mut processes: Vec<Child> = Vec::new();

        // Build the pipeline by chaining commands
        for (idx, cmd_template) in commands.iter().enumerate() {
            let args = substitute_variables(cmd_template, input_file);

            let mut command = Command::new(&args[0]);
            command.args(&args[1..]);

            // Set up stdin
            if idx == 0 {
                // First command - doesn't need stdin piping (reads from file)
                command.stdin(Stdio::null());
            } else {
                // Get stdout from previous process
                if let Some(prev_child) = processes.last_mut() {
                    let stdout = prev_child.stdout.take()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get stdout from previous command"))?;
                    command.stdin(stdout);
                }
            }

            // Set up stdout
            command.stdout(Stdio::piped());

            // Suppress stderr
            command.stderr(Stdio::null());

            let child = command.spawn()?;
            processes.push(child);
        }

        Ok(Self { processes })
    }

    /// Get the output stream from the pipeline
    /// Returns tokio wrapper around the std stdout
    pub fn take_stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        if let Some(child) = self.processes.last_mut() {
            child.stdout.take().and_then(|stdout| {
                // Convert std::process::ChildStdout to tokio::process::ChildStdout
                tokio::process::ChildStdout::from_std(stdout).ok()
            })
        } else {
            None
        }
    }

    /// Wait for pipeline to complete (async)
    pub async fn wait(&mut self) -> Result<()> {
        // Wait for each process in the pipeline
        // Note: This blocks but is moved to a blocking thread pool
        let mut procs = std::mem::take(&mut self.processes);
        tokio::task::spawn_blocking(move || {
            for process in &mut procs {
                if let Ok(status) = process.wait() {
                    if !status.success() {
                        return Err(anyhow::anyhow!("Pipeline process failed with status: {}", status));
                    }
                }
            }
            Ok(())
        }).await?
    }

    /// Kill the pipeline
    pub fn kill(&mut self) -> Result<()> {
        for process in &mut self.processes {
            let _ = process.kill();
        }
        Ok(())
    }
}

impl Drop for TranscodePipeline {
    fn drop(&mut self) {
        // Ensure processes are killed on drop
        let _ = self.kill();
    }
}

/// Get transcode commands for a format pair
fn get_transcode_commands(from: &str, to: &str) -> Result<Vec<Vec<String>>> {
    // Hardcoded common transcoding rules
    // In production, this would parse convert.conf
    match (from, to) {
        // FLAC transcoding
        ("flac", "mp3") => Ok(vec![
            vec!["flac".to_string(), "-dcs".to_string(), "$FILE$".to_string()],
            vec!["lame".to_string(), "-b".to_string(), "320".to_string(), "-".to_string()],
        ]),
        ("flac", "wav") => Ok(vec![
            vec!["flac".to_string(), "-dcs".to_string(), "$FILE$".to_string()],
        ]),

        // WAV transcoding
        ("wav", "mp3") => Ok(vec![
            vec!["lame".to_string(), "-b".to_string(), "320".to_string(), "$FILE$".to_string()],
        ]),

        // AIFF transcoding (Apple's uncompressed format)
        ("aiff", "mp3") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "mp3".to_string(),
                "-b:a".to_string(),
                "320k".to_string(),
                "-".to_string(),
            ],
        ]),
        ("aiff", "flac") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "flac".to_string(),
                "-".to_string(),
            ],
        ]),

        // M4A/AAC transcoding (using ffmpeg)
        ("m4a" | "aac", "mp3") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "mp3".to_string(),
                "-b:a".to_string(),
                "320k".to_string(),
                "-".to_string(),
            ],
        ]),

        // OGG Vorbis transcoding
        ("ogg", "mp3") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "mp3".to_string(),
                "-b:a".to_string(),
                "320k".to_string(),
                "-".to_string(),
            ],
        ]),

        // Opus transcoding
        ("opus", "mp3") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "mp3".to_string(),
                "-b:a".to_string(),
                "320k".to_string(),
                "-".to_string(),
            ],
        ]),

        // WMA transcoding
        ("wma", "mp3") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "mp3".to_string(),
                "-b:a".to_string(),
                "320k".to_string(),
                "-".to_string(),
            ],
        ]),

        // APE (Monkey's Audio) transcoding
        ("ape", "mp3") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "mp3".to_string(),
                "-b:a".to_string(),
                "320k".to_string(),
                "-".to_string(),
            ],
        ]),
        ("ape", "flac") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "flac".to_string(),
                "-".to_string(),
            ],
        ]),

        // WavPack transcoding
        ("wv", "mp3") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "mp3".to_string(),
                "-b:a".to_string(),
                "320k".to_string(),
                "-".to_string(),
            ],
        ]),
        ("wv", "flac") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "flac".to_string(),
                "-".to_string(),
            ],
        ]),

        // DSD transcoding (DSF/DFF to PCM via DoP - DSD over PCM)
        // Note: DSD requires special handling, usually converted to high-res PCM first
        ("dsf", "flac") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "flac".to_string(),
                "-sample_fmt".to_string(),
                "s32".to_string(),
                "-ar".to_string(),
                "88200".to_string(), // Downsample to 88.2kHz PCM
                "-".to_string(),
            ],
        ]),
        ("dsf", "wav") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "wav".to_string(),
                "-sample_fmt".to_string(),
                "s32".to_string(),
                "-ar".to_string(),
                "88200".to_string(),
                "-".to_string(),
            ],
        ]),
        ("dff", "flac") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "flac".to_string(),
                "-sample_fmt".to_string(),
                "s32".to_string(),
                "-ar".to_string(),
                "88200".to_string(),
                "-".to_string(),
            ],
        ]),
        ("dff", "wav") => Ok(vec![
            vec![
                "ffmpeg".to_string(),
                "-i".to_string(),
                "$FILE$".to_string(),
                "-f".to_string(),
                "wav".to_string(),
                "-sample_fmt".to_string(),
                "s32".to_string(),
                "-ar".to_string(),
                "88200".to_string(),
                "-".to_string(),
            ],
        ]),

        // Direct copy for same format (no transcoding needed)
        ("mp3", "mp3")
        | ("flac", "flac")
        | ("wav", "wav")
        | ("aiff", "aiff")
        | ("m4a", "m4a")
        | ("aac", "aac")
        | ("ogg", "ogg")
        | ("opus", "opus")
        | ("wma", "wma")
        | ("ape", "ape")
        | ("wv", "wv")
        | ("dsf", "dsf")
        | ("dff", "dff") => Ok(vec![
            vec!["cat".to_string(), "$FILE$".to_string()],
        ]),

        _ => Err(anyhow::anyhow!(
            "No transcoding rule for {} -> {}",
            from,
            to
        )),
    }
}

/// Substitute variables in command template
fn substitute_variables(template: &[String], file_path: &str) -> Vec<String> {
    template
        .iter()
        .map(|arg| arg.replace("$FILE$", file_path))
        .collect()
}
