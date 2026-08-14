use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};

use crate::plugin::ReposeSettingsRes;
use crate::state::{ReposeOutput, ReposeState};
use repose_core::{RenderContext, Scheduler, View, ViewKind};

pub fn sync_viewport_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: NonSendMut<ReposeState>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.resolution.scale_factor();
    let w = window.resolution.physical_width().max(1);
    let h = window.resolution.physical_height().max(1);

    if state.fb_width != w
        || state.fb_height != h
        || (state.scale_factor - scale).abs() > f32::EPSILON
    {
        state.fb_width = w;
        state.fb_height = h;
        state.scale_factor = scale;
        state.runtime.set_viewport_and_scale(w, h, scale);
        state.force_compose = true;
    }
}

pub fn compose_repose_system(
    settings: Res<ReposeSettingsRes>,
    mut state: NonSendMut<ReposeState>,
    mut output: ResMut<ReposeOutput>,
) {
    let needs = state.force_compose
        || settings.0.compose_every_frame
        || repose_core::take_frame_request()
        || repose_core::take_signal_fired();

    if !needs {
        output.needs_redraw = false;
        return;
    }

    state.runtime.tick_animations();

    let mut root = std::mem::replace(
        &mut state.root,
        Box::new(|_: &mut Scheduler, _: &RenderContext| View::new(0, ViewKind::Box)),
    );
    let render_ctx = state.render_ctx.clone();
    let frame = state.runtime.compose(&mut root, &render_ctx);
    state.root = root;

    let wants_pointer = state.runtime.hover_id.is_some() || state.runtime.capture_id.is_some();
    let wants_keyboard = !state.runtime.textfield_states.is_empty() || state.runtime.ime_preedit;

    let ime_allowed = state.runtime.sched.focused.is_some_and(|fid| {
        frame
            .semantics_nodes
            .iter()
            .any(|n| n.id == fid && n.role == repose_core::semantics::Role::TextField)
    });

    let ime_cursor_area = if ime_allowed {
        state.runtime.sched.focused.and_then(|fid| {
            frame.hit_regions.iter().find(|h| h.id == fid).map(|hit| {
                let sf = state.scale_factor as f64;
                (
                    hit.rect.x as f64 / sf,
                    hit.rect.y as f64 / sf,
                    hit.rect.w as f64 / sf,
                    hit.rect.h as f64 / sf,
                )
            })
        })
    } else {
        None
    };

    state.scene = frame.scene.clone();
    state.runtime.reconcile_hover_from_mouse_pos(&frame);
    state.runtime.cache_frame(frame);

    let cursor = state.runtime.cursor_suggestion();

    output.cursor = cursor;
    output.ime_allowed = ime_allowed;
    output.ime_cursor_area = ime_cursor_area;
    output.clipboard_text = None;
    output.wants_pointer = wants_pointer;
    output.wants_keyboard = wants_keyboard;
    output.needs_redraw = true;

    state.force_compose = false;
    state.last_output.wants_pointer = wants_pointer;
    state.last_output.wants_keyboard = wants_keyboard;
    state.last_output.platform_cursor = cursor;
    state.last_output.ime_allowed = ime_allowed;
    state.last_output.ime_cursor_area = ime_cursor_area;
}
