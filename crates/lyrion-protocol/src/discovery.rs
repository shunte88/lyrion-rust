//! UDP discovery protocol for Squeezebox players
//!
//! Implements two discovery formats:
//! 1. Legacy format (SLIMP3, old Squeezebox): 'd' + device info → 'D' + hostname
//! 2. Modern TLV format (newer devices, squeezelite): 'e' + TLVs → 'E' + TLVs

use anyhow::{Context, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

/// Discovery packet types
#[derive(Debug)]
pub enum DiscoveryRequest {
    /// Legacy discovery: 'd' + device_id + revision + MAC
    Legacy {
        device_id: u8,
        revision: u8,
        mac: [u8; 6],
    },
    /// Modern TLV discovery: 'e' + TLV entries
    Tlv { tlvs: Vec<TlvEntry> },
}

/// TLV (Type-Length-Value) entry
#[derive(Debug, Clone)]
pub struct TlvEntry {
    pub tag: [u8; 4],      // 4-byte type
    pub value: Vec<u8>,    // Variable-length value (0-255 bytes)
}

impl TlvEntry {
    pub fn new(tag: &[u8; 4], value: Vec<u8>) -> Self {
        Self {
            tag: *tag,
            value,
        }
    }

    pub fn tag_str(&self) -> String {
        String::from_utf8_lossy(&self.tag).to_string()
    }
}

/// Discovery response builder
pub struct DiscoveryResponse {
    server_name: String,
    server_uuid: String,
    server_version: String,
    http_port: u16,
    bind_addr: String,
}

impl DiscoveryResponse {
    pub fn new(
        server_name: String,
        server_uuid: String,
        server_version: String,
        http_port: u16,
        bind_addr: String,
    ) -> Self {
        Self {
            server_name,
            server_uuid,
            server_version,
            http_port,
            bind_addr,
        }
    }

    /// Build legacy discovery response: 'D' + hostname (17 bytes total)
    pub fn build_legacy(&self) -> Vec<u8> {
        let mut response = Vec::with_capacity(18);
        response.push(b'D');

        // Truncate hostname to 16 bytes, pad with nulls
        let hostname_bytes = self.server_name.as_bytes();
        let hostname_len = hostname_bytes.len().min(16);
        response.extend_from_slice(&hostname_bytes[..hostname_len]);

        // Pad to 17 bytes (16 hostname + null terminator)
        while response.len() < 18 {
            response.push(0);
        }

        response
    }

    /// Build TLV discovery response: 'E' + TLV entries
    pub fn build_tlv(&self, request_tlvs: &[TlvEntry]) -> Vec<u8> {
        let mut response = Vec::new();
        response.push(b'E');

        // Process each requested TLV
        for tlv in request_tlvs {
            if let Some(value) = self.get_tlv_value(&tlv.tag) {
                if value.len() > 255 {
                    warn!("TLV {} value too long, truncating", tlv.tag_str());
                    let truncated = &value[..255];
                    response.extend_from_slice(&tlv.tag);
                    response.push(truncated.len() as u8);
                    response.extend_from_slice(truncated);
                } else {
                    response.extend_from_slice(&tlv.tag);
                    response.push(value.len() as u8);
                    response.extend_from_slice(&value);
                }
            }
        }

        // Limit total response size
        if response.len() > 1450 {
            warn!("Discovery response too long ({}), truncating", response.len());
            response.truncate(1450);
        }

        response
    }

    /// Get value for a specific TLV tag
    fn get_tlv_value(&self, tag: &[u8; 4]) -> Option<Vec<u8>> {
        match tag {
            b"NAME" => Some(self.server_name.as_bytes().to_vec()),
            b"IPAD" => Some(self.bind_addr.as_bytes().to_vec()),
            b"JSON" => Some(self.http_port.to_string().as_bytes().to_vec()),
            b"VERS" => Some(self.server_version.as_bytes().to_vec()),
            b"UUID" => Some(self.server_uuid.as_bytes().to_vec()),
            b"JVID" => {
                // Info only - client sending Jive ID, no response needed
                None
            }
            _ => {
                debug!("Unknown TLV tag: {:?}", String::from_utf8_lossy(tag));
                None
            }
        }
    }
}

/// Parse discovery request from UDP packet
pub fn parse_discovery_request(data: &[u8]) -> Result<DiscoveryRequest> {
    if data.is_empty() {
        anyhow::bail!("Empty discovery packet");
    }

    match data[0] {
        b'd' => parse_legacy_discovery(data),
        b'e' => parse_tlv_discovery(data),
        _ => anyhow::bail!("Unknown discovery packet type: {:02x}", data[0]),
    }
}

/// Parse legacy discovery format: 'd' + padding + device_id + revision + padding + MAC
fn parse_legacy_discovery(data: &[u8]) -> Result<DiscoveryRequest> {
    // Format: 'd' (1) + padding (1) + device_id (1) + revision (1) + padding (8) + MAC (6)
    // Total: 18 bytes minimum
    if data.len() < 18 {
        anyhow::bail!("Legacy discovery packet too short: {} bytes", data.len());
    }

    let device_id = data[2];
    let revision = data[3];
    let mac: [u8; 6] = data[12..18]
        .try_into()
        .context("Failed to extract MAC address")?;

    info!(
        "Legacy discovery: device_id={}, revision={}, MAC={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        device_id,
        revision,
        mac[0],
        mac[1],
        mac[2],
        mac[3],
        mac[4],
        mac[5]
    );

    Ok(DiscoveryRequest::Legacy {
        device_id,
        revision,
        mac,
    })
}

/// Parse TLV discovery format: 'e' + TLV entries
fn parse_tlv_discovery(data: &[u8]) -> Result<DiscoveryRequest> {
    let mut tlvs = Vec::new();
    let mut offset = 1; // Skip 'e'

    while offset + 5 <= data.len() {
        // TLV: 4-byte tag + 1-byte length + value
        let tag: [u8; 4] = data[offset..offset + 4]
            .try_into()
            .context("Failed to extract TLV tag")?;
        let length = data[offset + 4] as usize;

        if offset + 5 + length > data.len() {
            warn!("TLV packet truncated, stopping parse");
            break;
        }

        let value = data[offset + 5..offset + 5 + length].to_vec();

        debug!(
            "TLV: {} len={}",
            String::from_utf8_lossy(&tag),
            length
        );

        tlvs.push(TlvEntry { tag, value });

        offset += 5 + length;
    }

    info!("TLV discovery: {} entries", tlvs.len());

    Ok(DiscoveryRequest::Tlv { tlvs })
}

/// UDP discovery server
pub struct DiscoveryServer {
    socket: UdpSocket,
    response_builder: DiscoveryResponse,
}

impl DiscoveryServer {
    /// Create and bind discovery server
    pub async fn bind(
        bind_addr: &str,
        port: u16,
        server_name: String,
        server_uuid: String,
        server_version: String,
        http_port: u16,
    ) -> Result<Self> {
        let addr = format!("{}:{}", bind_addr, port);
        let socket = UdpSocket::bind(&addr)
            .await
            .context("Failed to bind UDP discovery socket")?;

        // Enable broadcast
        socket
            .set_broadcast(true)
            .context("Failed to enable broadcast")?;

        info!("UDP discovery server listening on {}", addr);

        Ok(Self {
            socket,
            response_builder: DiscoveryResponse::new(
                server_name,
                server_uuid,
                server_version,
                http_port,
                bind_addr.to_string(),
            ),
        })
    }

    /// Run discovery server loop
    pub async fn run(self) -> Result<()> {
        let mut buf = vec![0u8; 1500];

        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((len, peer_addr)) => {
                    debug!("Discovery packet from {}: {} bytes", peer_addr, len);

                    if let Err(e) = self.handle_discovery(&buf[..len], peer_addr).await {
                        warn!("Failed to handle discovery from {}: {}", peer_addr, e);
                    }
                }
                Err(e) => {
                    warn!("UDP recv error: {}", e);
                }
            }
        }
    }

    /// Handle a discovery request
    async fn handle_discovery(&self, data: &[u8], peer_addr: SocketAddr) -> Result<()> {
        let request = parse_discovery_request(data)?;

        let response = match request {
            DiscoveryRequest::Legacy {
                device_id,
                revision,
                mac,
            } => {
                info!(
                    "Sending legacy discovery response to {} (device={}, rev={})",
                    peer_addr, device_id, revision
                );
                self.response_builder.build_legacy()
            }
            DiscoveryRequest::Tlv { tlvs } => {
                info!("Sending TLV discovery response to {} ({} tags)", peer_addr, tlvs.len());
                self.response_builder.build_tlv(&tlvs)
            }
        };

        self.socket
            .send_to(&response, peer_addr)
            .await
            .context("Failed to send discovery response")?;

        debug!("Sent {} byte discovery response to {}", response.len(), peer_addr);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_legacy_discovery() {
        let mut data = vec![b'd', 0x00, 0x0c, 0x00]; // 'd' + padding + device_id=12 + revision=0
        data.extend_from_slice(&[0; 8]); // padding
        data.extend_from_slice(&[0xc4, 0x62, 0x37, 0x01, 0x98, 0x40]); // MAC

        let result = parse_discovery_request(&data).unwrap();

        match result {
            DiscoveryRequest::Legacy {
                device_id,
                revision,
                mac,
            } => {
                assert_eq!(device_id, 12);
                assert_eq!(revision, 0);
                assert_eq!(mac, [0xc4, 0x62, 0x37, 0x01, 0x98, 0x40]);
            }
            _ => panic!("Expected Legacy request"),
        }
    }

    #[test]
    fn test_parse_tlv_discovery() {
        let mut data = vec![b'e'];
        // Add NAME TLV
        data.extend_from_slice(b"NAME");
        data.push(0); // length 0 (just requesting)

        // Add UUID TLV
        data.extend_from_slice(b"UUID");
        data.push(0);

        let result = parse_discovery_request(&data).unwrap();

        match result {
            DiscoveryRequest::Tlv { tlvs } => {
                assert_eq!(tlvs.len(), 2);
                assert_eq!(&tlvs[0].tag, b"NAME");
                assert_eq!(&tlvs[1].tag, b"UUID");
            }
            _ => panic!("Expected TLV request"),
        }
    }

    #[test]
    fn test_build_legacy_response() {
        let builder = DiscoveryResponse::new(
            "TestServer".to_string(),
            "test-uuid".to_string(),
            "0.1.0".to_string(),
            9000,
            "0.0.0.0".to_string(),
        );

        let response = builder.build_legacy();

        assert_eq!(response[0], b'D');
        assert_eq!(response.len(), 18);
        assert!(response[1..11].starts_with(b"TestServer"));
    }

    #[test]
    fn test_build_tlv_response() {
        let builder = DiscoveryResponse::new(
            "TestServer".to_string(),
            "test-uuid-123".to_string(),
            "0.1.0".to_string(),
            9000,
            "192.168.1.100".to_string(),
        );

        let request_tlvs = vec![
            TlvEntry::new(b"NAME", vec![]),
            TlvEntry::new(b"UUID", vec![]),
            TlvEntry::new(b"JSON", vec![]),
        ];

        let response = builder.build_tlv(&request_tlvs);

        assert_eq!(response[0], b'E');
        assert!(response.len() > 1);

        // Should contain all three tags
        let response_str = String::from_utf8_lossy(&response);
        assert!(response_str.contains("NAME"));
        assert!(response_str.contains("UUID"));
        assert!(response_str.contains("JSON"));
    }
}
