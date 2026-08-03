use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use repose_core::CursorIcon as ReposeCursor;

use crate::state::ReposeOutput;

pub fn apply_cursor_system(
    output: Res<ReposeOutput>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
    mut last: Local<Option<SystemCursorIcon>>,
) {
    if !output.is_changed() {
        return;
    }
    let Ok(entity) = windows.single() else {
        return;
    };
    let icon = match output.cursor.unwrap_or(ReposeCursor::Default) {
        ReposeCursor::Default => SystemCursorIcon::Default,
        ReposeCursor::Pointer => SystemCursorIcon::Pointer,
        ReposeCursor::Text => SystemCursorIcon::Text,
        ReposeCursor::EwResize => SystemCursorIcon::EwResize,
        ReposeCursor::NsResize => SystemCursorIcon::NsResize,
        ReposeCursor::Grab => SystemCursorIcon::Grab,
        ReposeCursor::Grabbing => SystemCursorIcon::Grabbing,
    };
    if *last == Some(icon) {
        return;
    }
    *last = Some(icon);
    commands.entity(entity).insert(CursorIcon::System(icon));
}
