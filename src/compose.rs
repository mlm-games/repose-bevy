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
    let frame_out = state.runtime.frame(&mut root, &render_ctx);
    state.root = root;

    let frame = repose_core::runtime::Frame {
        scene: frame_out.scene.clone(),
        hit_regions: frame_out.hit_regions,
        semantics_nodes: frame_out.semantics_nodes,
        focus_chain: frame_out.focus_chain,
    };

    state.scene = frame_out.scene;
    state.runtime.reconcile_hover_from_mouse_pos(&frame);
    state.runtime.cache_frame(frame);

    output.cursor = frame_out.platform.cursor;
    output.ime_allowed = frame_out.platform.ime_allowed;
    output.ime_cursor_area = frame_out.platform.ime_cursor_area;
    output.clipboard_text = frame_out.platform.clipboard_text;
    output.wants_pointer = frame_out.wants_pointer;
    output.wants_keyboard = frame_out.wants_keyboard;
    output.needs_redraw = true;

    state.force_compose = false;
    state.last_output.wants_pointer = frame_out.wants_pointer;
    state.last_output.wants_keyboard = frame_out.wants_keyboard;
    state.last_output.platform_cursor = frame_out.platform.cursor;
    state.last_output.ime_allowed = frame_out.platform.ime_allowed;
    state.last_output.ime_cursor_area = frame_out.platform.ime_cursor_area;
}
