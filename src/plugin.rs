use bevy::prelude::*;
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    sync::{Mutex, OnceLock},
};

use crate::compose::{compose_repose_system, sync_viewport_system};
use crate::cursor::apply_cursor_system;
use crate::input::{
    cursor_left_system, keyboard_system, mouse_button_system, pointer_move_system, scroll_system,
    window_focus_system,
};
use crate::platform::{
    ClipboardBridge, apply_ime_system, clipboard_system, ime_input_system, install_clipboard_hooks,
};
use crate::render::ReposeRenderPlugin;
use crate::state::{ReposeOutput, ReposeState};
use repose_core::{RenderContext, Scheduler, View};

#[derive(Clone, Debug)]
pub struct ReposePluginSettings {
    pub clear_alpha: f32,
    pub compose_every_frame: bool,
    pub msaa_samples: u32,
    pub overlay: bool,
    pub sampler: bevy::image::ImageSampler,
}

impl Default for ReposePluginSettings {
    fn default() -> Self {
        Self {
            clear_alpha: 0.0,
            compose_every_frame: true,
            msaa_samples: 4,
            overlay: true,
            sampler: bevy::image::ImageSampler::nearest(),
        }
    }
}

fn register_font_once(bytes: &'static [u8]) {
    static REGISTERED: OnceLock<Mutex<HashSet<(u64, usize)>>> = OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    let key = (hasher.finish(), bytes.len());
    let registered = REGISTERED.get_or_init(|| Mutex::new(HashSet::new()));
    let mut registered = registered.lock().unwrap();
    if registered.insert(key) {
        repose_text::register_font_data(bytes);
    }
}

pub struct ReposePlugin {
    pub settings: ReposePluginSettings,
    root: Mutex<Option<Box<dyn FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static>>>,
    fonts: Vec<&'static [u8]>,
}

impl ReposePlugin {
    pub fn new<F>(root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            settings: ReposePluginSettings::default(),
            root: Mutex::new(Some(Box::new(root))),
            fonts: Vec::new(),
        }
    }

    pub fn with_settings<F>(settings: ReposePluginSettings, root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            settings,
            root: Mutex::new(Some(Box::new(root))),
            fonts: Vec::new(),
        }
    }

    pub fn with_font_bytes(mut self, bytes: &'static [u8]) -> Self {
        self.fonts.push(bytes);
        self
    }
}

#[derive(Resource, Clone, Debug)]
pub struct ReposeSettingsRes(pub ReposePluginSettings);

impl Plugin for ReposePlugin {
    fn build(&self, app: &mut App) {
        for font in &self.fonts {
            register_font_once(font);
        }

        let settings = self.settings.clone();

        let root = self
            .root
            .lock()
            .unwrap()
            .take()
            .expect("ReposePlugin root already consumed");

        let bridge = ClipboardBridge::default();
        install_clipboard_hooks(bridge.clone());

        // NOTE: is already the default in animation/ but might change it later
        // repose_core::animation::set_clock(Box::new(repose_core::animation::SystemClock));

        app.insert_resource(ReposeSettingsRes(settings.clone()))
            .insert_resource(ReposeOutput::default())
            .insert_resource(bridge)
            .insert_non_send(ReposeState::new(root))
            .add_plugins(ReposeRenderPlugin {
                settings: settings.clone(),
            })
            .add_systems(
                PreUpdate,
                (
                    sync_viewport_system,
                    pointer_move_system,
                    mouse_button_system,
                    scroll_system,
                    ime_input_system,
                    keyboard_system,
                    cursor_left_system,
                    window_focus_system,
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    compose_repose_system,
                    apply_cursor_system,
                    apply_ime_system,
                    clipboard_system,
                )
                    .chain(),
            );
    }
}
