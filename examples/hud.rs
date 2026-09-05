use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use repose_bevy::{ReposePlugin, ReposePluginSettings};
use repose_core::prelude::{Color as ReposeColor, Modifier};
use repose_ui::{Box, Row, Spacer, Text, ViewExt};

#[derive(Resource)]
struct SharedState {
    inner: Arc<Mutex<SharedInner>>,
    heal_requested: Arc<AtomicBool>,
}

struct SharedInner {
    hp: i32,
    score: i32,
}

fn main() {
    let shared = Arc::new(Mutex::new(SharedInner { hp: 100, score: 0 }));
    let shared_ui = shared.clone();
    let heal_flag = Arc::new(AtomicBool::new(false));
    let heal_flag_ui = heal_flag.clone();
    let heal_flag_sys = heal_flag.clone();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "repose-bevy HUD".into(),
                resolution: (900, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ReposePlugin::with_settings(
            ReposePluginSettings {
                clear_alpha: 0.0,
                compose_every_frame: true,
                msaa_samples: 1,
                overlay: true,
                ..Default::default()
            },
            move |_s, _c| hud_ui(&shared_ui, &heal_flag_ui),
        ))
        .insert_resource(SharedState {
            inner: shared,
            heal_requested: heal_flag_sys,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, tick_game)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn hud_ui(
    shared: &Arc<Mutex<SharedInner>>,
    heal_flag: &Arc<AtomicBool>,
) -> repose_core::prelude::View {
    let state = shared.lock().unwrap();
    let heal = heal_flag.clone();

    Row(Modifier::new()
        .padding(16.0)
        .gap(24.0)
        .fill_max_width()
        .background(ReposeColor::from_rgba(20, 20, 30, 200))
        .clip_rounded(8.0))
    .child((
        Text(format!("HP: {: >3}", state.hp)),
        Spacer(),
        Text(format!("Score: {: >5}", state.score)),
        Spacer(),
        Box(Modifier::new()
            .padding(12.0)
            .background(ReposeColor::from_rgba(60, 60, 80, 255))
            .clip_rounded(4.0)
            .on_click(move || heal.store(true, Ordering::Relaxed)))
        .child(Text("+10 HP".to_string())),
    ))
}

fn tick_game(
    time: Res<Time>,
    shared: Res<SharedState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut last_hp_tick: Local<f64>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let mut state = shared.inner.lock().unwrap();
        state.score += 10;
    }

    if shared.heal_requested.swap(false, Ordering::Relaxed) {
        let mut state = shared.inner.lock().unwrap();
        state.hp = (state.hp + 10).min(100);
    }

    let now = time.elapsed_secs_f64();
    if *last_hp_tick == 0.0 {
        *last_hp_tick = now;
    }
    if now - *last_hp_tick >= 5.0 {
        let mut state = shared.inner.lock().unwrap();
        state.hp = (state.hp - 1).max(0);
        *last_hp_tick = now;
    }
}
