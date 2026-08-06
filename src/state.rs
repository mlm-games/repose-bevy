use bevy::prelude::*;
use repose_core::{RenderContext, Scene, Scheduler, View};

pub struct ReposeState {
    pub runtime: repose_app::ReposeRuntime,
    pub render_ctx: RenderContext,
    pub scene: Scene,
    pub fb_width: u32,
    pub fb_height: u32,
    pub scale_factor: f32,
    pub force_compose: bool,
    pub root: Box<dyn FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static>,
    pub last_output: FrameOutputShell,
}

#[derive(Default)]
pub struct FrameOutputShell {
    pub platform_cursor: Option<repose_core::CursorIcon>,
    pub ime_allowed: bool,
    pub ime_cursor_area: Option<(f64, f64, f64, f64)>,
    pub clipboard_text: Option<String>,
    pub wants_pointer: bool,
    pub wants_keyboard: bool,
}

impl ReposeState {
    pub fn new<F>(root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            runtime: repose_app::ReposeRuntime::new(),
            render_ctx: RenderContext::new(),
            scene: Scene::default(),
            fb_width: 1,
            fb_height: 1,
            scale_factor: 1.0,
            force_compose: true,
            root: Box::new(root),
            last_output: FrameOutputShell::default(),
        }
    }
}

#[derive(Resource, Default, Debug, Clone)]
pub struct ReposeOutput {
    pub cursor: Option<repose_core::CursorIcon>,
    pub ime_allowed: bool,
    pub ime_cursor_area: Option<(f64, f64, f64, f64)>,
    pub clipboard_text: Option<String>,
    pub wants_pointer: bool,
    pub wants_keyboard: bool,
    /// True from the frame a pointer press was consumed by UI until the button
    /// is released. Lets consumers block world input (e.g. drag-place) while
    /// the pointer is captured by a UI element.
    pub pointer_consumed: bool,
    pub needs_redraw: bool,
}

#[derive(Resource, Clone, Debug)]
pub struct ReposeUiImage {
    pub image: Handle<Image>,
    pub width: u32,
    pub height: u32,
}
