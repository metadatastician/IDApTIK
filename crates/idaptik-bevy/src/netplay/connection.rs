//! Network connection management for multiplayer netplay.
//!
//! This module handles the Phoenix Channels connection in a separate tokio
//! runtime thread, with automatic retry on failure.

use idaptik_net::envelope::{HELLO_TAG, NET_PROTO, Role};
use idaptik_net::lockstep::Outgoing;
use idaptik_net::phoenix::PhoenixClient;
use idaptik_net::ws::PlainWebSocketTransport;
use serde_json::Value;
use std::sync::mpsc;
use std::thread;
use tokio::runtime::Runtime;

/// Configuration for a network connection
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub relay_url: String,
    pub session_id: String,
    pub role: Role,
    pub seed: u32,
    pub difficulty: String,
    pub reduced_motion: bool,
    pub max_ticks: u64,
}

/// Messages sent between Bevy (main thread) and connection task (tokio thread)
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    /// Send an outgoing lockstep message to the relay
    Send(Outgoing),
    /// Received an incoming message from the relay
    Received(IncomingMessage),
    /// Status update from the connection task
    Status(NetplayStatus),
}

/// Wrapper for incoming messages from the relay
#[derive(Debug, Clone)]
pub enum IncomingMessage {
    /// A command from the peer
    Command {
        at: u64,
        cmd: idaptik_core::scenario::command::Command,
    },
    /// A watermark commit from the peer
    Commit { through: u64 },
    /// Resync payload
    Resync(Value),
    /// Peer joined the session
    PeerJoined,
    /// Peer left the session
    PeerLeft,
    /// Connection established
    Connected,
    /// Connection lost
    Disconnected,
}

/// Current status of the netplay connection
#[derive(Debug, Clone, PartialEq)]
pub enum NetplayStatus {
    Connecting,
    WaitingForPeer,
    Running,
    PeerLost,
    Resyncing,
    Error(String),
    Disconnected,
    SelectingRole,
}

/// Handle to the connection task for graceful shutdown
pub struct ConnectionHandle {
    join_handle: thread::JoinHandle<()>,
}

impl ConnectionHandle {
    pub fn shutdown(self) {
        // Signal the thread to stop
        // For now, just drop which will stop the thread when it finishes
        let _ = self.join_handle.join();
    }
}

/// Spawn a connection task on a separate tokio runtime thread
pub fn spawn_connection_task(
    config: ConnectionConfig,
    rx_ui: mpsc::Receiver<NetworkMessage>,
    tx_back: mpsc::Sender<NetworkMessage>,
) -> ConnectionHandle {
    let join_handle = thread::spawn(move || {
        // Create a tokio runtime for this thread
        let runtime = Runtime::new().expect("Failed to create tokio runtime");

        // Run the async connection task
        runtime.block_on(async move {
            run_connection_loop(config, &rx_ui, &tx_back).await;
        });
    });

    ConnectionHandle { join_handle }
}

/// Main async loop for the connection task
async fn run_connection_loop(
    config: ConnectionConfig,
    rx_ui: &mpsc::Receiver<NetworkMessage>,
    tx_back: &mpsc::Sender<NetworkMessage>,
) {
    let mut retry_count = 0;
    const MAX_RETRIES: usize = 10;
    const RETRY_DELAY_MS: u64 = 1000;

    loop {
        match run_single_connection(&config, rx_ui, tx_back).await {
            Ok(_) => break, // Clean exit
            Err(e) => {
                retry_count += 1;
                if retry_count >= MAX_RETRIES {
                    let _ = tx_back.send(NetworkMessage::Status(NetplayStatus::Error(format!(
                        "Failed after {} retries: {}",
                        MAX_RETRIES, e
                    ))));
                    break;
                }
                let _ = tx_back.send(NetworkMessage::Status(NetplayStatus::Error(format!(
                    "Connection error, retry {}/{}...",
                    retry_count, MAX_RETRIES
                ))));
                tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }
}

/// Run a single connection attempt
async fn run_single_connection(
    config: &ConnectionConfig,
    rx_ui: &mpsc::Receiver<NetworkMessage>,
    tx_back: &mpsc::Sender<NetworkMessage>,
) -> Result<(), String> {
    // Connect to relay
    let transport = PlainWebSocketTransport::connect(&config.relay_url)
        .await
        .map_err(|e| format!("WebSocket connect: {}", e))?;

    let mut client = PhoenixClient::new(transport);

    // Create session topic (burble game-session fabric)
    let topic = format!("game:{}", config.session_id);

    // Build join params for burble (game id and role are required)
    let join_params = serde_json::json!({
        "game": "idaptik",
        "role": config.role.as_str(),
    });

    // Build hello payload from script configuration
    // We construct it manually since we don't have the ScriptFile in this thread
    let hello_payload = serde_json::json!({
        "event": HELLO_TAG,
        "proto": NET_PROTO,
        "seed": config.seed,
        "difficulty": &config.difficulty,
        "reduced_motion": config.reduced_motion,
        "max_ticks": config.max_ticks,
        "commands": 0, // 0 for live seats
        "rejoin": false, // false for fresh connections
    });

    // Join session with burble-compatible params
    let _join_response = client
        .join(&topic, join_params)
        .await
        .map_err(|e| format!("Join session: {}", e))?;

    // Send hello as the first control message after joining
    client
        .push("event", hello_payload)
        .await
        .map_err(|e| format!("Send hello: {}", e))?;

    // Notify Bevy that we're connected
    let _ = tx_back.send(NetworkMessage::Status(NetplayStatus::WaitingForPeer));

    // Main message loop
    loop {
        // Check for messages from Bevy (main thread)
        if let Ok(msg) = rx_ui.try_recv() {
            match msg {
                NetworkMessage::Send(outgoing) => {
                    send_outgoing(&mut client, outgoing, config).await?;
                }
                NetworkMessage::Status(_) | NetworkMessage::Received(_) => {
                    // Ignore other message types from Bevy
                }
            }
        }

        // Check for messages from relay
        match client
            .next_broadcast(std::time::Duration::from_millis(50))
            .await
        {
            Ok(Some(broadcast)) => {
                // Process broadcast from relay
                process_broadcast(&broadcast, tx_back)?
            }
            Ok(None) => {
                // Timeout, no message - continue
                continue;
            }
            Err(_e) => {
                // Connection error
                return Err("Connection error".to_string());
            }
        }
    }
}

/// Send an outgoing lockstep message to the relay
async fn send_outgoing(
    client: &mut PhoenixClient<PlainWebSocketTransport>,
    outgoing: Outgoing,
    config: &ConnectionConfig,
) -> Result<(), String> {
    match outgoing {
        Outgoing::Command { at, cmd } => {
            // Encode command for relay
            let payload = idaptik_net::envelope::encode_command(&cmd, 0, at)
                .map_err(|e| format!("Encode command: {}", e))?;
            client
                .push("command", payload)
                .await
                .map_err(|e| format!("Push command: {}", e))?;
        }
        Outgoing::Commit { through } => {
            // Send commit watermark
            let commit = idaptik_net::envelope::Commit {
                role: config.role,
                through,
            };
            let payload = commit.to_control();
            client
                .push("event", payload)
                .await
                .map_err(|e| format!("Push commit: {}", e))?;
        }
    }
    Ok(())
}

/// Process a broadcast message from the relay
fn process_broadcast(
    broadcast: &idaptik_net::phoenix::Broadcast,
    tx_back: &mpsc::Sender<NetworkMessage>,
) -> Result<(), String> {
    // Check if this is a control message (net: prefix)
    if broadcast.event.starts_with("net:") {
        // Handle control messages
        match broadcast.event.as_str() {
            "net:commit" => {
                // Parse commit watermark
                if let Some(through) = broadcast.payload.get("through").and_then(|v| v.as_u64()) {
                    let _ = tx_back.send(NetworkMessage::Received(IncomingMessage::Commit {
                        through,
                    }));
                }
            }
            "net:resync" => {
                let _ = tx_back.send(NetworkMessage::Received(IncomingMessage::Resync(
                    broadcast.payload.clone(),
                )));
            }
            "net:hello" => {
                let _ = tx_back.send(NetworkMessage::Received(IncomingMessage::PeerJoined));
            }
            _ => {
                // Unknown net: message
            }
        }
    } else {
        // Regular command message
        match idaptik_net::envelope::decode_command(&broadcast.payload) {
            Ok((at, cmd)) => {
                let _ = tx_back.send(NetworkMessage::Received(IncomingMessage::Command {
                    at,
                    cmd,
                }));
            }
            Err(_) => {
                // Not a command, or decode error
                // Could be an event from the sim
            }
        }
    }
    Ok(())
}
