//! UI components for multiplayer netplay mode.

pub mod role_select;

pub use role_select::{
    NetplayAppState, RoleButton, RoleSelectionPlugin, RoleSelectionState, RoleSelectionTitle,
    RoleSelectionUi, setup_role_selection_ui,
};
