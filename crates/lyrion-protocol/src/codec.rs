//! Slimproto codec for encoding/decoding messages

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use crate::messages::{SlimprotoMessage, Opcode, HeloMessage, StatMessage, IrMessage, ButtonMessage, DisconnectReason};

/// Slimproto codec
/// Message format: [4-byte opcode][4-byte length][payload]
pub struct SlimprotoCodec;

impl Decoder for SlimprotoCodec {
    type Item = SlimprotoMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least 8 bytes for header (opcode + length)
        if src.len() < 8 {
            return Ok(None);
        }

        // Read opcode (4 bytes)
        let opcode_bytes: [u8; 4] = src[0..4].try_into().unwrap();

        // Read length (4 bytes, big-endian)
        let length = u32::from_be_bytes([src[4], src[5], src[6], src[7]]) as usize;

        // Check if we have the full message
        let total_len = 8 + length;
        if src.len() < total_len {
            // Reserve space for the full message
            src.reserve(total_len - src.len());
            return Ok(None);
        }

        // Advance past header
        src.advance(8);

        // Extract payload
        let payload = src.split_to(length);

        // Parse message based on opcode
        let message = match Opcode::from_bytes(&opcode_bytes) {
            Some(Opcode::Helo) => {
                match HeloMessage::parse(&payload) {
                    Ok(helo) => SlimprotoMessage::Helo(helo),
                    Err(e) => {
                        tracing::warn!("Failed to parse HELO: {}", e);
                        SlimprotoMessage::Unknown {
                            opcode: opcode_bytes,
                            data: payload.to_vec(),
                        }
                    }
                }
            }
            Some(Opcode::Stat) => {
                match StatMessage::parse(&payload) {
                    Ok(stat) => SlimprotoMessage::Stat(stat),
                    Err(e) => {
                        tracing::warn!("Failed to parse STAT: {}", e);
                        SlimprotoMessage::Unknown {
                            opcode: opcode_bytes,
                            data: payload.to_vec(),
                        }
                    }
                }
            }
            Some(Opcode::Ir) => {
                match IrMessage::parse(&payload) {
                    Ok(ir) => SlimprotoMessage::Ir(ir),
                    Err(e) => {
                        tracing::warn!("Failed to parse IR: {}", e);
                        SlimprotoMessage::Unknown {
                            opcode: opcode_bytes,
                            data: payload.to_vec(),
                        }
                    }
                }
            }
            Some(Opcode::Butn) => {
                match ButtonMessage::parse(&payload) {
                    Ok(butn) => SlimprotoMessage::Butn(butn),
                    Err(e) => {
                        tracing::warn!("Failed to parse BUTN: {}", e);
                        SlimprotoMessage::Unknown {
                            opcode: opcode_bytes,
                            data: payload.to_vec(),
                        }
                    }
                }
            }
            Some(Opcode::Bye) => SlimprotoMessage::Bye,
            Some(Opcode::Dsco) => {
                let reason = if !payload.is_empty() {
                    DisconnectReason::from_u8(payload[0])
                } else {
                    DisconnectReason::ConnectionClosed
                };
                SlimprotoMessage::Dsco(reason)
            }
            _ => {
                tracing::debug!("Unknown opcode: {:?}", std::str::from_utf8(&opcode_bytes));
                SlimprotoMessage::Unknown {
                    opcode: opcode_bytes,
                    data: payload.to_vec(),
                }
            }
        };

        Ok(Some(message))
    }
}

impl Encoder<SlimprotoMessage> for SlimprotoCodec {
    type Error = std::io::Error;

    fn encode(&mut self, msg: SlimprotoMessage, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        // For now, encoding is not implemented as server mostly receives messages
        // Will be implemented when we need to send commands to players
        tracing::warn!("Encoding not yet implemented for message: {:?}", msg);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_insufficient_data() {
        let mut codec = SlimprotoCodec;
        let mut buf = BytesMut::from(&b"HEL"[..]);

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_header_only() {
        let mut codec = SlimprotoCodec;
        let mut buf = BytesMut::new();

        // Write HELO header with length 20
        buf.put_slice(b"HELO");
        buf.put_u32(20);

        let result = codec.decode(&mut buf).unwrap();
        assert!(result.is_none()); // Payload not yet available
    }
}
