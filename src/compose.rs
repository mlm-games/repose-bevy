use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};

use crate::bridge::pending_scope;
use crate::plugin::ReposeSettingsRes;
use crate::state::{ReposeOutput, ReposeState};
use repose_core::{RenderContext, Scheduler, View, ViewKind};
use repose_ui::ViewExt;

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

pub fn compose_repose_system(world: &mut World) {
    let needs = {
        let settings = world.resource::<ReposeSettingsRes>();
        let state = world.non_send::<ReposeState>();
        // Bind eagerly: `||` short-circuits, so `a || b` would leave `b` flag set.
        let a = repose_core::take_frame_request();
        let b = repose_core::take_signal_fired();
        let frame_requested = a || b;
        let bevy_requested = world
            .get_resource::<crate::state::ReposeFrameRequest>()
            .is_some_and(|r| r.take());
        state.force_compose
            || settings.0.compose_every_frame
            || frame_requested
            || bevy_requested
    };

    if !needs {
        if let Some(mut output) = world.get_resource_mut::<ReposeOutput>() {
            if output.needs_redraw {
                output.needs_redraw = false;
            }
        }
        return;
    }

    let mut state = world
        .remove_non_send::<ReposeState>()
        .expect("ReposeState missing");

    let mut root = std::mem::replace(
        &mut state.root,
        Box::new(|_: &mut World, _: &mut Scheduler, _: &RenderContext| View::new(0, ViewKind::Box)),
    );
    let mut panels = std::mem::take(&mut state.panels);
    let render_ctx = state.render_ctx.clone();
    let pending = std::sync::Arc::clone(&state.pending_bevy_clicks);

    let frame_out = pending_scope(&pending, || {
        let mut wrapper = |sched: &mut Scheduler, ctx: &RenderContext| {
            let main_view = root(world, sched, ctx);
            if panels.is_empty() {
                return main_view;
            }
            let mut views = Vec::with_capacity(panels.len() + 1);
            views.push(main_view);
            for p in panels.iter_mut() {
                views.push(p(world, sched, ctx));
            }
            repose_ui::Box(repose_core::Modifier::new().fill_max_size()).child(views)
        };
        state.runtime.compose_frame_output(&mut wrapper, &render_ctx)
    });

    state.root = root;
    state.panels = panels;

    let mut scene = frame_out.scene;
    repose_core::dnd::overlay_drag_indicator(&mut scene, state.runtime.mouse_pos_px, false);

    state.runtime.tick_overlays(state.last_redraw);
    state.last_redraw = web_time::Instant::now();

    state.scene = scene;
    state.last_platform = frame_out.platform.clone();

    if let Some(mut output) = world.get_resource_mut::<ReposeOutput>() {
        let new_cursor = frame_out
            .platform
            .cursor
            .or_else(|| state.runtime.cursor_suggestion());
        if output.cursor != new_cursor {
            output.cursor = new_cursor;
        }
        if output.ime_allowed != frame_out.platform.ime_allowed {
            output.ime_allowed = frame_out.platform.ime_allowed;
        }
        if output.ime_cursor_area != frame_out.platform.ime_cursor_area {
            output.ime_cursor_area = frame_out.platform.ime_cursor_area;
        }
        if output.ime_purpose != frame_out.platform.ime_purpose {
            output.ime_purpose = frame_out.platform.ime_purpose;
        }
        if output.ime_auto_correct != frame_out.platform.ime_auto_correct {
            output.ime_auto_correct = frame_out.platform.ime_auto_correct;
        }
        if output.ime_capitalization != frame_out.platform.ime_capitalization {
            output.ime_capitalization = frame_out.platform.ime_capitalization;
        }
        if output.clipboard_text != frame_out.platform.clipboard_text {
            output.clipboard_text = frame_out.platform.clipboard_text.clone();
        }
        if output.wants_pointer != frame_out.wants_pointer {
            output.wants_pointer = frame_out.wants_pointer;
        }
        if output.wants_keyboard != frame_out.wants_keyboard {
            output.wants_keyboard = frame_out.wants_keyboard;
        }
        if !output.needs_redraw {
            output.needs_redraw = true;
        }
    }

    state.force_compose = false;

    world.insert_non_send(state);
}
