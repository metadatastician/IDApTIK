//! Bevy plugin for multiplayer netplay mode.
//!
//! This plugin integrates the idaptik-net delay-lockstep protocol into Bevy,
//! allowing the GUI frontend to participate in multiplayer sessions.

use bevy::prelude::*;
use idaptik_net::envelope::Role;
use idaptik_net::lockstep::LockstepCore;
use idaptik_tui::script::ScriptFile;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use tokio::runtime::Runtime;

use crate::driver::VisualBuffers;
use crate::netplay::connection::{
    ConnectionConfig, IncomingMessage, NetplayStatus, NetworkMessage, spawn_connection_task,
};
use crate::netplay::input::BevyInputFeed;

/// Mode for netplay: hosting a new session or joining an existing one.
#[derive(Debug, Clone)]
pub enum NetplayMode {
    /// Host a new multiplayer session
    Host,
    /// Join an existing multiplayer session
    Join {
        /// Address of the host to join
        host: String,
    },
}

/// Configuration for the netplay plugin.
#[derive(Debug, Clone, Resource)]
pub struct NetplayConfig {
    /// Host or join mode
    pub mode: NetplayMode,
    /// Player's role in the session
    pub role: String,
    /// Session ID to join/host
    pub session: String,
    /// Relay WebSocket URL
    pub relay_url: String,
    /// Path to the script file
    pub script_path: PathBuf,
    /// Input delay in ticks for delay-lockstep
    pub input_delay: u64,
}

/// Resource holding the netplay state.
#[derive(Resource)]
pub struct NetplayState {
    /// The lockstep core managing the simulation
    pub core: Option<LockstepCore>,
    /// Current connection status
    pub status: NetplayStatus,
    /// The resolved role (after selection if needed)
    pub role: Option<Role>,
    /// The script file loaded from disk
    pub script: Option<ScriptFile>,
    /// Visual buffers for rendering
    pub visual: Option<VisualBuffers>,
    /// Sender for network messages to connection task (Sync-safe when cloned)
    pub tx_network: Option<mpsc::Sender<NetworkMessage>>,
    /// Receiver for network messages from connection task, wrapped for thread safety
    pub rx_network: Option<Arc<Mutex<mpsc::Receiver<NetworkMessage>>>>,
}

impl Default for NetplayState {
    fn default() -> Self {
        Self {
            core: None,
            status: NetplayStatus::SelectingRole,
            role: None,
            script: None,
            visual: None,
            tx_network: None,
            rx_network: None,
        }
    }
}

impl NetplayState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Plugin for multiplayer netplay mode.
pub struct NetplayPlugin {
    config: NetplayConfig,
}

impl NetplayPlugin {
    pub fn new(config: NetplayConfig) -> Self {
        Self { config }
    }
}

impl Plugin for NetplayPlugin {
    fn build(&self, app: &mut App) {
        // Parse role from config
        let role: Role = self.config.role.parse().unwrap_or(Role::Infiltrator);

        // Initialize netplay state
        let mut state = NetplayState::new();
        state.status = NetplayStatus::Connecting;
        state.role = Some(role);

        // We'll create channels and spawn connection in setup_netplay_system
        // after we have the script loaded

        // Insert resources
        app.insert_resource(state)
            .insert_resource(self.config.clone())
            .insert_resource(BevyInputFeed::new())
            .add_systems(Startup, (setup_netplay_system, setup_connection_status_ui))
            .add_systems(
                FixedUpdate,
                (
                    crate::netplay::input::advance_input_feed_tick,
                    pump_outgoing_system,
                    process_incoming_system,
                    advance_lockstep_system,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    crate::netplay::input::capture_lockstep_input,
                    render_status_system,
                ),
            );
    }
}

/// System to initialize netplay after script loading
fn setup_netplay_system(mut state: ResMut<NetplayState>, config: Res<NetplayConfig>) {
    // Only initialize once
    if state.core.is_some() {
        return;
    }

    // Load the script file
    match idaptik_tui::headless::load(&config.script_path) {
        Ok(script) => {
            // Parse role from config
            let role: Role = config.role.parse().unwrap_or(Role::Infiltrator);

            // Extract script configuration
            let script_seed = script.seed;
            let script_difficulty = script.difficulty.clone();
            let script_reduced_motion = script.reduced_motion;
            let script_max_ticks = script.max_ticks;

            // Use the script's seed
            let actual_seed = script_seed;

            // Create the LockstepCore
            match LockstepCore::new(role, config.input_delay, &script) {
                Ok(core) => {
                    state.core = Some(core);
                    state.script = Some(script);

                    // Initialize visual buffers from the lockstep core's simulation
                    if let Some(ref core) = state.core {
                        state.visual = Some(VisualBuffers::primed(core.sim()));
                    }

                    // Create two channels:
                    // 1. Bevy -> Connection: Bevy sends outgoing lockstep messages to connection
                    // 2. Connection -> Bevy: Connection sends incoming network messages to Bevy
                    let (tx_from_bevy, rx_from_bevy) = mpsc::channel(); // Bevy → Connection
                    let (tx_to_bevy, rx_to_bevy) = mpsc::channel(); // Connection → Bevy

                    // Store in state
                    state.tx_network = Some(tx_from_bevy); // Bevy uses this to send to connection
                    state.rx_network = Some(Arc::new(Mutex::new(rx_to_bevy))); // Bevy uses this to receive from connection

                    // Spawn connection task in separate thread
                    let role_for_task = state.role.unwrap_or(Role::Infiltrator);
                    let connection_config = ConnectionConfig {
                        relay_url: config.relay_url.clone(),
                        session_id: config.session.clone(),
                        role: role_for_task,
                        seed: actual_seed,
                        difficulty: script_difficulty,
                        reduced_motion: script_reduced_motion,
                        max_ticks: script_max_ticks,
                    };
                    let runtime = Runtime::new().expect("Failed to create tokio runtime");
                    let handle = spawn_connection_task(
                        connection_config,
                        rx_from_bevy, // rx_ui: connection receives from Bevy
                        tx_to_bevy,   // tx_back: connection sends to Bevy
                    );

                    // Note: We don't store runtime and handle since they're not Sync
                    std::mem::forget(runtime);
                    std::mem::forget(handle);

                    state.status = NetplayStatus::WaitingForPeer;
                }
                Err(e) => {
                    state.status =
                        NetplayStatus::Error(format!("Failed to create lockstep core: {}", e));
                }
            }
        }
        Err(e) => {
            state.status = NetplayStatus::Error(format!("Failed to load script: {}", e));
        }
    }
}

/// System to pump outgoing messages from lockstep to network
fn pump_outgoing_system(mut state: ResMut<NetplayState>, mut input_feed: ResMut<BevyInputFeed>) {
    // If we have a core, pump outgoing messages
    if let Some(core) = &mut state.core {
        let outgoing = core.pump_outgoing(&mut *input_feed);
        if let Some(tx) = &state.tx_network {
            for msg in outgoing {
                // Convert Lockstep Outgoing to NetworkMessage
                // This needs proper type conversion
                let _ = tx.send(NetworkMessage::Send(msg));
            }
        }
    }
}

/// System to process incoming network messages
fn process_incoming_system(mut state: ResMut<NetplayState>) {
    if let Some(rx_arc) = &state.rx_network {
        let mut status_update: Option<NetplayStatus> = None;
        let mut incoming_messages: Vec<IncomingMessage> = Vec::new();

        // Collect messages while holding the lock
        {
            let rx = rx_arc.lock().unwrap();
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    NetworkMessage::Received(incoming) => {
                        incoming_messages.push(incoming);
                    }
                    NetworkMessage::Status(new_status) => {
                        status_update = Some(new_status);
                    }
                    NetworkMessage::Send(_) => {
                        // Shouldn't receive Send messages here
                    }
                }
            }
        }

        // Process collected messages outside the lock
        for incoming in incoming_messages {
            match incoming {
                IncomingMessage::Command { at, cmd } => {
                    // Queue incoming command for the lockstep core
                    if let Some(core) = &mut state.core {
                        let _ = core.on_peer_command(at, cmd);
                    }
                }
                IncomingMessage::Commit { through } => {
                    // Update lockstep with peer's watermark
                    if let Some(core) = &mut state.core {
                        let _ = core.on_peer_commit(through);
                    }
                }
                IncomingMessage::Resync(resync_payload) => {
                    // Handle resync request
                    state.status = NetplayStatus::Resyncing;

                    // Try to parse the resync payload and rebuild the lockstep core
                    if let Ok(resync_data) =
                        serde_json::from_value::<idaptik_net::lockstep::Resync>(resync_payload)
                    {
                        if let Some(role) = state.role {
                            let input_delay = 3; // Use default input delay for resync
                            match idaptik_net::lockstep::LockstepCore::adopt_resync(
                                role,
                                input_delay,
                                resync_data,
                            ) {
                                Ok(new_core) => {
                                    // Replace the existing core with the resynced one
                                    state.core = Some(new_core);

                                    // Reinitialize visual buffers from the new core's simulation
                                    state.visual = Some(VisualBuffers::primed(
                                        state.core.as_ref().unwrap().sim(),
                                    ));

                                    // Transition back to running state
                                    state.status = NetplayStatus::Running;
                                }
                                Err(err) => {
                                    state.status =
                                        NetplayStatus::Error(format!("Resync failed: {}", err));
                                }
                            }
                        } else {
                            state.status =
                                NetplayStatus::Error("Resync failed: role not set".to_string());
                        }
                    } else {
                        state.status =
                            NetplayStatus::Error("Resync failed: invalid payload".to_string());
                    }
                }
                IncomingMessage::PeerJoined => {
                    state.status = NetplayStatus::WaitingForPeer;
                }
                IncomingMessage::PeerLeft => {
                    state.status = NetplayStatus::PeerLost;
                }
                IncomingMessage::Connected => {
                    state.status = NetplayStatus::Running;
                }
                IncomingMessage::Disconnected => {
                    state.status = NetplayStatus::Disconnected;
                    // Notify lockstep core that peer was lost
                    if let Some(core) = &mut state.core {
                        core.on_peer_lost();
                    }
                }
            }
        }

        // Update status if needed
        if let Some(new_status) = status_update {
            state.status = new_status;
        }
    }
}

/// System to advance the lockstep core
fn advance_lockstep_system(mut state: ResMut<NetplayState>) {
    // Take ownership of core and visual to avoid borrow checker issues
    let core = std::mem::take(&mut state.core);
    let visual = std::mem::take(&mut state.visual);

    if let (Some(mut core), Some(mut visual)) = (core, visual) {
        core.advance_with(|sim, _events| {
            // Update visual state from sim
            visual.commit(sim, false);
        });
        state.core = Some(core);
        state.visual = Some(visual);
    }
}

/// Marker component for connection status UI
#[derive(Component)]
pub struct ConnectionStatusUi;

/// Text formatting for connection status
fn status_text() -> (TextFont, TextColor) {
    (TextFont::from_font_size(14.0), TextColor(Color::WHITE))
}

/// Setup connection status UI
fn setup_connection_status_ui(mut commands: Commands) {
    commands
        .spawn((
            ConnectionStatusUi,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                right: Val::Px(10.0),
                width: Val::Auto,
                height: Val::Auto,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.12)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Status label
            parent.spawn((Text::new("Status: "), status_text()));
            // Status text (will be updated dynamically)
            parent.spawn((
                ConnectionStatusText,
                Text::new("Disconnected"),
                status_text(),
            ));
        });
}

/// Marker component for the status text
#[derive(Component)]
pub struct ConnectionStatusText;

/// System to update and show/hide connection status UI based on netplay state
fn render_status_system(
    state: Res<NetplayState>,
    mut status_ui_query: Query<&mut Visibility, With<ConnectionStatusUi>>,
    mut status_text_query: Query<&mut Text, With<ConnectionStatusText>>,
) {
    // Only show status UI in multiplayer mode
    if state.core.is_none() {
        // Hide status UI if not in netplay mode
        for mut visibility in &mut status_ui_query {
            *visibility = Visibility::Hidden;
        }
        return;
    }

    // Show status UI in netplay mode
    for mut visibility in &mut status_ui_query {
        *visibility = Visibility::Visible;
    }

    // Update status text based on current state
    let status_text = match state.status {
        NetplayStatus::Connecting => "Connecting...",
        NetplayStatus::WaitingForPeer => "Waiting for peer...",
        NetplayStatus::Running => "Connected - Running",
        NetplayStatus::PeerLost => "Peer Lost!",
        NetplayStatus::Resyncing => "Resyncing...",
        NetplayStatus::Error(ref err) => &format!("Error: {}", err),
        NetplayStatus::Disconnected => "Disconnected",
        NetplayStatus::SelectingRole => "Select Role",
    };

    for mut text in &mut status_text_query {
        // In Bevy 0.19, Text is a newtype, so we replace the whole string
        text.0 = status_text.to_string();
    }
}
