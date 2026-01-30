# Audio Playback End-to-End Debug Session

**Date**: 2026-01-29
**Goal**: Get squeezelite to play audio from lyrion-rust server
**Status**: 🟡 Critical progress made, one blocking issue identified

---

## Session Summary

### Major Achievements ✅

1. **Fixed Critical Socket Communication Bug**
   - **Problem**: "FATAL: slimproto packet too big: 29556 > 4096"
   - **Root Cause**: Using `socket.into_split()` with Framed codec caused byte alignment issues
   - **Solution**: Used `Arc<Mutex<TcpStream>>` for coordinated read/write access
   - **Result**: Squeezelite now receives Slimproto packets correctly

2. **Implemented Complete `strm 's'` Command**
   - Added Start variant to StreamCommand enum (crates/lyrion-protocol/src/messages.rs:298)
   - Proper encoding: 60 bytes (8-byte header + 24-byte command + 28-byte HTTP request)
   - Hex verified: `73 74 72 6d 00 00 00 34 ...` = "strm" + length 52

3. **Correct PCM Parameters for WAV Files**
   - Changed from all '?' (0x3F) to proper values:
     - pcm_sample_size: 1 (16-bit)
     - pcm_sample_rate: 3 (44.1kHz)
     - pcm_channels: 2 (stereo)
     - pcm_endian: 1 (little-endian)

4. **HTTP Protocol Fixes**
   - Changed HTTP/1.1 to HTTP/1.0 (per LMS spec)
   - Set real server IP: 192.168.1.210 (not 0)

### Current Blocking Issue ⚠️

**HTTP Streaming Endpoint Hangs Indefinitely**

**Symptom**:
```bash
curl -I http://localhost:9000/stream/924
# Hangs for 2+ minutes, receives 0 bytes
```

**Impact**:
- Squeezelite receives `strm` command successfully (no errors)
- Squeezelite likely tries to fetch HTTP stream
- Stream request hangs → squeezelite times out after 36s
- Player disconnects with "No messages from server - connection dead"

**Evidence**:
- `/api/v1/tracks` works fine (returns data instantly)
- `/stream/924` hangs completely (tested with curl -I)
- No log entries for stream requests (handler never executes?)
- Server doesn't log any errors

**Location**: `crates/lyrion-server/src/streaming.rs:26` (`stream_track` function)

---

## Technical Details

### Slimproto Command Flow

1. **HELO** (Client → Server): Player announces itself
   ```
   Device: 12 (SqueezePlay/squeezelite)
   MAC: c4:62:37:01:98:40
   ```

2. **JSON-RPC play** (User → Server): `["c4:62:37:01:98:40", ["play", 924]]`

3. **strm 's'** (Server → Client): Start streaming command
   ```
   Opcode: strm
   Length: 52
   Command: 's'
   Format: 'p' (PCM)
   Server: 192.168.1.210:9000
   Request: "GET /stream/924 HTTP/1.0\r\n\r\n"
   ```

4. **HTTP GET** (Client → Server): *Expected but not working*
   ```
   GET /stream/924 HTTP/1.0
   ```

5. **Audio Stream** (Server → Client): *Should stream WAV data*

### Code Changes Made

#### 1. StreamCommand::Start Implementation
**File**: `crates/lyrion-protocol/src/messages.rs`

```rust
pub enum StreamCommand {
    Start {
        autostart: u8,
        format: u8,  // 'p' for PCM, 'm' for MP3, etc.
        pcm_sample_size: u8,  // 1=16bit
        pcm_sample_rate: u8,  // 3=44.1kHz
        pcm_channels: u8,     // 2=stereo
        pcm_endian: u8,       // 1=little-endian
        buffer_threshold: u8,
        // ... other fields
        request_string: String,
    },
    // ... other variants
}
```

#### 2. Bidirectional Socket Communication
**File**: `crates/lyrion-protocol/src/server.rs`

**Before** (broken):
```rust
let (read_half, mut write_half) = socket.into_split();
let mut framed = Framed::new(read_half, SlimprotoCodec);
// write_half used separately → byte alignment issues
```

**After** (fixed):
```rust
let socket = Arc::new(TokioMutex::new(socket));
// Coordinated access for both read and write
let mut sock = socket.lock().await;
sock.write_all(&data).await?;
```

#### 3. JSON-RPC Play Handler
**File**: `crates/lyrion-server/src/jsonrpc.rs`

Added complete implementation that:
- Looks up track in database
- Determines format and PCM parameters
- Builds strm command
- Sends to player via SlimprotoServer

### Test Results

| Test | Result | Notes |
|------|--------|-------|
| Server starts | ✅ | Listening on 3483 (Slimproto) and 9000 (HTTP) |
| Player connects | ✅ | HELO received, player registered |
| UDP discovery | ✅ | Squeezelite finds server automatically |
| Send strm command | ✅ | 60 bytes sent, no errors |
| Squeezelite receives strm | ✅ | No "packet too big" error |
| HTTP /api/v1/tracks | ✅ | Returns data instantly |
| HTTP /stream/924 | ❌ | Hangs indefinitely, 0 bytes received |
| Audio playback | ❌ | Blocked by stream endpoint hang |

---

## Next Steps

### Immediate Priority

**Debug HTTP Streaming Endpoint** (Task #9)

1. Add debug logging to `stream_track` function:
   ```rust
   tracing::info!("stream_track called for track_id: {}", track_id);
   tracing::info!("Looking up track in database...");
   let track = Track::find_by_id(&state.db_pool, track_id).await?;
   tracing::info!("Track found: {}", track.url);
   ```

2. Check if handler is even being called:
   - Add middleware logging
   - Check Axum router setup
   - Verify route is registered

3. Identify blocking point:
   - Database query?
   - File I/O?
   - Transcoding pipeline?

4. Test with simpler endpoint:
   - Create `/test-stream` that returns static data
   - Verify basic streaming works

### Alternative Approaches

**Option A**: Compare with real LMS
- Capture Wireshark trace of working LMS
- Compare byte-by-byte with our implementation
- Identify any missing commands or handshakes

**Option B**: Implement missing Slimproto commands first
- `audg` (audio gain/volume)
- `setd` (set display)
- Send these before `strm` to match LMS flow

**Option C**: Use official squeezelite debug build
- Recompile squeezelite with full debug symbols
- Add logging to see exact strm processing
- Understand why it's not acting on our command

---

## Session Timeline

1. **10:29** - Started squeezelite
2. **10:31** - Started lyrion-server, player connected
3. **10:45-10:57** - Debugged "packet too big" error
4. **11:00** - Fixed socket splitting issue
5. **11:02** - Fixed PCM parameters
6. **11:04** - Tested with real IP address
7. **11:06** - Discovered HTTP streaming hang

**Total Debug Time**: ~37 minutes
**Breakthroughs**: 2 (socket fix, PCM params)
**Blocking Issues**: 1 (stream endpoint)

---

## Key Files Modified

```
crates/lyrion-protocol/src/messages.rs    (+100 lines) - StreamCommand::Start
crates/lyrion-protocol/src/server.rs      (+50 lines)  - Bidirectional comms
crates/lyrion-server/src/jsonrpc.rs       (+80 lines)  - Play command handler
```

## Diagnostic Commands

```bash
# Check server status
ps aux | grep lyrion-server
netstat -tuln | grep -E "3483|9000"

# Test endpoints
curl http://localhost:9000/api/v1/players
curl http://localhost:9000/api/v1/tracks?limit=1
curl -I http://localhost:9000/stream/924  # Currently hangs!

# Monitor logs
tail -f server-realip.log
tail -f /tmp/squeezelite-test2.log

# Send play command
curl -X POST http://localhost:9000/jsonrpc.js \
  -H 'Content-Type: application/json' \
  -d '{"id": 1, "method": "slim.request", "params": ["c4:62:37:01:98:40", ["play", 924]]}'
```

---

**Status**: Ready for final debug push to fix streaming endpoint
