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

#[derive(Default)]
pub struct ClipboardState {
    /// Text Repose wants written to the OS clipboard (via `set_clipboard_fn`).
    pub pending_write: Option<String>,
    /// Text fetched from Bevy clipboard, ready for `paste_text()` reads.
    pub cached_read: Option<String>,
}

pub(crate) static GLOBAL_CLIPBOARD: std::sync::OnceLock<Arc<Mutex<ClipboardState>>> =
    std::sync::OnceLock::new();

/// Install Repose clipboard hooks that bridge through a process-global slot.
/// Repose's clipboard hooks are global function pointers; calling
/// `install_clipboard_hooks` per `App` would make the second `App` (e.g. in
/// `cargo test` multi-App hygiene) overwrite the first. We install exactly
/// once and every `App` shares the same backing `Arc<Mutex<_>>` so the hooks
/// remain valid regardless of which `World` inserted the `ClipboardBridge`.
pub fn install_clipboard_hooks(bridge: ClipboardBridge) {
    let canonical = GLOBAL_CLIPBOARD
        .get_or_init(|| bridge.0.clone())
        .clone();

    static INSTALLED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INSTALLED.get_or_init(|| {
        let wb = canonical.clone();
        repose_core::clipboard::set_clipboard_fn(Box::new(move |text| {
            wb.lock().pending_write = Some(text.to_string());
        }));
        let rb = canonical.clone();
        repose_core::clipboard::set_clipboard_read_fn(Box::new(move || {
            rb.lock().cached_read.take()
        }));
    });

}

/// Sync clipboard writes from Repose output to Bevy's Clipboard resource,
/// and refresh the read cache from Bevy's Clipboard. `Clipboard` is optional
/// so headless CI without a display (or where `Clipboard: Default` would
/// eagerly open OS clipboard) doesn't panic.
pub fn clipboard_system(
    bridge: Res<ClipboardBridge>,
    clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    output: Res<ReposeOutput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
) {
    let mut b = bridge.0.lock();

    if let Some(mut clipboard) = clipboard {
        if let Some(text) = &output.clipboard_text {
            let _ = clipboard.set_text(text.as_str());
        } else if let Some(text) = b.pending_write.take() {
            let _ = clipboard.set_text(&text);
        }

        let paste_chord = keys.pressed(KeyCode::ControlLeft)
            || keys.pressed(KeyCode::ControlRight)
            || keys.pressed(KeyCode::SuperLeft)
            || keys.pressed(KeyCode::SuperRight);
        if paste_chord && keys.just_pressed(KeyCode::KeyV) {
            let mut read = clipboard.fetch_text();
            if let Some(Ok(text)) = read.poll_result() {
                b.cached_read = Some(text);
                if state
                    .runtime
                    .dispatch_action(repose_core::shortcuts::Action::Paste)
                {
                    state.force_compose = true;
                }
            }
        }
    } else if b.pending_write.is_some() {
        b.pending_write = None;
    }
}

/// Apply IME enable + caret area to the primary window.
/// Writes are conditional so we don't dirty `Window` every frame at Citybound scale.
pub fn apply_ime_system(
    output: Res<ReposeOutput>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };

    if window.ime_enabled != output.ime_allowed {
        window.ime_enabled = output.ime_allowed;
    }
    if let Some((x, y, _w, _h)) = output.ime_cursor_area {
        let pos = Vec2::new(x as f32, y as f32);
        if window.ime_position != pos {
            window.ime_position = pos;
        }
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
