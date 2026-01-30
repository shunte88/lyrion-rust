# Slimproto Player Connection Guide

## Issue: Players Not Connecting

### Root Cause
Squeezelite (and other Squeezebox players) use **UDP autodiscovery** by default to find the Slimserver. The Lyrion Rust server currently only implements the **TCP Slimproto protocol** on port 3483, but not the UDP discovery protocol.

### Solution
Connect players explicitly by specifying the server address.

## Squeezelite Connection

### Command Format
```bash
squeezelite -n <player_name> -s <server_address>:<port>
```

### Example
```bash
# Start squeezelite with explicit server connection
squeezelite -n mythy -s localhost:3483 -d all=debug

# For remote connections
squeezelite -n kitchen -s 192.168.1.100:3483

# Running in background
squeezelite -n mythy -s localhost:3483 > /tmp/squeezelite.log 2>&1 &
```

### Key Parameters
- `-n <name>` - Player name (displayed in UI)
- `-s <server>:<port>` - Server address (bypasses autodiscovery)
- `-d all=debug` - Enable debug logging (optional)
- `-m <mac>` - Set MAC address manually (optional)

## Verification

### Check Server Logs
```bash
tail -f /tmp/lyrion-server.log | grep -i helo
```

Expected output:
```
INFO lyrion_protocol::server: New connection from 127.0.0.1:xxxxx
INFO lyrion_protocol::server: Player HELO: MAC=c4:62:37:01:98:40, Device=12, Revision=0
```

### Check API Endpoint
```bash
curl http://localhost:9000/api/v1/players | jq .
```

Expected output:
```json
[
  {
    "mac": "c4:62:37:01:98:40",
    "device_id": 12,
    "revision": 0,
    "uuid": "00000000000000000000000000000000"
  }
]
```

### Check Network Connection
```bash
netstat -tn | grep 3483
```

Expected output:
```
tcp  0  0  127.0.0.1:3483  127.0.0.1:xxxxx  ESTABLISHED
```

## Hardware Squeezebox Players

For hardware Squeezebox devices (SB3, Boom, Touch, Radio):

1. Access player settings menu
2. Navigate to: **Settings → Advanced → Server Settings**
3. Set **Server Address**: `<server_ip>:3483`
4. Disable or set **Switch Server**: Never
5. Restart player

## Future Enhancement: UDP Discovery

To support autodiscovery, we need to implement:

### UDP Discovery Protocol (Port 3483)
- **Discovery Request**: Client sends UDP broadcast `d` (0x64)
- **Discovery Response**: Server responds with JSON containing:
  ```json
  {
    "uuid": "server-uuid",
    "name": "Lyrion Music Server",
    "vers": "0.1.0",
    "JSON": "/jsonrpc.js",
    "host": "server-hostname",
    "port": 3483
  }
  ```

### Implementation Location
- File: `crates/lyrion-protocol/src/discovery.rs` (new)
- Bind UDP socket on 0.0.0.0:3483
- Listen for discovery packets
- Respond with server info

### Reference
- Perl implementation: `/data2/slimserver/Slim/Networking/Discovery.pm`

## Troubleshooting

### Player Not Appearing in UI
1. Check server is running: `ps aux | grep lyrion-server`
2. Check port is listening: `netstat -tuln | grep 3483`
3. Check player connected with `-s` flag
4. Check server logs for HELO message
5. Verify API returns player: `curl http://localhost:9000/api/v1/players`

### Connection Refused
- Ensure lyrion-server is running
- Check firewall rules (allow port 3483)
- Verify server is listening on correct interface (0.0.0.0:3483)

### Player Connects but Disconnects Immediately
- Check server logs for error messages
- Verify Slimproto codec is parsing messages correctly
- Enable debug logging: `RUST_LOG=debug lyrion-server`

## Status

✅ **Working**: TCP Slimproto protocol (port 3483)
✅ **Working**: Player registration and HELO handling
✅ **Working**: UDP autodiscovery protocol (legacy + TLV formats)
⏳ **TODO**: Player browsing/control commands

## Autodiscovery Status

✅ **Implemented**: Full UDP discovery support on port 3483
- Legacy format: SLIMP3, old Squeezebox devices
- Modern TLV format: Newer devices, squeezelite, monitoring tools
- Supported TLV tags: NAME, IPAD, JSON, VERS, UUID, JVID

**How to Use Autodiscovery**:
```bash
# No -s flag needed anymore!
squeezelite -n mythy

# Squeezelite will automatically discover and connect
```

**Server Logs Confirm**:
```
INFO lyrion_protocol::discovery: UDP discovery server listening on 0.0.0.0:3483
INFO lyrion_protocol::discovery: TLV discovery: 6 entries
INFO lyrion_protocol::discovery: Sending TLV discovery response to 192.168.1.210:35975
INFO lyrion_protocol::server: Player HELO: MAC=c4:62:37:01:98:40, Device=12
```

**Squeezelite Logs Confirm**:
```
discover_server:800 sending discovery
discover_server:811 got response from: 192.168.1.210:3483
slimproto:903 connecting to 192.168.1.210:3483
slimproto:942 connected
```

---

**Last Updated**: 2026-01-29
**Tested With**: Squeezelite 2.0.0-1517, UDP discovery confirmed working
