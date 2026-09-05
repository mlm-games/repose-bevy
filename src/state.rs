use std::sync::Arc;

use bevy::prelude::*;
use parking_lot::Mutex;
use repose_app::PlatformOutput;
use repose_core::{RenderContext, Scene, Scheduler, View};

pub type ReposeRootFn =
    Box<dyn FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static>;

pub type BevyClickQueue = Arc<Mutex<Vec<Box<dyn FnOnce(&mut World) + Send + 'static>>>>;

pub struct ReposeState {
    pub runtime: repose_app::ReposeRuntime,
    pub render_ctx: RenderContext,
    pub scene: Scene,
    pub fb_width: u32,
    pub fb_height: u32,
    pub scale_factor: f32,
    pub force_compose: bool,
    pub root: ReposeRootFn,
    pub panels: Vec<ReposeRootFn>,
    pub pending_bevy_clicks: BevyClickQueue,
    pub last_platform: PlatformOutput,
    pub last_redraw: web_time::Instant,
}

pub struct ReposePendingPanels(pub Vec<ReposeRootFn>);

impl Default for ReposePendingPanels {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl ReposeState {
    pub fn new<F>(root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        let mut root = root;
        Self::new_with_world(move |_world, s, c| root(s, c))
    }

    pub fn new_with_world<F>(root: F) -> Self
    where
        F: FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static,
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
            panels: Vec::new(),
            pending_bevy_clicks: Arc::new(Mutex::new(Vec::new())),
            last_platform: PlatformOutput::default(),
            last_redraw: web_time::Instant::now(),
        }
    }

    pub fn from_boxed(root: ReposeRootFn) -> Self {
        Self {
            runtime: repose_app::ReposeRuntime::new(),
            render_ctx: RenderContext::new(),
            scene: Scene::default(),
            fb_width: 1,
            fb_height: 1,
            scale_factor: 1.0,
            force_compose: true,
            root,
            panels: Vec::new(),
            pending_bevy_clicks: Arc::new(Mutex::new(Vec::new())),
            last_platform: PlatformOutput::default(),
            last_redraw: web_time::Instant::now(),
        }
    }
}

#[derive(Resource, Default, Debug, Clone)]
pub struct ReposeOutput {
    pub cursor: Option<repose_core::CursorIcon>,
    pub ime_allowed: bool,
    pub ime_cursor_area: Option<(f64, f64, f64, f64)>,
    pub ime_purpose: repose_core::ImePurposeHint,
    pub ime_auto_correct: bool,
    pub ime_capitalization: repose_core::KeyboardCapitalization,
    pub clipboard_text: Option<String>,
    pub wants_pointer: bool,
    pub wants_keyboard: bool,
    pub pointer_consumed: bool,
    pub scroll_consumed: bool,
    pub keyboard_consumed: bool,
    pub needs_redraw: bool,
}

impl ReposeOutput {
    pub fn wants_pointer(&self) -> bool {
        self.wants_pointer || self.pointer_consumed
    }
    pub fn wants_keyboard(&self) -> bool {
        self.wants_keyboard || self.keyboard_consumed
    }
    pub fn any_consumed(&self) -> bool {
        self.pointer_consumed || self.scroll_consumed || self.keyboard_consumed
    }
}

pub fn repose_wants_pointer(output: Res<ReposeOutput>) -> bool {
    output.wants_pointer
}
pub fn repose_pointer_consumed(output: Res<ReposeOutput>) -> bool {
    output.pointer_consumed
}
pub fn repose_scroll_consumed(output: Res<ReposeOutput>) -> bool {
    output.scroll_consumed
}
pub fn repose_wants_keyboard(output: Res<ReposeOutput>) -> bool {
    output.wants_keyboard
}
pub mod gating {
    use super::*;
    pub fn wants_pointer(o: Res<ReposeOutput>) -> bool {
        o.wants_pointer
    }
    pub fn wants_keyboard(o: Res<ReposeOutput>) -> bool {
        o.wants_keyboard
    }
    pub fn pointer_consumed(o: Res<ReposeOutput>) -> bool {
        o.pointer_consumed
    }
    pub fn scroll_consumed(o: Res<ReposeOutput>) -> bool {
        o.scroll_consumed
    }
}

#[derive(Resource, Clone, Debug)]
pub struct ReposeUiImage {
    pub image: Handle<Image>,
    pub width: u32,
    pub height: u32,
}

#[derive(Resource, Default)]
pub struct ReposeFrameRequest(std::sync::atomic::AtomicBool);

impl ReposeFrameRequest {
    pub fn request(&self) {
        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn take(&self) -> bool {
        self.0.swap(false, std::sync::atomic::Ordering::Relaxed)
    }
    pub fn is_requested(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub fn request_repose_frame(world: &mut World) {
    if let Some(req) = world.get_resource::<ReposeFrameRequest>() {
        req.request();
    }
}
pub fn request_repose_frame_res(req: Res<ReposeFrameRequest>) {
    req.request();
}
