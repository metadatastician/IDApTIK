//! UI components for multiplayer netplay mode.

pub mod role_select;

pub use role_select::{
    NetplayAppState, RoleSelectionPlugin, RoleSelectionState, RoleSelectionUi, RoleButton,
    RoleSelectionTitle, setup_role_selection_ui,
};
