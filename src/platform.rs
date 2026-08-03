use parking_lot::Mutex;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::window::{Ime, PrimaryWindow};

use crate::state::{ReposeOutput, ReposeState};
use repose_core::input::ImeEvent;

#[derive(Resource, Clone)]
pub struct ClipboardBridge(pub Arc<Mutex<ClipboardState>>);

impl Default for ClipboardBridge {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ClipboardState::default())))
    }
}

pub struct ClipboardState {
    /// Text Repose wants written to the OS clipboard (via `set_clipboard_fn`).
    pub pending_write: Option<String>,
    /// Text fetched from Bevy clipboard, ready for `paste_text()` reads.
    pub cached_read: Option<String>,
}

impl Default for ClipboardState {
    fn default() -> Self {
        Self {
            pending_write: None,
            cached_read: None,
        }
    }
}

/// Install Repose clipboard hooks that bridge through the shared state.
pub fn install_clipboard_hooks(bridge: ClipboardBridge) {
    let wb = bridge.clone();
    repose_core::clipboard::set_clipboard_fn(Box::new(move |text| {
        wb.0.lock().pending_write = Some(text.to_string());
    }));

    let rb = bridge;
    repose_core::clipboard::set_clipboard_read_fn(Box::new(move || rb.0.lock().cached_read.take()));
}

/// Sync clipboard writes from Repose output to Bevy's Clipboard resource,
/// and refresh the read cache from Bevy's Clipboard.
pub fn clipboard_system(
    bridge: Res<ClipboardBridge>,
    mut clipboard: ResMut<bevy::clipboard::Clipboard>,
    output: Res<ReposeOutput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
) {
    let mut b = bridge.0.lock();

    // Write: flushed from output.clipboard_text (captured during compose) or from observer
    if let Some(text) = &output.clipboard_text {
        let _ = clipboard.set_text(text.as_str());
    } else if let Some(text) = b.pending_write.take() {
        let _ = clipboard.set_text(&text);
    }

    // Read: refresh cache on potential paste chord (Ctrl+V / Cmd+V)
    let paste_chord = keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight);
    if paste_chord && keys.just_pressed(KeyCode::KeyV) {
        let mut read = clipboard.fetch_text();
        if let Some(Ok(text)) = read.poll_result() {
            // Inject paste directly for immediate handling
            state.runtime.paste_into_focused(&text);
            b.cached_read = Some(text);
            state.force_compose = true;
        }
    }
}

/// Apply IME enable + caret area to the primary window.
pub fn apply_ime_system(
    output: Res<ReposeOutput>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };

    window.ime_enabled = output.ime_allowed;
    if let Some((x, y, _w, _h)) = output.ime_cursor_area {
        window.ime_position = Vec2::new(x as f32, y as f32);
    }
}

/// Forward Bevy IME messages into ReposeRuntime.
pub fn ime_input_system(mut events: MessageReader<Ime>, mut state: NonSendMut<ReposeState>) {
    for ev in events.read() {
        let ime = match ev {
            Ime::Enabled { .. } => ImeEvent::Start,
            Ime::Preedit { value, cursor, .. } => ImeEvent::Update {
                text: value.clone(),
                cursor: *cursor,
            },
            Ime::Commit { value, .. } => ImeEvent::Commit(value.clone()),
            Ime::Disabled { .. } => ImeEvent::Cancel,
        };
        state.runtime.handle_ime(&ime);
        state.force_compose = true;
    }
}
