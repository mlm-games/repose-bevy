use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::input::keyboard::{Key as BevyKey, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{CursorMoved, PrimaryWindow, WindowEvent, WindowFocused};
use repose_core::input::{
    Key, KeyEvent, KeyEventType, Modifiers, PointerButton,
};
use repose_core::Vec2;

use crate::state::ReposeState;

fn physical_pos(window: &Window, logical: Vec2) -> Vec2 {
    let sf = window.resolution.scale_factor();
    Vec2 {
        x: logical.x * sf,
        y: logical.y * sf,
    }
}

fn modifiers_from_input(keys: &ButtonInput<KeyCode>) -> Modifiers {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let meta = keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight);
    let command = if cfg!(target_os = "macos") { meta } else { ctrl };
    Modifiers {
        shift,
        ctrl,
        alt,
        meta,
        command,
    }
}

pub fn pointer_move_system(
    mut window_events: MessageReader<WindowEvent>,
    windows: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    state.runtime.modifiers = modifiers_from_input(&keys);

    for event in window_events.read() {
        if let WindowEvent::CursorMoved(CursorMoved { position, .. }) = event {
            let pos = physical_pos(window, Vec2 { x: position.x, y: position.y });
            let _ = state.runtime.handle_pointer_move(pos);
        }
    }
}

pub fn mouse_button_system(
    mut button_events: MessageReader<MouseButtonInput>,
    windows: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    state.runtime.modifiers = modifiers_from_input(&keys);

    let cursor_pos = window.cursor_position().unwrap_or(bevy::prelude::Vec2::ZERO);
    let pos = physical_pos(window, Vec2 { x: cursor_pos.x, y: cursor_pos.y });

    for ev in button_events.read() {
        let button = match ev.button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            MouseButton::Middle => PointerButton::Tertiary,
            _ => continue,
        };
        match ev.state {
            ButtonState::Pressed => {
                let _ = state.runtime.handle_pointer_press(pos, button);
                state.force_compose = true;
            }
            ButtonState::Released => {
                state.runtime.handle_pointer_release(pos, button);
                state.force_compose = true;
            }
        }
    }
}

pub fn scroll_system(
    mut wheel: MessageReader<MouseWheel>,
    mut state: NonSendMut<ReposeState>,
) {
    for ev in wheel.read() {
        let scale = match ev.unit {
            bevy::input::mouse::MouseScrollUnit::Line => 32.0,
            bevy::input::mouse::MouseScrollUnit::Pixel => 1.0,
        };
        let delta = Vec2 {
            x: ev.x * scale,
            y: -ev.y * scale,
        };
        if state.runtime.handle_scroll(delta) {
            state.force_compose = true;
        }
    }
}

fn map_key(key: &BevyKey) -> Key {
    match key {
        BevyKey::Character(c) => {
            let ch = c.chars().next().unwrap_or('\0');
            Key::Character(ch.to_ascii_lowercase())
        }
        BevyKey::Enter => Key::Enter,
        BevyKey::Tab => Key::Tab,
        BevyKey::Backspace => Key::Backspace,
        BevyKey::Delete => Key::Delete,
        BevyKey::Escape => Key::Escape,
        BevyKey::ArrowLeft => Key::ArrowLeft,
        BevyKey::ArrowRight => Key::ArrowRight,
        BevyKey::ArrowUp => Key::ArrowUp,
        BevyKey::ArrowDown => Key::ArrowDown,
        BevyKey::Home => Key::Home,
        BevyKey::End => Key::End,
        BevyKey::PageUp => Key::PageUp,
        BevyKey::PageDown => Key::PageDown,
        BevyKey::Space => Key::Space,
        BevyKey::F1 => Key::F(1),
        BevyKey::F2 => Key::F(2),
        BevyKey::F3 => Key::F(3),
        BevyKey::F4 => Key::F(4),
        BevyKey::F5 => Key::F(5),
        BevyKey::F6 => Key::F(6),
        BevyKey::F7 => Key::F(7),
        BevyKey::F8 => Key::F(8),
        BevyKey::F9 => Key::F(9),
        BevyKey::F10 => Key::F(10),
        BevyKey::F11 => Key::F(11),
        BevyKey::F12 => Key::F(12),
        _ => Key::Unknown,
    }
}

pub fn keyboard_system(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
) {
    let mods = modifiers_from_input(&keys);
    state.runtime.modifiers = mods;

    for ev in events.read() {
        let key = map_key(&ev.logical_key);
        let event_type = match ev.state {
            ButtonState::Pressed => KeyEventType::Down,
            ButtonState::Released => KeyEventType::Up,
        };
        let utf16 = match &key {
            Key::Character(c) => *c as u16,
            _ => 0,
        };
        let ke = KeyEvent {
            key,
            modifiers: mods,
            is_repeat: ev.repeat,
            event_type,
            utf16_code_point: utf16,
        };
        if state.runtime.handle_key(&ke) {
            state.force_compose = true;
        }
    }
}

pub fn window_focus_system(
    mut window_events: MessageReader<WindowEvent>,
    mut state: NonSendMut<ReposeState>,
) {
    for event in window_events.read() {
        if let WindowEvent::WindowFocused(WindowFocused { focused: false, .. }) = event {
            state.runtime.handle_focus_lost();
            state.force_compose = true;
        }
    }
}
