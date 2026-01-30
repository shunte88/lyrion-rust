# Getting Started with Lyrion Rust

Quick start guide for running the Rust implementation of Lyrion Music Server.

## Prerequisites

- Rust 1.80 or later
- SQLite development files
- Music collection to scan

## Installation

### 1. Build the project

```bash
cd /data2/slimserver/lyrion-rust
cargo build --release
```

This creates two binaries:
- `target/release/lyrion-server` - Main server
- `target/release/lyrion-scanner` - Music library scanner

### 2. Scan your music library

```bash
./target/release/lyrion-scanner /path/to/your/music
```

This will:
- Walk through all subdirectories
- Find audio files (MP3, FLAC, etc.)
- Extract metadata
- Populate `lyrion-rust.db`

Example output:
```
[2026-01-28T12:00:00Z INFO  lyrion_scanner] Scanning music directory: /music
[2026-01-28T12:00:00Z INFO  lyrion_scanner] Found 1234 audio files to process
[2026-01-28T12:00:10Z INFO  lyrion_scanner] Progress: 100/1234 files
...
[2026-01-28T12:05:00Z INFO  lyrion_scanner] Scan complete: 1234 processed, 0 errors
[2026-01-28T12:05:00Z INFO  lyrion_scanner] Database statistics:
[2026-01-28T12:05:00Z INFO  lyrion_scanner]   Tracks: 1234
[2026-01-28T12:05:00Z INFO  lyrion_scanner]   Albums: 98
[2026-01-28T12:05:00Z INFO  lyrion_scanner]   Artists: 156
```

### 3. Start the server

```bash
./target/release/lyrion-server
```

The server listens on:
- **Port 3483**: Slimproto (Squeezebox players)
- **Port 9000**: HTTP API

## Testing the Server

### Check server is running

```bash
curl http://localhost:9000/
```

Expected output:
```
Lyrion Music Server
Rust Edition v0.1.0

Endpoints:
  /api/v1/players
  /api/v1/tracks
  /jsonrpc.js
```

### List tracks

```bash
curl http://localhost:9000/api/v1/tracks?limit=10
```

### Search tracks

```bash
curl 'http://localhost:9000/api/v1/tracks/search?q=love'
```

### JSON-RPC (LMS compatible)

```bash
curl -X POST http://localhost:9000/jsonrpc.js \
  -H 'Content-Type: application/json' \
  -d '{
    "id": 1,
    "method": "slim.request",
    "params": ["player_id", ["status"]]
  }'
```

## Connecting a Squeezebox

1. Power on your Squeezebox player
2. Go to Settings → Advanced → Network
3. Set server IP to your machine's IP address
4. Set server port to 3483 (default)
5. Restart the player

Watch the server logs for:
```
[INFO lyrion_protocol::server] New connection from 192.168.1.100:12345
[INFO lyrion_protocol::server] Player HELO: MAC=00:11:22:33:44:55, Device=4, Revision=123
```

Check connected players:
```bash
curl http://localhost:9000/api/v1/players
```

Expected output:
```json
[
  {
    "mac": "00:11:22:33:44:55",
    "device_id": 4,
    "revision": 123,
    "uuid": "abc123..."
  }
]
```

## Directory Structure

After building and running:

```
lyrion-rust/
├── target/release/
│   ├── lyrion-server      # Main server binary
│   └── lyrion-scanner     # Scanner binary
├── lyrion-rust.db         # SQLite database (created by scanner)
└── web/dist/              # Web UI files (future)
```

## Environment Variables

Set logging level:
```bash
export RUST_LOG=debug
./target/release/lyrion-server
```

Levels: `error`, `warn`, `info`, `debug`, `trace`

## Migration from Perl Slimserver

### Option 1: Import existing database

```bash
cp /path/to/slimserver/Cache/library.db lyrion-rust.db
./target/release/lyrion-server
```

The server will automatically apply any missing migrations.

### Option 2: Fresh scan

```bash
./target/release/lyrion-scanner /path/to/music
./target/release/lyrion-server
```

## Troubleshooting

### Server won't start - Port already in use

```bash
# Check what's using port 9000
lsof -i :9000

# Or use a different port (edit main.rs and rebuild)
# Change: let http_addr = "0.0.0.0:9001";
```

### Player won't connect

1. Check firewall allows port 3483
2. Verify server is running: `netstat -ln | grep 3483`
3. Check server logs for connection attempts
4. Try telnet test: `telnet server-ip 3483`

### Scanner finds no files

1. Check path is correct: `ls /path/to/music`
2. Verify file extensions are supported: `.mp3`, `.flac`, etc.
3. Check file permissions: scanner needs read access

### Database errors

```bash
# Check database integrity
sqlite3 lyrion-rust.db "PRAGMA integrity_check;"

# View tables
sqlite3 lyrion-rust.db ".tables"

# Count tracks
sqlite3 lyrion-rust.db "SELECT COUNT(*) FROM tracks;"
```

## Performance Tuning

### Scanner optimization

```bash
# Process multiple directories in parallel
./target/release/lyrion-scanner /music/collection1 &
./target/release/lyrion-scanner /music/collection2 --db-path=lyrion-rust.db &
wait
```

### Database optimization

```bash
# Analyze for better query plans
sqlite3 lyrion-rust.db "ANALYZE;"

# Vacuum to reclaim space
sqlite3 lyrion-rust.db "VACUUM;"
```

## Development

### Running in debug mode

```bash
cargo run --bin lyrion-server
```

### Running tests

```bash
cargo test
```

### Checking code

```bash
cargo clippy
cargo fmt --check
```

### Building documentation

```bash
cargo doc --open
```

## What's Working (Phase 1)

✅ Database layer with SQLite
✅ Slimproto protocol (player connections)
✅ HTTP REST API
✅ JSON-RPC endpoint (stubs)
✅ MP3/FLAC metadata parsing
✅ Music library scanner
✅ Player detection and registration

## What's NOT Working Yet

❌ Audio streaming (Phase 2)
❌ Transcoding (Phase 2)
❌ Actual playback control (Phase 2)
❌ Multi-room sync (Phase 3)
❌ Web UI (Phase 4)
❌ Plugins (Phase 5)

For current limitations, see `IMPLEMENTATION_STATUS.md`.

## Next Steps

1. **Try it out**: Build, scan, start server
2. **Connect a player**: Watch for HELO messages
3. **Test the API**: Try the REST endpoints
4. **Report issues**: What works? What doesn't?

## Resources

- Main plan: See comprehensive project plan document
- Implementation status: `IMPLEMENTATION_STATUS.md`
- API docs: `README.md`
- Original Perl code: `/data2/slimserver/`

## Getting Help

Current status: **Phase 1 implementation complete**

This is a working foundation that:
- Accepts Squeezebox connections
- Manages a music database
- Provides an HTTP API

But it doesn't yet stream audio or control playback. That's Phase 2!
