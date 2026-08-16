use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};

use crate::plugin::ReposeSettingsRes;
use crate::state::{ReposeOutput, ReposeState};
use repose_core::{Frame, RenderContext, Scheduler, View, ViewKind};

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

    let frame = Frame {
        scene: frame_out.scene.clone(),
        hit_regions: frame_out.hit_regions,
        semantics_nodes: frame_out.semantics_nodes,
        focus_chain: frame_out.focus_chain,
    };

    let mut scene = frame.scene.clone();
    repose_core::dnd::overlay_drag_indicator(&mut scene, state.runtime.mouse_pos_px, false);

    let scale_factor = state.scale_factor;
    state.runtime.after_compose(&frame, scale_factor);
    state.runtime.cache_frame(frame);
    state.runtime.tick_overlays(state.last_redraw);
    state.last_redraw = web_time::Instant::now();

    state.scene = scene;
    state.last_platform = frame_out.platform.clone();

    output.cursor = frame_out
        .platform
        .cursor
        .or_else(|| state.runtime.cursor_suggestion());
    output.ime_allowed = frame_out.platform.ime_allowed;
    output.ime_cursor_area = frame_out.platform.ime_cursor_area;
    output.ime_purpose = frame_out.platform.ime_purpose;
    output.ime_auto_correct = frame_out.platform.ime_auto_correct;
    output.ime_capitalization = frame_out.platform.ime_capitalization;
    output.clipboard_text = frame_out.platform.clipboard_text;
    output.wants_pointer = frame_out.wants_pointer;
    output.wants_keyboard = frame_out.wants_keyboard;
    output.needs_redraw = true;

    state.force_compose = false;
}
