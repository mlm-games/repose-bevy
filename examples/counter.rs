use bevy::prelude::*;
use repose_bevy::ReposePlugin;
use repose_core::prelude::{remember_state, Color as ReposeColor, Modifier};
use repose_ui::{Column, Text, ViewExt};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "repose-bevy counter".into(),
                resolution: (900, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ReposePlugin::new(|_sched, _ctx| counter_ui()))
        .add_systems(Startup, setup_cam)
        .run();
}

fn setup_cam(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn counter_ui() -> repose_core::prelude::View {
    let count = remember_state(|| 0i32);
    let inc = count.clone();
    let reset = count.clone();

    Column(
        Modifier::new()
            .padding(32.0)
            .gap(16.0)
            .background(ReposeColor::from_rgba(31, 31, 41, 235))
            .clip_rounded(12.0)
            .fill_max_size(),
    )
    .child((
        Text("Repose inside Bevy".to_string()),
        Text(format!("Count: {}", *count.borrow())),
        repose_ui::Box(
            Modifier::new().on_click(move || {
                *inc.borrow_mut() += 1;
            }),
        )
        .child(Text("Increment".to_string())),
        repose_ui::Box(
            Modifier::new().on_click(move || {
                *reset.borrow_mut() = 0;
            }),
        )
        .child(Text("Reset".to_string())),
    ))
}
