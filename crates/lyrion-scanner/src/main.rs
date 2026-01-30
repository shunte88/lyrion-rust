//! Lyrion Scanner - Music library scanner
//! Scans directories for audio files and populates database with incremental update support

use anyhow::Result;
use lyrion_db::{DatabaseConfig, initialize_database, Track, Contributor, Genre, models::roles};
use lyrion_formats::{parse_file, AudioMetadata};
use std::path::Path;
use walkdir::WalkDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sha2::{Sha256, Digest};

#[derive(Debug)]
struct ScanOptions {
    music_dir: String,
    db_path: String,
    force_rescan: bool,
    update_artwork_only: bool,
    progress_interval: usize,
}

#[derive(Debug, Clone, Copy)]
enum ProcessStatus {
    Created,
    Updated,
    Skipped,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lyrion_scanner=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: lyrion-scanner <music_directory> [options]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --database <path>      Database path (default: lyrion-rust.db)");
        eprintln!("  --force-rescan         Rescan all files, ignore timestamps");
        eprintln!("  --update-artwork       Only update cover art for existing tracks");
        eprintln!("  --progress <n>         Show progress every n files (default: 100)");
        eprintln!();
        eprintln!("Incremental Scanning:");
        eprintln!("  By default, the scanner only processes new or modified files.");
        eprintln!("  Use --force-rescan to process all files regardless of timestamps.");
        std::process::exit(1);
    }

    let music_dir = &args[1];

    // Parse options
    let mut db_path = "lyrion-rust.db".to_string();
    let mut force_rescan = false;
    let mut update_artwork_only = false;
    let mut progress_interval = 100;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--database" => {
                if i + 1 < args.len() {
                    db_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --database requires a path");
                    std::process::exit(1);
                }
            }
            "--force-rescan" => {
                force_rescan = true;
                i += 1;
            }
            "--update-artwork" => {
                update_artwork_only = true;
                i += 1;
            }
            "--progress" => {
                if i + 1 < args.len() {
                    progress_interval = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    eprintln!("Error: --progress requires a number");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    let options = ScanOptions {
        music_dir: music_dir.to_string(),
        db_path: db_path.clone(),
        force_rescan,
        update_artwork_only,
        progress_interval,
    };

    tracing::info!("Scanning music directory: {}", options.music_dir);
    tracing::info!("Database: {}", options.db_path);
    if options.force_rescan {
        tracing::info!("Mode: Full rescan (ignoring timestamps)");
    } else {
        tracing::info!("Mode: Incremental scan (only new/modified files)");
    }
    if options.update_artwork_only {
        tracing::info!("Mode: Update cover art only");
    }

    // Initialize database
    let db_config = DatabaseConfig {
        path: db_path.to_string(),
        max_connections: 5,
    };

    let db_pool = initialize_database(&db_config).await?;

    // Scan directory
    let mut total_files = 0;
    let mut processed_files = 0;
    let mut created_files = 0;
    let mut updated_files = 0;
    let mut skipped_files = 0;
    let mut error_files = 0;

    // Count total files first
    for entry in WalkDir::new(music_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if is_audio_file(entry.path()) {
                total_files += 1;
            }
        }
    }

    tracing::info!("Found {} audio files to process", total_files);

    let start_time = std::time::Instant::now();

    // Process files
    for entry in WalkDir::new(music_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        if !is_audio_file(path) {
            continue;
        }

        processed_files += 1;

        if processed_files % options.progress_interval == 0 {
            let percent = (processed_files as f32 / total_files as f32) * 100.0;
            tracing::info!(
                "Progress: {}/{} files ({:.1}% - {} new, {} updated, {} skipped)",
                processed_files, total_files, percent,
                created_files, updated_files, skipped_files
            );
        }

        match process_audio_file_with_options(path, &db_pool, &options).await {
            Ok(status) => {
                match status {
                    ProcessStatus::Created => created_files += 1,
                    ProcessStatus::Updated => updated_files += 1,
                    ProcessStatus::Skipped => skipped_files += 1,
                }
                tracing::debug!("Processed: {:?} - {:?}", path, status);
            }
            Err(e) => {
                error_files += 1;
                tracing::warn!("Error processing {:?}: {}", path, e);
            }
        }
    }

    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f32();
    let files_per_sec = processed_files as f32 / elapsed_secs;
    let formatted_time = format_duration(elapsed_secs);

    tracing::info!(
        "Scan complete: {} files processed in {} ({:.0} files/sec)",
        processed_files,
        formatted_time,
        files_per_sec
    );
    tracing::info!(
        "  {} new, {} updated, {} skipped, {} errors",
        created_files, updated_files, skipped_files, error_files
    );

    // Print statistics
    let track_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
        .fetch_one(&db_pool)
        .await?;

    let album_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM albums")
        .fetch_one(&db_pool)
        .await?;

    let artist_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contributors")
        .fetch_one(&db_pool)
        .await?;

    let genre_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM genres")
        .fetch_one(&db_pool)
        .await?;

    let year_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT year) FROM tracks WHERE year IS NOT NULL"
    )
        .fetch_one(&db_pool)
        .await?;

    tracing::info!("Database statistics:");
    tracing::info!("  Artists .....: {artist_count:>11}");
    tracing::info!("  Albums ......: {album_count:>11}");
    tracing::info!("  Tracks ......: {track_count:>11}");
    tracing::info!("  Genres ......: {genre_count:>11}");
    tracing::info!("  Years .......: {year_count:>11}");

    Ok(())
}

/// Format duration in a human-readable way
fn format_duration(secs: f32) -> String {
    let total_secs = secs as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = ((secs - total_secs as f32) * 10.0) as u32;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if seconds > 10 {
        format!("{}s", seconds)
    } else {
        format!("{}.{}s", seconds, millis)
    }
}

/// Check if file is an audio file
fn is_audio_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(
            ext_str.as_str(),
            "mp3" | "flac" | "m4a" | "aac" | "mp4" | "alac" | "ogg" | "opus" | "wav" | "aiff" | "aif" | "aifc" | "wma" | "ape" | "wv" | "dsf" | "dff"
        )
    } else {
        false
    }
}

/// Compute hash of key metadata attributes for change detection
/// Includes: title, artist, album, year, track#, disc#, genre, cover art size
fn compute_metadata_hash(metadata: &AudioMetadata, cover_size: Option<usize>) -> String {
    let mut hasher = Sha256::new();

    // Hash key metadata fields that matter for updates
    if let Some(t) = &metadata.title {
        hasher.update(t.as_bytes());
    }
    if let Some(a) = &metadata.artist {
        hasher.update(a.as_bytes());
    }
    if let Some(al) = &metadata.album {
        hasher.update(al.as_bytes());
    }
    if let Some(y) = metadata.year {
        hasher.update(&y.to_le_bytes());
    }
    if let Some(tn) = metadata.track_number {
        hasher.update(&tn.to_le_bytes());
    }
    if let Some(dn) = metadata.disc_number {
        hasher.update(&dn.to_le_bytes());
    }
    if let Some(g) = &metadata.genre {
        hasher.update(g.as_bytes());
    }
    if let Some(c) = &metadata.composer {
        hasher.update(c.as_bytes());
    }
    if let Some(c) = &metadata.conductor {
        hasher.update(c.as_bytes());
    }
    if let Some(b) = metadata.bpm {
        hasher.update(&b.to_le_bytes());
    }
    if let Some(l) = &metadata.lyrics {
        hasher.update(l.chars().take(100).collect::<String>().as_bytes());
    }
    if let Some(m) = &metadata.musicbrainz_id {
        hasher.update(m.as_bytes());
    }
    if let Some(rg) = metadata.replay_gain {
        hasher.update(&rg.to_le_bytes());
    }
    // Hash cover art size (not full data for performance)
    if let Some(size) = cover_size {
        hasher.update(&size.to_le_bytes());
    }

    format!("{:x}", hasher.finalize())
}

/// Find folder cover art (cover.jpg, folder.jpg, etc.)
fn find_folder_cover_art(audio_path: &Path) -> Option<Vec<u8>> {
    let dir = audio_path.parent()?;

    // Common cover art filenames
    let cover_names = [
        "cover.jpg", "cover.jpeg", "cover.png",
        "folder.jpg", "folder.jpeg", "folder.png",
        "front.jpg", "front.jpeg", "front.png",
        "Cover.jpg", "Cover.jpeg", "Folder.jpg",
    ];

    for name in &cover_names {
        let cover_path = dir.join(name);
        if cover_path.exists() {
            if let Ok(data) = std::fs::read(&cover_path) {
                // Only return if reasonable size (< 5MB)
                if data.len() < 5_000_000 {
                    return Some(data);
                }
            }
        }
    }

    None
}

/// Process a single audio file with options (hash-based change detection)
async fn process_audio_file_with_options(
    path: &Path,
    db_pool: &sqlx::SqlitePool,
    options: &ScanOptions,
) -> Result<ProcessStatus> {
    let path_str = path.to_string_lossy().to_string();

    // Parse metadata first to compute hash
    let metadata = parse_file(&path_str)?;

    // Get cover art (embedded or folder)
    let cover_art = metadata.artwork.clone()
        .or_else(|| find_folder_cover_art(path));

    // Compute hash of key metadata attributes
    let current_hash = compute_metadata_hash(&metadata, cover_art.as_ref().map(|c| c.len()));

    // Get file mtime for timestamp field
    let file_metadata = std::fs::metadata(path)?;
    let file_mtime = file_metadata.modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    // Check if already in database with hash comparison
    let existing: Option<(i64, Option<String>)> = sqlx::query_as(
        "SELECT id, metadata_hash FROM tracks WHERE url = ?"
    )
    .bind(&path_str)
    .fetch_optional(db_pool)
    .await?;

    if let Some((track_id, db_hash)) = existing {
        // Compare hashes (unless force rescan)
        if !options.force_rescan {
            if let Some(ref hash) = db_hash {
                if hash == &current_hash {
                    // Exact metadata match, skip
                    tracing::debug!("Skipping unchanged track: {}", path_str);
                    return Ok(ProcessStatus::Skipped);
                }
            }
        }

        // Hash differs or force rescan - actual metadata change detected
        tracing::debug!("Metadata changed for track: {}", path_str);

        if options.update_artwork_only {
            update_track_artwork_only(track_id, path, db_pool, &current_hash).await?;
        } else {
            // Delete and reinsert for clean update
            delete_and_reinsert(track_id, path, db_pool, &metadata, cover_art, &current_hash, file_mtime).await?;
        }
        return Ok(ProcessStatus::Updated);
    }

    // New file - insert it (unless update-artwork-only mode)
    if options.update_artwork_only {
        return Ok(ProcessStatus::Skipped);
    }

    // Insert new track with hash
    tracing::debug!("Inserting new track: {}", path_str);

    // Get or create album
    let album_id = if let Some(album_title) = &metadata.album {
        let album_search = album_title.to_lowercase();

        let existing_album: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM albums WHERE titlesearch = ?"
        )
        .bind(&album_search)
        .fetch_optional(db_pool)
        .await?;

        if let Some((id,)) = existing_album {
            id
        } else {
            // Create new album
            let result = sqlx::query(
                "INSERT INTO albums (title, titlesort, titlesearch, year, compilation)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(album_title)
            .bind(album_title)
            .bind(&album_search)
            .bind(metadata.year.map(|y| y as i16))
            .bind(false)
            .execute(db_pool)
            .await?;

            result.last_insert_rowid()
        }
    } else {
        // No album
        0
    };

    // Insert track
    let track = Track {
        id: 0,
        url: path_str.clone(),
        title: metadata.title.clone(),
        titlesort: metadata.title.clone(),
        titlesearch: metadata.title.as_ref().map(|t| t.to_lowercase()),
        customsearch: None,
        album: if album_id > 0 { Some(album_id) } else { None },
        tracknum: metadata.track_number.map(|n| n as i32),
        content_type: Some(metadata.format.clone()),
        timestamp: metadata.modified_time.map(|t| t.and_utc().timestamp()),
        filesize: Some(metadata.file_size as i64),
        audio_size: None,
        audio_offset: None,
        year: metadata.year.map(|y| y as i16),
        secs: metadata.duration_ms.map(|ms| (ms as f32) / 1000.0),
        cover: cover_art,
        vbr_scale: None,
        bitrate: metadata.bitrate.map(|b| b as f32),
        samplerate: metadata.sample_rate.map(|s| s as i32),
        samplesize: None,
        channels: metadata.channels.map(|c| c as i8),
        block_alignment: None,
        endian: None,
        bpm: metadata.bpm.map(|b| b as i16),
        tagversion: None,
        drm: Some(false),
        disc: metadata.disc_number.map(|d| d as i8),
        audio: Some(true),
        remote: Some(false),
        lossless: Some(
            matches!(metadata.format.as_str(), "flac" | "wav" | "aiff" | "ape" | "wv")
                || metadata.format.starts_with("dsf:")
                || metadata.format.starts_with("dff:")
        ),
        lyrics: metadata.lyrics.clone(),
        musicbrainz_id: metadata.musicbrainz_id.clone(),
        musicmagic_mixable: None,
        replay_gain: metadata.replay_gain,
        replay_peak: metadata.replay_peak,
        extid: None,
        metadata_hash: Some(current_hash),
    };

    let track_id = track.insert(db_pool).await?;

    // Link artist
    if let Some(artist_name) = &metadata.artist {
        let contributor_id = Contributor::find_or_create(db_pool, artist_name).await?;

        // Link artist to track
        sqlx::query(
            "INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)"
        )
        .bind(roles::ARTIST)
        .bind(contributor_id)
        .bind(track_id)
        .execute(db_pool)
        .await?;
    }

    // Link composer
    if let Some(composer_name) = &metadata.composer {
        let composer_id = Contributor::find_or_create(db_pool, composer_name).await?;
        sqlx::query("INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)")
            .bind(roles::COMPOSER)
            .bind(composer_id)
            .bind(track_id)
            .execute(db_pool)
            .await?;
    }

    // Link conductor
    if let Some(conductor_name) = &metadata.conductor {
        let conductor_id = Contributor::find_or_create(db_pool, conductor_name).await?;
        sqlx::query("INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)")
            .bind(roles::CONDUCTOR)
            .bind(conductor_id)
            .bind(track_id)
            .execute(db_pool)
            .await?;
    }

    // Link album artist if different from track artist
    if let Some(album_artist_name) = &metadata.album_artist {
        if metadata.artist.as_ref() != Some(album_artist_name) {
            let album_artist_id = Contributor::find_or_create(db_pool, album_artist_name).await?;
            sqlx::query("INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)")
                .bind(roles::ALBUMARTIST)
                .bind(album_artist_id)
                .bind(track_id)
                .execute(db_pool)
                .await?;
        }
    }

    // Link genre
    if let Some(genre_name) = &metadata.genre {
        let genre_id = Genre::find_or_create(db_pool, genre_name).await?;

        // Link genre to track
        sqlx::query(
            "INSERT OR IGNORE INTO genre_track (genre, track) VALUES (?, ?)"
        )
        .bind(genre_id)
        .bind(track_id)
        .execute(db_pool)
        .await?;
    }

    Ok(ProcessStatus::Created)
}

/// Update only cover art for existing track (and hash)
async fn update_track_artwork_only(
    track_id: i64,
    path: &Path,
    db_pool: &sqlx::SqlitePool,
    hash: &str,
) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();

    // Parse metadata to get embedded artwork
    let metadata = parse_file(&path_str)?;

    // Get cover art (embedded or folder)
    let cover_art = metadata.artwork.clone()
        .or_else(|| find_folder_cover_art(path));

    if cover_art.is_some() {
        // Update cover art and hash
        sqlx::query("UPDATE tracks SET cover = ?, metadata_hash = ? WHERE id = ?")
            .bind(&cover_art)
            .bind(hash)
            .bind(track_id)
            .execute(db_pool)
            .await?;
    }

    Ok(())
}

/// Delete and reinsert track with updated metadata
/// Cleaner than UPDATE - properly handles all relationship changes
async fn delete_and_reinsert(
    track_id: i64,
    path: &Path,
    db_pool: &sqlx::SqlitePool,
    metadata: &AudioMetadata,
    cover_art: Option<Vec<u8>>,
    hash: &str,
    file_mtime: i64,
) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();

    // Delete old track (cascades to contributor_track and genre_track if foreign keys enabled)
    sqlx::query("DELETE FROM contributor_track WHERE track = ?")
        .bind(track_id)
        .execute(db_pool)
        .await?;

    sqlx::query("DELETE FROM genre_track WHERE track = ?")
        .bind(track_id)
        .execute(db_pool)
        .await?;

    sqlx::query("DELETE FROM tracks WHERE id = ?")
        .bind(track_id)
        .execute(db_pool)
        .await?;

    // Get or create album
    let album_id = if let Some(album_title) = &metadata.album {
        let album_search = album_title.to_lowercase();
        let existing_album: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM albums WHERE titlesearch = ?"
        )
        .bind(&album_search)
        .fetch_optional(db_pool)
        .await?;

        if let Some((id,)) = existing_album {
            id
        } else {
            let result = sqlx::query(
                "INSERT INTO albums (title, titlesort, titlesearch, year, compilation)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(album_title)
            .bind(album_title)
            .bind(&album_search)
            .bind(metadata.year.map(|y| y as i16))
            .bind(false)
            .execute(db_pool)
            .await?;
            result.last_insert_rowid()
        }
    } else {
        0
    };

    // Insert fresh track with new metadata and hash
    let track = Track {
        id: 0,
        url: path_str.clone(),
        title: metadata.title.clone(),
        titlesort: metadata.title.clone(),
        titlesearch: metadata.title.as_ref().map(|t| t.to_lowercase()),
        customsearch: None,
        album: if album_id > 0 { Some(album_id) } else { None },
        tracknum: metadata.track_number.map(|n| n as i32),
        content_type: Some(metadata.format.clone()),
        timestamp: Some(file_mtime),
        filesize: Some(metadata.file_size as i64),
        audio_size: None,
        audio_offset: None,
        year: metadata.year.map(|y| y as i16),
        secs: metadata.duration_ms.map(|ms| (ms as f32) / 1000.0),
        cover: cover_art,
        vbr_scale: None,
        bitrate: metadata.bitrate.map(|b| b as f32),
        samplerate: metadata.sample_rate.map(|s| s as i32),
        samplesize: None,
        channels: metadata.channels.map(|c| c as i8),
        block_alignment: None,
        endian: None,
        bpm: metadata.bpm.map(|b| b as i16),
        tagversion: None,
        drm: Some(false),
        disc: metadata.disc_number.map(|d| d as i8),
        audio: Some(true),
        remote: Some(false),
        lossless: Some(
            matches!(metadata.format.as_str(), "flac" | "wav" | "aiff" | "ape" | "wv")
                || metadata.format.starts_with("dsf:")
                || metadata.format.starts_with("dff:")
        ),
        lyrics: metadata.lyrics.clone(),
        musicbrainz_id: metadata.musicbrainz_id.clone(),
        musicmagic_mixable: None,
        replay_gain: metadata.replay_gain,
        replay_peak: metadata.replay_peak,
        extid: None,
        metadata_hash: Some(hash.to_string()),
    };

    let new_track_id = track.insert(db_pool).await?;

    // Link artist
    if let Some(artist_name) = &metadata.artist {
        let contributor_id = Contributor::find_or_create(db_pool, artist_name).await?;
        sqlx::query(
            "INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)"
        )
        .bind(roles::ARTIST)
        .bind(contributor_id)
        .bind(new_track_id)
        .execute(db_pool)
        .await?;
    }

    // Link composer
    if let Some(composer_name) = &metadata.composer {
        let composer_id = Contributor::find_or_create(db_pool, composer_name).await?;
        sqlx::query("INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)")
            .bind(roles::COMPOSER)
            .bind(composer_id)
            .bind(new_track_id)
            .execute(db_pool)
            .await?;
    }

    // Link conductor
    if let Some(conductor_name) = &metadata.conductor {
        let conductor_id = Contributor::find_or_create(db_pool, conductor_name).await?;
        sqlx::query("INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)")
            .bind(roles::CONDUCTOR)
            .bind(conductor_id)
            .bind(new_track_id)
            .execute(db_pool)
            .await?;
    }

    // Link album artist if different from track artist
    if let Some(album_artist_name) = &metadata.album_artist {
        if metadata.artist.as_ref() != Some(album_artist_name) {
            let album_artist_id = Contributor::find_or_create(db_pool, album_artist_name).await?;
            sqlx::query("INSERT OR IGNORE INTO contributor_track (role, contributor, track) VALUES (?, ?, ?)")
                .bind(roles::ALBUMARTIST)
                .bind(album_artist_id)
                .bind(new_track_id)
                .execute(db_pool)
                .await?;
        }
    }

    // Link genre
    if let Some(genre_name) = &metadata.genre {
        let genre_id = Genre::find_or_create(db_pool, genre_name).await?;
        sqlx::query(
            "INSERT OR IGNORE INTO genre_track (genre, track) VALUES (?, ?)"
        )
        .bind(genre_id)
        .bind(new_track_id)
        .execute(db_pool)
        .await?;
    }

    Ok(())
}
