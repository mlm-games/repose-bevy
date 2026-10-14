use bevy::prelude::*;
use bevy::window::{CursorIcon, CursorOptions, PrimaryWindow, SystemCursorIcon};
use repose_core::CursorIcon as ReposeCursor;

use crate::state::ReposeOutput;

/// What the window should show, so an unchanged value costs no component writes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CursorState {
    icon: SystemCursorIcon,
    visible: bool,
}

pub fn apply_cursor_system(
    output: Res<ReposeOutput>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
    mut last: Local<Option<CursorState>>,
) {
    if !output.is_changed() {
        return;
    }
    let Ok(entity) = windows.single() else {
        return;
    };
    let cursor = output.cursor.as_ref().unwrap_or(&ReposeCursor::Default);
    let state = match cursor {
        ReposeCursor::Default => CursorState {
            icon: SystemCursorIcon::Default,
            visible: true,
        },
        ReposeCursor::Pointer => CursorState {
            icon: SystemCursorIcon::Pointer,
            visible: true,
        },
        ReposeCursor::Text => CursorState {
            icon: SystemCursorIcon::Text,
            visible: true,
        },
        ReposeCursor::EwResize => CursorState {
            icon: SystemCursorIcon::EwResize,
            visible: true,
        },
        ReposeCursor::NsResize => CursorState {
            icon: SystemCursorIcon::NsResize,
            visible: true,
        },
        ReposeCursor::NwseResize => CursorState {
            icon: SystemCursorIcon::NwseResize,
            visible: true,
        },
        ReposeCursor::NeswResize => CursorState {
            icon: SystemCursorIcon::NeswResize,
            visible: true,
        },
        ReposeCursor::Grab => CursorState {
            icon: SystemCursorIcon::Grab,
            visible: true,
        },
        ReposeCursor::Grabbing => CursorState {
            icon: SystemCursorIcon::Grabbing,
            visible: true,
        },
        // Repose hides the OS arrow with `set_cursor_visible(false)` rather
        // than naming an icon, so Bevy's equivalent is the window's
        // `CursorOptions.visible` and the icon itself stops mattering.
        ReposeCursor::Hidden => CursorState {
            icon: SystemCursorIcon::Default,
            visible: false,
        },
        // Repose's custom cursor carries raw RGBA, and Bevy's needs a
        // `Handle<Image>`, so bridging it means uploading a texture. Until
        // that lands, fall back rather than pick a wrong-looking arrow.
        ReposeCursor::Custom(_) => {
            warn!("repose-bevy: custom cursor images are not bridged yet; using Default");
            CursorState {
                icon: SystemCursorIcon::Default,
                visible: true,
            }
        }
    };
    if *last == Some(state) {
        return;
    }
    *last = Some(state);
    commands.entity(entity).insert((
        CursorIcon::System(state.icon),
        CursorOptions {
            visible: state.visible,
            ..CursorOptions::default()
        },
    ));
}
