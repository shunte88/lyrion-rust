//! Slimproto message types
//! Message format: [4-byte opcode][4-byte length][payload]

use bytes::Buf;

/// Message opcodes (4 bytes ASCII)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Helo, // Player hello
    Stat, // Status update
    Ir,   // IR remote button
    Butn, // Physical button
    Anic, // Animation complete
    Body, // HTTP body
    Bye,  // Goodbye
    Dbug, // Debug
    Dsco, // Disconnect
    Knob, // Knob turn
    Meta, // HTTP metadata
    Rawi, // Raw IR
    Resp, // HTTP response
    Setd, // Settings
    Ureq, // Update request
    Alss, // Ambient light sensor
    Shut, // Shutdown
}

impl Opcode {
    pub fn as_bytes(&self) -> [u8; 4] {
        match self {
            Self::Helo => *b"HELO",
            Self::Stat => *b"STAT",
            Self::Ir => *b"IR  ",
            Self::Butn => *b"BUTN",
            Self::Anic => *b"ANIC",
            Self::Body => *b"BODY",
            Self::Bye => *b"BYE!",
            Self::Dbug => *b"DBUG",
            Self::Dsco => *b"DSCO",
            Self::Knob => *b"KNOB",
            Self::Meta => *b"META",
            Self::Rawi => *b"RAWI",
            Self::Resp => *b"RESP",
            Self::Setd => *b"SETD",
            Self::Ureq => *b"UREQ",
            Self::Alss => *b"ALSS",
            Self::Shut => *b"SHUT",
        }
    }

    pub fn from_bytes(bytes: &[u8; 4]) -> Option<Self> {
        match bytes {
            b"HELO" => Some(Self::Helo),
            b"STAT" => Some(Self::Stat),
            b"IR  " => Some(Self::Ir),
            b"BUTN" => Some(Self::Butn),
            b"ANIC" => Some(Self::Anic),
            b"BODY" => Some(Self::Body),
            b"BYE!" => Some(Self::Bye),
            b"DBUG" => Some(Self::Dbug),
            b"DSCO" => Some(Self::Dsco),
            b"KNOB" => Some(Self::Knob),
            b"META" => Some(Self::Meta),
            b"RAWI" => Some(Self::Rawi),
            b"RESP" => Some(Self::Resp),
            b"SETD" => Some(Self::Setd),
            b"UREQ" => Some(Self::Ureq),
            b"ALSS" => Some(Self::Alss),
            b"SHUT" => Some(Self::Shut),
            _ => None,
        }
    }
}

/// Main message enum
#[derive(Debug, Clone)]
pub enum SlimprotoMessage {
    Helo(HeloMessage),
    Stat(StatMessage),
    Ir(IrMessage),
    Butn(ButtonMessage),
    Bye,
    Dsco(DisconnectReason),
    Unknown { opcode: [u8; 4], data: Vec<u8> },
}

/// HELO message (player hello)
/// Unpacked from Slimproto.pm line 961-970
#[derive(Debug, Clone)]
pub struct HeloMessage {
    pub device_id: u8,
    pub revision: u8,
    pub mac: [u8; 6],
    pub uuid: Option<String>,
    pub wlan_channellist: u16,
    pub bytes_received: u64,
    pub language: String,
}

impl HeloMessage {
    /// Parse HELO message from payload
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 18 {
            return Err("HELO message too short".to_string());
        }

        let device_id = data[0];
        let revision = data[1];

        let mac = [
            data[2], data[3], data[4],
            data[5], data[6], data[7],
        ];

        let (uuid, offset) = if data.len() >= 36 {
            let uuid_bytes = &data[8..24];
            let uuid = hex::encode(uuid_bytes);
            (Some(uuid), 24)
        } else {
            (None, 8)
        };

        let wlan_raw = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let wlan_channellist = wlan_raw & 0x3fff;

        let bytes_received_h = u32::from_be_bytes([
            data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5],
        ]);
        let bytes_received_l = u32::from_be_bytes([
            data[offset + 6], data[offset + 7],
            data[offset + 8], data[offset + 9],
        ]);
        let bytes_received = ((bytes_received_h as u64) << 32) | (bytes_received_l as u64);

        let lang_bytes = &data[offset + 10..offset + 12];
        let language = String::from_utf8_lossy(lang_bytes).to_string();

        Ok(Self {
            device_id,
            revision,
            mac,
            uuid,
            wlan_channellist,
            bytes_received,
            language,
        })
    }

    pub fn mac_string(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.mac[0], self.mac[1], self.mac[2],
            self.mac[3], self.mac[4], self.mac[5]
        )
    }
}

/// STAT message (status update)
/// From Slimproto.pm line 711-722
#[derive(Debug, Clone)]
pub struct StatMessage {
    pub event: StatEvent,
    pub num_crlf: u8,
    pub mas_initialized: u8,
    pub mas_mode: u8,
    pub rptr: u32,
    pub wptr: u32,
    pub bytes_received: u64,
    pub signal_strength: u16,
    pub jiffies: u32,
    pub output_buffer_size: u32,
    pub output_buffer_fullness: u32,
    pub elapsed_seconds: u32,
    pub elapsed_milliseconds: u16,
    pub timestamp: u32,
}

impl StatMessage {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 52 {
            return Err("STAT message too short".to_string());
        }

        let mut buf = &data[..];

        Ok(Self {
            event: StatEvent::from_u32(buf.get_u32())
                .ok_or("Invalid STAT event")?,
            num_crlf: buf.get_u8(),
            mas_initialized: buf.get_u8(),
            mas_mode: buf.get_u8(),
            rptr: buf.get_u32(),
            wptr: buf.get_u32(),
            bytes_received: buf.get_u64(),
            signal_strength: buf.get_u16(),
            jiffies: buf.get_u32(),
            output_buffer_size: buf.get_u32(),
            output_buffer_fullness: buf.get_u32(),
            elapsed_seconds: buf.get_u32(),
            elapsed_milliseconds: buf.get_u16(),
            timestamp: buf.get_u32(),
        })
    }
}

/// STAT event codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatEvent {
    TimerStart = 0x0,
    TimerStartStop = 0x1,
    PauseUnpause = 0x2,
    UnderrunTrackStart = 0x3,
    TrackStart = 0x4,
    Unknown,
}

impl StatEvent {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x0 => Some(Self::TimerStart),
            0x1 => Some(Self::TimerStartStop),
            0x2 => Some(Self::PauseUnpause),
            0x3 => Some(Self::UnderrunTrackStart),
            0x4 => Some(Self::TrackStart),
            _ => Some(Self::Unknown),
        }
    }
}

/// IR message (infrared remote)
#[derive(Debug, Clone)]
pub struct IrMessage {
    pub time: u32,
    pub format: u32,
    pub bits: u32,
    pub code: u32,
}

impl IrMessage {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 16 {
            return Err("IR message too short".to_string());
        }

        let mut buf = &data[..];
        Ok(Self {
            time: buf.get_u32(),
            format: buf.get_u32(),
            bits: buf.get_u32(),
            code: buf.get_u32(),
        })
    }
}

/// BUTN message (physical button press)
#[derive(Debug, Clone)]
pub struct ButtonMessage {
    pub time: u32,
    pub code: u32,
}

impl ButtonMessage {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 8 {
            return Err("BUTN message too short".to_string());
        }

        let mut buf = &data[..];
        Ok(Self {
            time: buf.get_u32(),
            code: buf.get_u32(),
        })
    }
}

/// Disconnect reason
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum DisconnectReason {
    ConnectionClosed = 0x00,
    UnreachableConnection = 0x01,
    Unknown(u8),
}

impl DisconnectReason {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => Self::ConnectionClosed,
            0x01 => Self::UnreachableConnection,
            other => Self::Unknown(other),
        }
    }
}

/// Server-to-client stream control commands
/// Used for playback control and synchronization
#[derive(Debug, Clone)]
pub enum StreamCommand {
    /// Start streaming ('s')
    /// Tells the player to start streaming from a URL
    Start {
        /// Auto-start mode: 0=paused, 1=autostart (75%), 2=direct no autostart, 3=direct with autostart
        autostart: u8,
        /// Format byte: 'm'=mp3, 'p'=pcm, 'f'=flac, 'o'=ogg, etc.
        format: u8,
        /// PCM sample size: 0=8bit, 1=16bit, 2=20bit, 3=32bit, '?'=unknown
        pcm_sample_size: u8,
        /// PCM sample rate: 0=11kHz, 1=22kHz, 2=32kHz, 3=44.1kHz, 4=48kHz, etc., '?'=unknown
        pcm_sample_rate: u8,
        /// PCM channels: 1=mono, 2=stereo, '?'=unknown
        pcm_channels: u8,
        /// PCM endianness: 0=big, 1=little, '?'=unknown
        pcm_endian: u8,
        /// Buffer threshold in KB before playback starts
        buffer_threshold: u8,
        /// S/PDIF enable (usually 0)
        spdif_enable: u8,
        /// Transition duration in seconds
        transition_duration: u8,
        /// Transition type: 0=none, 1=crossfade, 2=fade_in, 3=fade_out, 4=fade_in_out
        transition_type: u8,
        /// Flags: bit 0-1=polarity, bit 2-3=output channels, bit 6=reconnect, bit 7=loop
        flags: u8,
        /// Output threshold
        output_threshold: u8,
        /// Reserved for slave streams (usually 0)
        reserved: u8,
        /// Replay gain value (0 = disabled)
        replay_gain: u32,
        /// Server port
        server_port: u16,
        /// Server IP address (0 = use control connection IP)
        server_ip: u32,
        /// HTTP request string (e.g., "GET /stream/123 HTTP/1.1\r\n\r\n")
        request_string: String,
    },
    /// Unpause/resume playback ('u')
    Unpause {
        /// Interval in milliseconds to resume at (0 for immediate)
        interval_ms: u32
    },
    /// Pause for a specific interval ('p')
    /// Used for sync adjustment when player is behind
    PauseFor {
        /// Duration to pause in milliseconds
        interval_ms: u32
    },
    /// Skip ahead by a specific interval ('a')
    /// Used for sync adjustment when player is ahead
    SkipAhead {
        /// Duration to skip in milliseconds
        interval_ms: u32
    },
    /// Stop playback ('t')
    /// Stops streaming and clears buffer
    Stop,
}

impl StreamCommand {
    /// Encode stream command to bytes
    /// Format: [2-byte length] [4-byte opcode] [payload]
    /// Length includes the 4-byte opcode
    pub fn encode(&self) -> Vec<u8> {
        let mut payload = Vec::new();

        match self {
            StreamCommand::Start {
                autostart,
                format,
                pcm_sample_size,
                pcm_sample_rate,
                pcm_channels,
                pcm_endian,
                buffer_threshold,
                spdif_enable,
                transition_duration,
                transition_type,
                flags,
                output_threshold,
                reserved,
                replay_gain,
                server_port,
                server_ip,
                request_string,
            } => {
                // Command 's' + parameters (24 bytes total header)
                payload.push(b's');
                payload.push(*autostart);
                payload.push(*format);
                payload.push(*pcm_sample_size);
                payload.push(*pcm_sample_rate);
                payload.push(*pcm_channels);
                payload.push(*pcm_endian);
                payload.push(*buffer_threshold);
                payload.push(*spdif_enable);
                payload.push(*transition_duration);
                payload.push(*transition_type);
                payload.push(*flags);
                payload.push(*output_threshold);
                payload.push(*reserved);
                payload.extend_from_slice(&replay_gain.to_be_bytes());
                payload.extend_from_slice(&server_port.to_be_bytes());
                payload.extend_from_slice(&server_ip.to_be_bytes());

                // HTTP request string
                payload.extend_from_slice(request_string.as_bytes());

                // Build frame: [2-byte length] [4-byte opcode] [payload]
                let mut buf = Vec::new();
                let frame_len = 4 + payload.len(); // opcode (4) + payload
                buf.extend_from_slice(&(frame_len as u16).to_be_bytes());
                buf.extend_from_slice(b"strm");
                buf.extend_from_slice(&payload);
                return buf;
            }
            StreamCommand::Unpause { interval_ms } => {
                // Payload: 1 (command) + 1 (autostart) + 4 (interval) = 6 bytes
                payload.push(b'u'); // command
                payload.push(0); // autostart flag
                payload.extend_from_slice(&interval_ms.to_be_bytes());
            }
            StreamCommand::PauseFor { interval_ms } => {
                // Payload: 1 (command) + 1 (reserved) + 4 (interval) + 2 (padding) = 8 bytes
                payload.push(b'p'); // command
                payload.push(0); // reserved
                payload.extend_from_slice(&interval_ms.to_be_bytes());
                payload.extend_from_slice(&[0, 0]); // padding
            }
            StreamCommand::SkipAhead { interval_ms } => {
                // Payload: 1 (command) + 1 (reserved) + 4 (interval) + 2 (padding) = 8 bytes
                payload.push(b'a'); // command
                payload.push(0); // reserved
                payload.extend_from_slice(&interval_ms.to_be_bytes());
                payload.extend_from_slice(&[0, 0]); // padding
            }
            StreamCommand::Stop => {
                // Payload: 1 (command) + 1 (reserved) + 4 (timestamp) + 2 (padding) = 8 bytes
                payload.push(b't'); // command
                payload.push(0); // reserved
                payload.extend_from_slice(&[0, 0, 0, 0]); // timestamp (0)
                payload.extend_from_slice(&[0, 0]); // padding
            }
        }

        // Build frame: [2-byte length] [4-byte opcode] [payload]
        let mut buf = Vec::new();
        let frame_len = 4 + payload.len(); // opcode (4) + payload
        buf.extend_from_slice(&(frame_len as u16).to_be_bytes());
        buf.extend_from_slice(b"strm");
        buf.extend_from_slice(&payload);
        buf
    }
}

/// Audio gain control command (audg)
/// Sets volume levels with optional balance and digital volume control
#[derive(Debug, Clone)]
pub struct AudioGainCommand {
    /// Old gain for left channel (fixed point)
    pub old_gain_left: u32,
    /// Old gain for right channel (fixed point)
    pub old_gain_right: u32,
    /// Digital volume control flag
    pub digital_volume_control: u8,
    /// Preamp level (255 = 0dB, lower = boost)
    pub preamp: u8,
    /// New gain for left channel (fixed point)
    pub new_gain_left: u32,
    /// New gain for right channel (fixed point)
    pub new_gain_right: u32,
}

impl AudioGainCommand {
    /// Create a new audio gain command from volume percentage (0-100)
    pub fn from_volume(volume: u8, old_volume: u8) -> Self {
        // Convert volume percentage to fixed point gain
        // Volume 0 = mute, 100 = full volume
        let volume_to_gain = |v: u8| -> u32 {
            if v == 0 {
                0
            } else {
                // Simple linear scale for now
                // In a real implementation, this would use dB conversion
                let gain_percent = v as f64 / 100.0;
                (gain_percent * 65536.0) as u32
            }
        };

        Self {
            old_gain_left: volume_to_gain(old_volume),
            old_gain_right: volume_to_gain(old_volume),
            digital_volume_control: 1, // Enable digital volume control
            preamp: 255, // 0dB preamp (no boost)
            new_gain_left: volume_to_gain(volume),
            new_gain_right: volume_to_gain(volume),
        }
    }

    /// Encode to Slimproto frame format
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Frame format: [2-byte length] [4-byte opcode] [payload]
        let payload_len = 4 + 4 + 1 + 1 + 4 + 4; // 18 bytes
        let frame_len = 4 + payload_len; // opcode + payload = 22

        buf.extend_from_slice(&(frame_len as u16).to_be_bytes());
        buf.extend_from_slice(b"audg");

        // Payload
        buf.extend_from_slice(&self.old_gain_left.to_be_bytes());
        buf.extend_from_slice(&self.old_gain_right.to_be_bytes());
        buf.push(self.digital_volume_control);
        buf.push(self.preamp);
        buf.extend_from_slice(&self.new_gain_left.to_be_bytes());
        buf.extend_from_slice(&self.new_gain_right.to_be_bytes());

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_command_pause_for() {
        let cmd = StreamCommand::PauseFor { interval_ms: 50 };
        let encoded = cmd.encode();

        // Frame format: [2-byte length] [4-byte opcode] [payload]
        // Check length (12 = 4 byte opcode + 8 byte payload)
        assert_eq!(u16::from_be_bytes([encoded[0], encoded[1]]), 12);
        // Check opcode
        assert_eq!(&encoded[2..6], b"strm");
        // Check command
        assert_eq!(encoded[6], b'p');
        // Check interval (50ms)
        assert_eq!(u32::from_be_bytes([encoded[8], encoded[9], encoded[10], encoded[11]]), 50);
    }

    #[test]
    fn test_stream_command_skip_ahead() {
        let cmd = StreamCommand::SkipAhead { interval_ms: 100 };
        let encoded = cmd.encode();

        assert_eq!(&encoded[0..4], b"strm");
        assert_eq!(encoded[8], b'a');
        assert_eq!(u32::from_be_bytes([encoded[10], encoded[11], encoded[12], encoded[13]]), 100);
    }

    #[test]
    fn test_stream_command_start() {
        let request = "GET /stream/123 HTTP/1.1\r\n\r\n".to_string();
        let cmd = StreamCommand::Start {
            autostart: 1,
            format: b'm',  // mp3
            pcm_sample_size: b'?',
            pcm_sample_rate: b'?',
            pcm_channels: b'?',
            pcm_endian: b'?',
            buffer_threshold: 30,
            spdif_enable: 0,
            transition_duration: 0,
            transition_type: 0,
            flags: 0,
            output_threshold: 1,
            reserved: 0,
            replay_gain: 0,
            server_port: 9000,
            server_ip: 0,  // use control connection IP
            request_string: request.clone(),
        };

        let encoded = cmd.encode();

        // Check opcode
        assert_eq!(&encoded[0..4], b"strm");

        // Check length (24 + request string length)
        let expected_len = 24 + request.len();
        assert_eq!(u32::from_be_bytes([encoded[4], encoded[5], encoded[6], encoded[7]]), expected_len as u32);

        // Check command (byte 8)
        assert_eq!(encoded[8], b's');

        // Check autostart (byte 9)
        assert_eq!(encoded[9], 1);

        // Check format (byte 10)
        assert_eq!(encoded[10], b'm');

        // Check buffer threshold (byte 15)
        assert_eq!(encoded[15], 30);

        // Check output threshold (byte 20)
        assert_eq!(encoded[20], 1);

        // Check replay_gain (bytes 22-25)
        assert_eq!(u32::from_be_bytes([encoded[22], encoded[23], encoded[24], encoded[25]]), 0);

        // Check server port (bytes 26-27)
        assert_eq!(u16::from_be_bytes([encoded[26], encoded[27]]), 9000);

        // Check server IP (bytes 28-31)
        assert_eq!(u32::from_be_bytes([encoded[28], encoded[29], encoded[30], encoded[31]]), 0);

        // Check request string starts at byte 32
        assert_eq!(&encoded[32..], request.as_bytes());

        // Verify total length (8-byte frame header + 24-byte payload header + request string)
        assert_eq!(encoded.len(), 8 + 24 + request.len());
    }
}
