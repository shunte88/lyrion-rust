//! Slimproto server
//! Listens for player connections and handles messages

use crate::{SlimprotoCodec, SlimprotoMessage, StreamCommand, SLIMPROTO_PORT};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_util::codec::{Framed, Decoder};
use futures::StreamExt;

/// Player connection info
#[derive(Debug, Clone)]
pub struct PlayerConnection {
    pub mac: [u8; 6],
    pub device_id: u8,
    pub revision: u8,
    pub uuid: Option<String>,
    pub command_tx: mpsc::UnboundedSender<Vec<u8>>,
}

/// Slimproto server state
pub struct SlimprotoServer {
    players: Arc<RwLock<HashMap<String, PlayerConnection>>>,
    message_tx: mpsc::UnboundedSender<(String, SlimprotoMessage)>,
}

impl SlimprotoServer {
    /// Create new server
    pub fn new() -> (Self, mpsc::UnboundedReceiver<(String, SlimprotoMessage)>) {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let server = Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
        };

        (server, message_rx)
    }

    /// Start server on SLIMPROTO_PORT
    pub async fn listen(&self, bind_addr: &str) -> Result<()> {
        let addr = format!("{}:{}", bind_addr, SLIMPROTO_PORT);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("Slimproto server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((socket, peer_addr)) => {
                    tracing::info!("New connection from {}", peer_addr);

                    let players = Arc::clone(&self.players);
                    let message_tx = self.message_tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(socket, players, message_tx).await {
                            tracing::error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Get connected players
    pub async fn get_players(&self) -> Vec<(String, PlayerConnection)> {
        let players = self.players.read().await;
        players.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Send a stream command to a player
    pub async fn send_command(&self, mac: &str, command: StreamCommand) -> Result<()> {
        let players = self.players.read().await;

        if let Some(player) = players.get(mac) {
            let encoded = command.encode();
            player.command_tx.send(encoded)
                .map_err(|_| anyhow::anyhow!("Failed to send command to player {}", mac))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Player {} not found", mac))
        }
    }

    /// Send raw bytes to a player
    pub async fn send_raw(&self, mac: &str, data: Vec<u8>) -> Result<()> {
        let players = self.players.read().await;

        if let Some(player) = players.get(mac) {
            player.command_tx.send(data)
                .map_err(|_| anyhow::anyhow!("Failed to send data to player {}", mac))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Player {} not found", mac))
        }
    }

    /// Send audio gain (volume) command to a player
    pub async fn send_audio_gain(&self, mac: &str, command: crate::AudioGainCommand) -> Result<()> {
        let encoded = command.encode();
        self.send_raw(mac, encoded).await
    }
}

/// Handle individual player connection
async fn handle_connection(
    mut socket: TcpStream,
    players: Arc<RwLock<HashMap<String, PlayerConnection>>>,
    message_tx: mpsc::UnboundedSender<(String, SlimprotoMessage)>,
) -> Result<()> {
    use tokio::io::{AsyncWriteExt, AsyncReadExt};

    let peer_addr = socket.peer_addr()?;

    // Set TCP_NODELAY to disable Nagle's algorithm
    socket.set_nodelay(true)?;

    let mut player_mac: Option<String> = None;

    // Create channel for sending commands to this player
    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    tracing::debug!("Starting message loop for {}", peer_addr);

    // Manual codec-like reading
    let mut buffer = bytes::BytesMut::with_capacity(4096);
    let mut codec = SlimprotoCodec;
    let mut read_buf = vec![0u8; 4096];

    loop {
        // Use tokio::select! to multiplex reading and writing on the same task
        tokio::select! {
            // Handle incoming commands to send
            command = command_rx.recv() => {
                if let Some(data) = command {
                    tracing::info!("Sending {} bytes to {}", data.len(), peer_addr);

                    // Hex dump for debugging
                    let hex_dump = data.iter()
                        .enumerate()
                        .fold(String::new(), |mut acc, (i, b)| {
                            if i % 16 == 0 {
                                acc.push_str(&format!("\n  {:04x}: ", i));
                            }
                            acc.push_str(&format!("{:02x} ", b));
                            acc
                        });
                    tracing::info!("Hex dump:{}", hex_dump);

                    // Also decode opcode and length
                    if data.len() >= 8 {
                        let opcode = std::str::from_utf8(&data[0..4]).unwrap_or("????");
                        let length = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                        tracing::info!("Opcode: '{}', Length: {}, Total: {}", opcode, length, data.len());
                    }

                    tracing::info!("Buffer state before write: {} bytes", buffer.len());
                    // Small delay to ensure squeezelite is ready
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    tracing::info!("Writing to socket...");
                    if let Err(e) = socket.write_all(&data).await {
                        tracing::error!("Failed to send command to {}: {}", peer_addr, e);
                        break;
                    }
                    tracing::info!("Flushing socket...");
                    if let Err(e) = socket.flush().await {
                        tracing::error!("Failed to flush to {}: {}", peer_addr, e);
                        break;
                    }
                    tracing::info!("Command sent and flushed successfully to {}", peer_addr);
                } else {
                    // Channel closed
                    break;
                }
            }
            // Handle incoming data from socket
            read_result = socket.read(&mut read_buf) => {
                match read_result {
                    Ok(0) => {
                        tracing::info!("Socket closed by peer: {}", peer_addr);
                        break; // Connection closed
                    }
                    Ok(n) => {
                        buffer.extend_from_slice(&read_buf[..n]);
                    }
                    Err(e) => {
                        tracing::error!("Error reading from {}: {}", peer_addr, e);
                        break;
                    }
                }
            }
        }

        // Try to decode messages after each iteration
        loop {
            if buffer.len() < 8 {
                break; // Need more data
            }

        // Try to decode a message
        match codec.decode(&mut buffer) {
            Ok(Some(message)) => {
                tracing::debug!("Received message from {}: {:?}", peer_addr, message);

                match &message {
                    SlimprotoMessage::Helo(helo) => {
                        let mac_str = helo.mac_string();
                        player_mac = Some(mac_str.clone());

                        tracing::info!(
                            "Player HELO: MAC={}, Device={}, Revision={}",
                            mac_str,
                            helo.device_id,
                            helo.revision
                        );
                        tracing::info!("Buffer after HELO decode: {} bytes remaining", buffer.len());

                        // Register player
                        let mut players_map = players.write().await;
                        players_map.insert(
                            mac_str.clone(),
                            PlayerConnection {
                                mac: helo.mac,
                                device_id: helo.device_id,
                                revision: helo.revision,
                                uuid: helo.uuid.clone(),
                                command_tx: command_tx.clone(),
                            },
                        );

                        // Forward message
                        let _ = message_tx.send((mac_str, message));
                    }
                    SlimprotoMessage::Stat(stat) => {
                        if let Some(ref mac) = player_mac {
                            tracing::debug!(
                                "STAT from {}: buffer={}/{}",
                                mac,
                                stat.output_buffer_fullness,
                                stat.output_buffer_size
                            );

                            // Forward message
                            let _ = message_tx.send((mac.clone(), message));
                        }
                    }
                    SlimprotoMessage::Bye => {
                        tracing::info!("Player {} disconnecting (BYE)", peer_addr);

                        if let Some(ref mac) = player_mac {
                            let mut players_map = players.write().await;
                            players_map.remove(mac);
                        }

                        break;
                    }
                    SlimprotoMessage::Dsco(reason) => {
                        tracing::info!("Player {} disconnected: {:?}", peer_addr, reason);

                        if let Some(ref mac) = player_mac {
                            let mut players_map = players.write().await;
                            players_map.remove(mac);
                        }

                        break;
                    }
                    _ => {
                        if let Some(ref mac) = player_mac {
                            let _ = message_tx.send((mac.clone(), message));
                        }
                    }
                }
            }
            Ok(None) => {
                // Need more data, continue reading to outer loop
                break;
            }
            Err(e) => {
                tracing::error!("Decode error from {}: {}", peer_addr, e);
                break;
            }
        }
        } // Close inner decode loop
    } // Close outer main loop

    // Clean up player on disconnect
    if let Some(mac) = player_mac {
        let mut players_map = players.write().await;
        players_map.remove(&mac);
        tracing::info!("Player {} disconnected", mac);
    }

    Ok(())
}
