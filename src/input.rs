use bevy::input::ButtonState;
use bevy::input::keyboard::{Key as BevyKey, KeyboardInput};
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorLeft, CursorMoved, PrimaryWindow, WindowEvent, WindowFocused};
use repose_core::Vec2;
use repose_core::input::{Key, KeyEvent, KeyEventType, Modifiers, PhysicalKey, PointerButton};

use crate::state::{ReposeOutput, ReposeState};

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
    let command = if cfg!(target_os = "macos") {
        meta
    } else {
        ctrl
    };
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
    mut cursor_moved: MessageReader<CursorMoved>,
    windows: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    state.runtime.modifiers = modifiers_from_input(&keys);

    let mut moved = false;
    let mut last = None;

    for event in window_events.read() {
        if let WindowEvent::CursorMoved(CursorMoved { position, .. }) = event {
            last = Some(*position);
            moved = true;
        }
    }
    for ev in cursor_moved.read() {
        last = Some(ev.position);
        moved = true;
    }

    if last.is_none() {
        last = window.cursor_position();
    }

    if last.is_none() {
        if state.runtime.pointer_inside {
            state.runtime.pointer_inside = false;
            state.runtime.clear_hover();
            state.force_compose = true;
        }
        return;
    }

    if let Some(position) = last {
        state.runtime.pointer_inside = true;
        let pos = physical_pos(
            window,
            Vec2 {
                x: position.x,
                y: position.y,
            },
        );
        let before = state.runtime.hover_id;
        let _ = state.runtime.handle_pointer_move(pos);
        if moved || state.runtime.hover_id != before {
            state.force_compose = true;
        }
    }
}

pub fn cursor_left_system(
    mut window_events: MessageReader<WindowEvent>,
    mut state: NonSendMut<ReposeState>,
) {
    for event in window_events.read() {
        if let WindowEvent::CursorLeft(CursorLeft { .. }) = event {
            state.runtime.pointer_inside = false;
            state.runtime.clear_hover();
            state.force_compose = true;
        }
    }
}

pub fn mouse_button_system(
    mut button_events: MessageReader<MouseButtonInput>,
    windows: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
    mut output: ResMut<ReposeOutput>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    state.runtime.modifiers = modifiers_from_input(&keys);

    let cursor_pos = window
        .cursor_position()
        .unwrap_or(bevy::prelude::Vec2::ZERO);
    let pos = physical_pos(
        window,
        Vec2 {
            x: cursor_pos.x,
            y: cursor_pos.y,
        },
    );

    for ev in button_events.read() {
        let button = match ev.button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            MouseButton::Middle => PointerButton::Tertiary,
            _ => continue,
        };
        match ev.state {
            ButtonState::Pressed => {
                let result = state.runtime.handle_pointer_press(pos, button);
                if output.pointer_consumed != result.consumed {
                    output.pointer_consumed = result.consumed;
                }
                state.force_compose = true;
            }
            ButtonState::Released => {
                state.runtime.handle_pointer_release(pos, button);
                if output.pointer_consumed {
                    output.pointer_consumed = false;
                }
                state.force_compose = true;
            }
        }
    }
}

pub fn scroll_system(
    mut wheel: MessageReader<MouseWheel>,
    mut state: NonSendMut<ReposeState>,
    mut output: ResMut<ReposeOutput>,
) {
    let mut any_consumed = false;
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
            any_consumed = true;
        }
    }
    if output.scroll_consumed != any_consumed {
        output.scroll_consumed = any_consumed;
    }
}

fn map_key(key: &BevyKey) -> Key {
    match key {
        BevyKey::Character(c) => {
            let ch = c.chars().next().unwrap_or('\0');
            Key::Character(ch)
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

/// Bevy's `KeyCode` and Repose's `PhysicalKey` both mean "key position,
/// independent of layout", so names line up 1:1. Bevy's extra media and
/// F13-F35 keys have no W3C `code`, so they read as `Unidentified`.
fn map_physical(key: KeyCode) -> PhysicalKey {
    match key {
        KeyCode::AltLeft => PhysicalKey::AltLeft,
        KeyCode::AltRight => PhysicalKey::AltRight,
        KeyCode::ArrowDown => PhysicalKey::ArrowDown,
        KeyCode::ArrowLeft => PhysicalKey::ArrowLeft,
        KeyCode::ArrowRight => PhysicalKey::ArrowRight,
        KeyCode::ArrowUp => PhysicalKey::ArrowUp,
        KeyCode::Backquote => PhysicalKey::Backquote,
        KeyCode::Backslash => PhysicalKey::Backslash,
        KeyCode::Backspace => PhysicalKey::Backspace,
        KeyCode::BracketLeft => PhysicalKey::BracketLeft,
        KeyCode::BracketRight => PhysicalKey::BracketRight,
        KeyCode::CapsLock => PhysicalKey::CapsLock,
        KeyCode::Comma => PhysicalKey::Comma,
        KeyCode::ContextMenu => PhysicalKey::ContextMenu,
        KeyCode::ControlLeft => PhysicalKey::ControlLeft,
        KeyCode::ControlRight => PhysicalKey::ControlRight,
        KeyCode::Delete => PhysicalKey::Delete,
        KeyCode::Digit0 => PhysicalKey::Digit0,
        KeyCode::Digit1 => PhysicalKey::Digit1,
        KeyCode::Digit2 => PhysicalKey::Digit2,
        KeyCode::Digit3 => PhysicalKey::Digit3,
        KeyCode::Digit4 => PhysicalKey::Digit4,
        KeyCode::Digit5 => PhysicalKey::Digit5,
        KeyCode::Digit6 => PhysicalKey::Digit6,
        KeyCode::Digit7 => PhysicalKey::Digit7,
        KeyCode::Digit8 => PhysicalKey::Digit8,
        KeyCode::Digit9 => PhysicalKey::Digit9,
        KeyCode::End => PhysicalKey::End,
        KeyCode::Enter => PhysicalKey::Enter,
        KeyCode::Equal => PhysicalKey::Equal,
        KeyCode::Escape => PhysicalKey::Escape,
        KeyCode::F1 => PhysicalKey::F1,
        KeyCode::F10 => PhysicalKey::F10,
        KeyCode::F11 => PhysicalKey::F11,
        KeyCode::F12 => PhysicalKey::F12,
        KeyCode::F2 => PhysicalKey::F2,
        KeyCode::F3 => PhysicalKey::F3,
        KeyCode::F4 => PhysicalKey::F4,
        KeyCode::F5 => PhysicalKey::F5,
        KeyCode::F6 => PhysicalKey::F6,
        KeyCode::F7 => PhysicalKey::F7,
        KeyCode::F8 => PhysicalKey::F8,
        KeyCode::F9 => PhysicalKey::F9,
        KeyCode::Home => PhysicalKey::Home,
        KeyCode::Insert => PhysicalKey::Insert,
        KeyCode::IntlBackslash => PhysicalKey::IntlBackslash,
        KeyCode::IntlRo => PhysicalKey::IntlRo,
        KeyCode::IntlYen => PhysicalKey::IntlYen,
        KeyCode::KeyA => PhysicalKey::KeyA,
        KeyCode::KeyB => PhysicalKey::KeyB,
        KeyCode::KeyC => PhysicalKey::KeyC,
        KeyCode::KeyD => PhysicalKey::KeyD,
        KeyCode::KeyE => PhysicalKey::KeyE,
        KeyCode::KeyF => PhysicalKey::KeyF,
        KeyCode::KeyG => PhysicalKey::KeyG,
        KeyCode::KeyH => PhysicalKey::KeyH,
        KeyCode::KeyI => PhysicalKey::KeyI,
        KeyCode::KeyJ => PhysicalKey::KeyJ,
        KeyCode::KeyK => PhysicalKey::KeyK,
        KeyCode::KeyL => PhysicalKey::KeyL,
        KeyCode::KeyM => PhysicalKey::KeyM,
        KeyCode::KeyN => PhysicalKey::KeyN,
        KeyCode::KeyO => PhysicalKey::KeyO,
        KeyCode::KeyP => PhysicalKey::KeyP,
        KeyCode::KeyQ => PhysicalKey::KeyQ,
        KeyCode::KeyR => PhysicalKey::KeyR,
        KeyCode::KeyS => PhysicalKey::KeyS,
        KeyCode::KeyT => PhysicalKey::KeyT,
        KeyCode::KeyU => PhysicalKey::KeyU,
        KeyCode::KeyV => PhysicalKey::KeyV,
        KeyCode::KeyW => PhysicalKey::KeyW,
        KeyCode::KeyX => PhysicalKey::KeyX,
        KeyCode::KeyY => PhysicalKey::KeyY,
        KeyCode::KeyZ => PhysicalKey::KeyZ,
        KeyCode::Minus => PhysicalKey::Minus,
        KeyCode::NumLock => PhysicalKey::NumLock,
        KeyCode::Numpad0 => PhysicalKey::Numpad0,
        KeyCode::Numpad1 => PhysicalKey::Numpad1,
        KeyCode::Numpad2 => PhysicalKey::Numpad2,
        KeyCode::Numpad3 => PhysicalKey::Numpad3,
        KeyCode::Numpad4 => PhysicalKey::Numpad4,
        KeyCode::Numpad5 => PhysicalKey::Numpad5,
        KeyCode::Numpad6 => PhysicalKey::Numpad6,
        KeyCode::Numpad7 => PhysicalKey::Numpad7,
        KeyCode::Numpad8 => PhysicalKey::Numpad8,
        KeyCode::Numpad9 => PhysicalKey::Numpad9,
        KeyCode::NumpadAdd => PhysicalKey::NumpadAdd,
        KeyCode::NumpadDecimal => PhysicalKey::NumpadDecimal,
        KeyCode::NumpadDivide => PhysicalKey::NumpadDivide,
        KeyCode::NumpadEnter => PhysicalKey::NumpadEnter,
        KeyCode::NumpadMultiply => PhysicalKey::NumpadMultiply,
        KeyCode::NumpadSubtract => PhysicalKey::NumpadSubtract,
        KeyCode::PageDown => PhysicalKey::PageDown,
        KeyCode::PageUp => PhysicalKey::PageUp,
        KeyCode::Pause => PhysicalKey::Pause,
        KeyCode::Period => PhysicalKey::Period,
        KeyCode::PrintScreen => PhysicalKey::PrintScreen,
        KeyCode::Quote => PhysicalKey::Quote,
        KeyCode::ScrollLock => PhysicalKey::ScrollLock,
        KeyCode::Semicolon => PhysicalKey::Semicolon,
        KeyCode::ShiftLeft => PhysicalKey::ShiftLeft,
        KeyCode::ShiftRight => PhysicalKey::ShiftRight,
        KeyCode::Slash => PhysicalKey::Slash,
        KeyCode::Space => PhysicalKey::Space,
        KeyCode::SuperLeft => PhysicalKey::SuperLeft,
        KeyCode::SuperRight => PhysicalKey::SuperRight,
        KeyCode::Tab => PhysicalKey::Tab,
        _ => PhysicalKey::Unidentified,
    }
}

pub fn keyboard_system(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: NonSendMut<ReposeState>,
    mut output: ResMut<ReposeOutput>,
) {
    let mods = modifiers_from_input(&keys);
    state.runtime.modifiers = mods;

    let mut any_consumed = false;
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
            physical: Some(map_physical(ev.key_code)),
        };
        if state.runtime.handle_key_with_text(&ke, ev.text.as_deref()) {
            state.force_compose = true;
            any_consumed = true;
        }
    }
    if output.keyboard_consumed != any_consumed {
        output.keyboard_consumed = any_consumed;
    }
}

pub fn window_focus_system(
    mut window_events: MessageReader<WindowEvent>,
    mut state: NonSendMut<ReposeState>,
    mut output: ResMut<ReposeOutput>,
) {
    for event in window_events.read() {
        if let WindowEvent::WindowFocused(WindowFocused { focused: false, .. }) = event {
            state.runtime.handle_focus_lost();
            if output.pointer_consumed {
                output.pointer_consumed = false;
            }
            state.force_compose = true;
        }
    }
}
