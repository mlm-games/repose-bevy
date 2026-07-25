use bevy::prelude::*;
use std::sync::Mutex;

use crate::compose::{compose_repose_system, sync_viewport_system};
use crate::cursor::apply_cursor_system;
use crate::input::{
    keyboard_system, mouse_button_system, pointer_move_system, scroll_system, window_focus_system,
};
use crate::platform::{
    apply_ime_system, clipboard_system, ime_input_system, install_clipboard_hooks, ClipboardBridge,
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
}

impl Default for ReposePluginSettings {
    fn default() -> Self {
        Self {
            clear_alpha: 0.0,
            compose_every_frame: true,
            msaa_samples: 1,
            overlay: true,
        }
    }
}

pub struct ReposePlugin {
    pub settings: ReposePluginSettings,
    root: Mutex<Option<Box<dyn FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static>>>,
}

impl ReposePlugin {
    pub fn new<F>(root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            settings: ReposePluginSettings::default(),
            root: Mutex::new(Some(Box::new(root))),
        }
    }

    pub fn with_settings<F>(settings: ReposePluginSettings, root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            settings,
            root: Mutex::new(Some(Box::new(root))),
        }
    }
}

#[derive(Resource, Clone, Debug)]
pub struct ReposeSettingsRes(pub ReposePluginSettings);

impl Plugin for ReposePlugin {
    fn build(&self, app: &mut App) {
        let settings = self.settings.clone();

        let root = self
            .root
            .lock()
            .unwrap()
            .take()
            .expect("ReposePlugin root already consumed");

        let bridge = ClipboardBridge::default();
        install_clipboard_hooks(bridge.clone());

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
                    window_focus_system,
                ),
            )
            .add_systems(
                Update,
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
