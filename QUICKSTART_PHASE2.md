# Quick Start - Phase 2 (Audio Streaming)

This guide shows how to test the new Phase 2 audio streaming features.

## Prerequisites

1. **Install Transcoding Tools**:
```bash
# Ubuntu/Debian
sudo apt-get install flac lame

# macOS
brew install flac lame

# Arch Linux
sudo pacman -S flac lame
```

2. **Build the Server**:
```bash
cd /data2/slimserver/lyrion-rust
cargo build --release
```

## Test 1: Direct Streaming (No Transcoding)

Stream an MP3 file directly without transcoding.

```bash
# Start server
./target/release/lyrion-server

# In another terminal, stream a track
curl -o test.mp3 http://localhost:9000/stream/1

# Play it
mpg123 test.mp3
```

Expected output in server logs:
```
[INFO lyrion_server::streaming] Streaming track 1 (Song Title) - file: /music/song.mp3
```

## Test 2: Transcoding (FLAC → MP3)

Stream a FLAC file transcoded to MP3 on-the-fly.

```bash
# Stream with format parameter
curl -o transcoded.mp3 'http://localhost:9000/stream/2?format=mp3'

# Verify it's MP3
file transcoded.mp3
# Output: MPEG ADTS, layer III, v1, 320 kbps

# Play it
mpg123 transcoded.mp3
```

Expected output in server logs:
```
[INFO lyrion_server::streaming] Streaming track 2 (FLAC Song) - transcoding file: /music/song.flac
[INFO lyrion_server::streaming] Transcoding flac -> mp3: /music/song.flac
```

## Test 3: With Real Squeezebox Player

1. **Configure Player**:
   - Power on your Squeezebox
   - Go to: Settings → Advanced → Network
   - Set server IP to your machine's IP
   - Set port to 3483
   - Restart player

2. **Check Connection**:
```bash
# Watch server logs
tail -f lyrion-server.log

# Should see:
# [INFO lyrion_protocol] New connection from 192.168.1.X
# [INFO lyrion_protocol] Player HELO: MAC=xx:xx:xx:xx:xx:xx, Device=4
```

3. **Browse and Play**:
   - Use player remote to browse library
   - Select a track
   - Press Play

Expected logs:
```
[INFO lyrion_server::streaming] Streaming track 123 (Artist - Song) - file: /music/song.flac
[INFO lyrion_server::streaming] Transcoding flac -> mp3
[DEBUG lyrion_core::streaming] Streaming state transition: Idle -> TrackWait
[DEBUG lyrion_core::streaming] Playing state transition: Stopped -> Buffering
[DEBUG lyrion_core::streaming] Playing state transition: Buffering -> Playing
```

## Test 4: Playlist Queue

Use curl to build a playlist:

```bash
# Play track 1
curl -X POST http://localhost:9000/jsonrpc.js \
  -H 'Content-Type: application/json' \
  -d '{
    "method": "slim.request",
    "params": ["player_mac", ["play", "file:///music/track1.mp3"]]
  }'

# Add track 2 to queue
curl -X POST http://localhost:9000/jsonrpc.js \
  -d '{
    "method": "slim.request",
    "params": ["player_mac", ["playlist", "add", "file:///music/track2.mp3"]]
  }'

# Skip to next
curl -X POST http://localhost:9000/jsonrpc.js \
  -d '{
    "method": "slim.request",
    "params": ["player_mac", ["playlist", "index", "+1"]]
  }'
```

## Test 5: Playback Controls

```bash
# Pause
curl -X POST http://localhost:9000/jsonrpc.js \
  -d '{"method":"slim.request","params":["player_mac",["pause"]]}'

# Resume
curl -X POST http://localhost:9000/jsonrpc.js \
  -d '{"method":"slim.request","params":["player_mac",["pause","0"]]}'

# Stop
curl -X POST http://localhost:9000/jsonrpc.js \
  -d '{"method":"slim.request","params":["player_mac",["stop"]]}'
```

## Test 6: Multiple Concurrent Streams

Test server performance with multiple simultaneous streams:

```bash
# Terminal 1
curl -o stream1.mp3 http://localhost:9000/stream/1 &

# Terminal 2  
curl -o stream2.mp3 'http://localhost:9000/stream/2?format=mp3' &

# Terminal 3
curl -o stream3.mp3 http://localhost:9000/stream/3 &

# Monitor CPU/memory
top -p $(pgrep lyrion-server)
```

Expected: All streams should work without dropouts.

## Troubleshooting

### No Audio Output

Check transcode tools are installed:
```bash
which flac
which lame
```

Test manually:
```bash
flac -dcs /path/to/file.flac | lame -b 320 - test.mp3
```

### 404 Not Found

Check track exists in database:
```bash
sqlite3 lyrion-rust.db "SELECT id, url FROM tracks LIMIT 10;"
```

Verify file paths are correct.

### Transcoding Errors

Enable debug logging:
```bash
RUST_LOG=debug ./target/release/lyrion-server
```

Check for errors like:
```
[ERROR lyrion_transcode] Failed to spawn flac: No such file or directory
```

### Player Won't Connect

Check firewall:
```bash
sudo ufw allow 3483/tcp
sudo ufw allow 9000/tcp
```

Test connectivity:
```bash
telnet server-ip 3483
```

## Performance Monitoring

Watch streaming activity:
```bash
# See active connections
netstat -an | grep :9000

# Monitor transcoding processes
ps aux | grep -E 'flac|lame'

# Watch logs
tail -f lyrion-server.log | grep streaming
```

## What's Working

✅ Direct streaming (MP3, FLAC, WAV)
✅ Transcoding (FLAC→MP3, WAV→MP3)
✅ Queue management
✅ Play/pause/stop controls
✅ Multiple concurrent streams
✅ Chunked transfer encoding
✅ Automatic format detection

## What's Not Working Yet

❌ Seek (jump to time in track)
❌ Volume control
❌ ICY metadata (Shoutcast-style)
❌ Gapless playback
❌ Multi-room sync (Phase 3)

## Next Phase

Phase 3 will add multi-room synchronization for playing the same audio on multiple Squeezeboxes with < 10ms accuracy.

See `PHASE2_COMPLETE.md` for full details.
