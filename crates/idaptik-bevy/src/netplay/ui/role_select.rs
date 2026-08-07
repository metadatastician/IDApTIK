//! Role selection for multiplayer mode.
//!
//! This module provides a GUI-based role selection mechanism for multiplayer
//! using Bevy's built-in UI system.

use bevy::prelude::*;
use idaptik_net::envelope::Role;

/// Plugin for role selection
pub struct RoleSelectionPlugin;

/// State for role selection
#[derive(Resource, Default)]
pub struct RoleSelectionState {
    /// Whether the role selection UI is currently active
    pub active: bool,
    /// The selected role (None if not yet selected)
    pub selected_role: Option<Role>,
}

/// App state for role selection
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NetplayAppState {
    #[default]
    Normal,
    RoleSelection,
}

/// Marker component for the role selection UI root
#[derive(Component)]
pub struct RoleSelectionUi;

/// Marker component for role selection buttons
#[derive(Component)]
pub struct RoleButton {
    pub role: Role,
}

/// Marker component for the role selection title
#[derive(Component)]
pub struct RoleSelectionTitle;

/// Text formatting for role selection UI
fn title_text() -> (TextFont, TextColor) {
    (
        TextFont::from_font_size(24.0),
        TextColor(Color::WHITE),
    )
}

fn subtitle_text() -> (TextFont, TextColor) {
    (
        TextFont::from_font_size(16.0),
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
    )
}

fn role_text() -> (TextFont, TextColor) {
    (
        TextFont::from_font_size(18.0),
        TextColor(Color::WHITE),
    )
}

fn description_text() -> (TextFont, TextColor) {
    (
        TextFont::from_font_size(12.0),
        TextColor(Color::srgb(0.7, 0.9, 0.7)),
    )
}

/// Button node style
fn button_node() -> Node {
    Node {
        width: Val::Px(200.0),
        height: Val::Px(50.0),
        margin: UiRect::all(Val::Px(10.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        ..Default::default()
    }
}

/// Normal button color
fn button_normal_color() -> BackgroundColor {
    BackgroundColor(Color::srgb(0.2, 0.2, 0.2))
}

/// Hover button color
fn button_hover_color() -> BackgroundColor {
    BackgroundColor(Color::srgb(0.3, 0.3, 0.3))
}

/// Selected/active button color
fn button_active_color() -> BackgroundColor {
    BackgroundColor(Color::srgb(0.4, 0.6, 0.8))
}

impl Plugin for RoleSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoleSelectionState>()
            .init_state::<NetplayAppState>()
            .add_systems(Startup, setup_role_selection_ui)
            .add_systems(Update, (
                check_cli_role_selection,
                handle_role_button_interaction,
            ))
            .add_systems(OnEnter(NetplayAppState::RoleSelection), show_role_selection_ui)
            .add_systems(OnExit(NetplayAppState::RoleSelection), hide_role_selection_ui);
    }
}

/// System to check if role was selected via CLI
fn check_cli_role_selection(state: Res<RoleSelectionState>) {
    // Role selection is handled via CLI arguments in main.rs
    // This system is kept for compatibility but GUI selection now works
    let _ = state;
}

/// Setup the role selection UI (hidden by default)
pub fn setup_role_selection_ui(mut commands: Commands) {
    commands
        .spawn((
            RoleSelectionUi,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(30.0),
                left: Val::Percent(40.0),
                width: Val::Percent(20.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(20.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.18)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                RoleSelectionTitle,
                Text::new("Select Your Role"),
                title_text(),
            ));
            
            // Description
            parent.spawn((
                Text::new("Choose how you want to play"),
                subtitle_text(),
            ));
            
            // Infiltrator button
            parent.spawn((
                RoleButton { role: Role::Infiltrator },
                Button,
                button_node(),
                button_normal_color(),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Infiltrator"),
                    role_text(),
                ));
                button.spawn((
                    Text::new("Stealth & Agility"),
                    description_text(),
                ));
            });
            
            // Hacker button  
            parent.spawn((
                RoleButton { role: Role::Hacker },
                Button,
                button_node(),
                button_normal_color(),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Hacker"),
                    role_text(),
                ));
                button.spawn((
                    Text::new("Systems & Control"),
                    TextColor(Color::srgb(0.7, 0.7, 0.9)),
                    TextFont::from_font_size(12.0),
                ));
            });
        });
}

/// Show the role selection UI
fn show_role_selection_ui(
    mut ui_query: Query<&mut Visibility, With<RoleSelectionUi>>,
    mut state: ResMut<RoleSelectionState>,
) {
    for mut visibility in &mut ui_query {
        *visibility = Visibility::Visible;
    }
    state.active = true;
}

/// Hide the role selection UI
fn hide_role_selection_ui(
    mut ui_query: Query<&mut Visibility, With<RoleSelectionUi>>,
    mut state: ResMut<RoleSelectionState>,
) {
    for mut visibility in &mut ui_query {
        *visibility = Visibility::Hidden;
    }
    state.active = false;
}

/// Handle button interactions for role selection
fn handle_role_button_interaction(
    mut interaction_query: Query<(
        &Interaction,
        &RoleButton,
        &mut BackgroundColor,
    )>,
    mut state: ResMut<RoleSelectionState>,
    mut app_state: ResMut<NextState<NetplayAppState>>,
) {
    for (interaction, role_button, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Select this role
                state.selected_role = Some(role_button.role);
                state.active = false;
                
                // Hide the UI and return to normal state
                app_state.set(NetplayAppState::Normal);
                
                // Set the button to active color to show selection
                *bg_color = button_active_color();
            }
            Interaction::Hovered => {
                // Highlight on hover
                *bg_color = button_hover_color();
            }
            Interaction::None => {
                // Return to normal color if not selected
                if state.selected_role != Some(role_button.role) {
                    *bg_color = button_normal_color();
                }
            }
        }
    }
}
