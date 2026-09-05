use bevy::ecs::system::{SystemParam, SystemState};
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorMoved, Ime, WindowEvent};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    sync::{Mutex, OnceLock},
};

use crate::bridge::bevy_click_drain_system;
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
use crate::state::{ReposeOutput, ReposePendingPanels, ReposeState};
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

/// Game-agnostic core: input, compose, `ReposeFrameRequest`, no RenderApp dependency.
/// Use for headless tests (`MinimalPlugins + ReposeCorePlugin`) or when you
/// provide a custom renderer. For the default overlay, use `ReposePlugin`.
pub struct ReposeCorePlugin {
    pub settings: ReposePluginSettings,
    root: Mutex<Option<crate::state::ReposeRootFn>>,
    fonts: Vec<&'static [u8]>,
}

/// Default overlay plugin: thin wrapper around `ReposeCorePlugin` + `ReposeRenderPlugin`
/// (shared-device/offscreen). For headless / CI, use `ReposeCorePlugin` directly.
pub struct ReposePlugin {
    core: ReposeCorePlugin,
}

fn core_build_logic(
    app: &mut App,
    settings: ReposePluginSettings,
    root: crate::state::ReposeRootFn,
    fonts: Vec<&'static [u8]>,
) {
    for font in &fonts {
        register_font_once(font);
    }
    app.add_message::<MouseButtonInput>();
    app.add_message::<MouseWheel>();
    app.add_message::<KeyboardInput>();
    app.add_message::<CursorMoved>();
    app.add_message::<WindowEvent>();
    app.add_message::<Ime>();
    app.init_resource::<ButtonInput<KeyCode>>();

    let bridge = {
        let tmp = ClipboardBridge::default();
        install_clipboard_hooks(tmp.clone());
        ClipboardBridge(
            crate::platform::GLOBAL_CLIPBOARD
                .get()
                .cloned()
                .unwrap_or(tmp.0),
        )
    };

    app.insert_resource(ReposeSettingsRes(settings.clone()))
        .insert_resource(ReposeOutput::default())
        .insert_resource(crate::state::ReposeFrameRequest::default())
        .insert_resource(bridge)
        .insert_non_send(ReposeState::from_boxed(root));

    if let Some(pending) = app.world_mut().remove_non_send::<crate::state::ReposePendingPanels>() {
        if let Some(mut state) = app.world_mut().get_non_send_mut::<ReposeState>() {
            state.panels.extend(pending.0);
        }
    }
    app.add_systems(
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
                bevy_click_drain_system,
            )
                .chain(),
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

impl ReposeCorePlugin {
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
            settings: ReposePluginSettings::default(),
            root: Mutex::new(Some(Box::new(root))),
            fonts: Vec::new(),
        }
    }
    pub fn with_settings<F>(settings: ReposePluginSettings, root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        let mut root = root;
        Self::with_settings_world(settings, move |_world, s, c| root(s, c))
    }
    pub fn with_settings_world<F>(settings: ReposePluginSettings, root: F) -> Self
    where
        F: FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static,
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
    pub fn from_system<Q, F>(func: F) -> Self
    where
        Q: SystemParam + 'static,
        F: for<'w, 's> FnMut(Q::Item<'w, 's>, &mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        let mut func = func;
        let mut state: Option<SystemState<Q>> = None;
        Self::new_with_world(move |world, sched, ctx| {
            if state.is_none() {
                state = Some(SystemState::new(world));
            }
            let st = state.as_mut().unwrap();
            let param = st.get_mut(world).unwrap();
            let view = func(param, sched, ctx);
            st.apply(world);
            view
        })
    }
    pub fn from_system_with_settings<Q, F>(settings: ReposePluginSettings, func: F) -> Self
    where
        Q: SystemParam + 'static,
        F: for<'w, 's> FnMut(Q::Item<'w, 's>, &mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        let mut func = func;
        let mut state: Option<SystemState<Q>> = None;
        Self::with_settings_world(settings, move |world, sched, ctx| {
            if state.is_none() {
                state = Some(SystemState::new(world));
            }
            let st = state.as_mut().unwrap();
            let param = st.get_mut(world).unwrap();
            let view = func(param, sched, ctx);
            st.apply(world);
            view
        })
    }
}

impl ReposePlugin {
    pub fn new<F>(root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            core: ReposeCorePlugin::new(root),
        }
    }

    pub fn new_with_world<F>(root: F) -> Self
    where
        F: FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            core: ReposeCorePlugin::new_with_world(root),
        }
    }

    pub fn with_settings<F>(settings: ReposePluginSettings, root: F) -> Self
    where
        F: FnMut(&mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            core: ReposeCorePlugin::with_settings(settings, root),
        }
    }

    pub fn with_settings_world<F>(settings: ReposePluginSettings, root: F) -> Self
    where
        F: FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        Self {
            core: ReposeCorePlugin::with_settings_world(settings, root),
        }
    }

    pub fn with_font_bytes(mut self, bytes: &'static [u8]) -> Self {
        self.core = self.core.with_font_bytes(bytes);
        self
    }

    pub fn from_system<Q, F>(func: F) -> Self
    where
        Q: SystemParam + 'static,
        F: for<'w, 's> FnMut(Q::Item<'w, 's>, &mut Scheduler, &RenderContext) -> View
            + Send
            + 'static,
    {
        Self {
            core: ReposeCorePlugin::from_system(func),
        }
    }

    pub fn from_system_with_settings<Q, F>(settings: ReposePluginSettings, func: F) -> Self
    where
        Q: SystemParam + 'static,
        F: for<'w, 's> FnMut(Q::Item<'w, 's>, &mut Scheduler, &RenderContext) -> View
            + Send
            + 'static,
    {
        Self {
            core: ReposeCorePlugin::from_system_with_settings(settings, func),
        }
    }
}

pub fn bevy_system<Q, F>(mut func: F) -> impl FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static
where
    Q: SystemParam + 'static,
    F: for<'w, 's> FnMut(Q::Item<'w, 's>, &mut Scheduler, &RenderContext) -> View + Send + 'static,
{
    let mut state: Option<SystemState<Q>> = None;
    move |world, sched, ctx| {
        if state.is_none() {
            state = Some(SystemState::new(world));
        }
        let st = state.as_mut().unwrap();
        let param = st.get_mut(world).unwrap();
        let view = func(param, sched, ctx);
        st.apply(world);
        view
    }
}

pub trait ReposeAppExt {
    fn add_repose_panel<F>(&mut self, panel: F) -> &mut Self
    where
        F: FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static;

    fn add_repose_system<Q, F>(&mut self, system: F) -> &mut Self
    where
        Q: SystemParam + 'static,
        F: for<'w, 's> FnMut(Q::Item<'w, 's>, &mut Scheduler, &RenderContext) -> View
            + Send
            + 'static;
}

impl ReposeAppExt for App {
    fn add_repose_panel<F>(&mut self, panel: F) -> &mut Self
    where
        F: FnMut(&mut World, &mut Scheduler, &RenderContext) -> View + Send + 'static,
    {
        if let Some(mut state) = self.world_mut().get_non_send_mut::<ReposeState>() {
            state.panels.push(Box::new(panel));
        } else if let Some(mut pending) =
            self.world_mut().get_non_send_mut::<ReposePendingPanels>()
        {
            pending.0.push(Box::new(panel));
        } else {
            self.world_mut()
                .insert_non_send(ReposePendingPanels(vec![Box::new(panel)]));
        }
        self
    }

    fn add_repose_system<Q, F>(&mut self, system: F) -> &mut Self
    where
        Q: SystemParam + 'static,
        F: for<'w, 's> FnMut(Q::Item<'w, 's>, &mut Scheduler, &RenderContext) -> View
            + Send
            + 'static,
    {
        self.add_repose_panel(bevy_system(system))
    }
}

#[derive(Resource, Clone, Debug)]
pub struct ReposeSettingsRes(pub ReposePluginSettings);

impl Plugin for ReposeCorePlugin {
    fn build(&self, app: &mut App) {
        let settings = self.settings.clone();
        let root = self
            .root
            .lock()
            .unwrap()
            .take()
            .expect("ReposeCorePlugin root already consumed");
        let fonts = self.fonts.clone();
        core_build_logic(app, settings, root, fonts);
    }
}

impl Plugin for ReposePlugin {
    fn build(&self, app: &mut App) {
        let settings = self.core.settings.clone();
        let root = self
            .core
            .root
            .lock()
            .unwrap()
            .take()
            .expect("ReposePlugin root already consumed");
        let fonts = self.core.fonts.clone();
        core_build_logic(app, settings.clone(), root, fonts);
        app.add_plugins(ReposeRenderPlugin {
            settings: settings.clone(),
        });
    }
}
