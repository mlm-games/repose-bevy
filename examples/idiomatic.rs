use bevy::prelude::*;
use repose_bevy::prelude::*;
use repose_core::prelude::{Modifier, remember_state};
use repose_ui::{Column, Text, ViewExt};

#[derive(Component, Clone)]
struct Building {
    name: String,
}

#[derive(Resource, Default, Clone)]
struct Score(pub i32);

#[derive(Message, Clone)]
struct SpawnBuilding(pub String);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "repose-bevy idiomatic".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ReposePlugin::from_system_with_settings::<
            (Query<&Building>, Res<Score>),
            _,
        >(
            ReposePluginSettings {
                compose_every_frame: false,
                ..Default::default()
            },
            idiomatic_ui,
        ))
        .add_repose_panel(|_world, _sched, _ctx| {
            repose_ui::Text("Panel 2".to_string())
        })
        .insert_resource(Score(0))
        .add_message::<SpawnBuilding>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (handle_spawns, camera_zoom_gated, request_on_change).chain(),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn(Building {
        name: "Town Hall".into(),
    });
}

fn idiomatic_ui(
    (query, score): (Query<&Building>, Res<Score>),
    _sched: &mut Scheduler,
    _ctx: &RenderContext,
) -> View {
    let count = query.iter().count();
    let local = remember_state(|| 0i32);
    let local_clone = local.clone();

    Column(Modifier::new().padding(16.0).gap(12.0)).child((
        Text(format!("Buildings: {count}")),
        Text(format!("Score: {}", score.0)),
        Text(format!("Local clicks: {}", *local.borrow())),
        repose_ui::Box(
            Modifier::new()
                .padding(12.0)
                .background(repose_core::Color::from_rgba(60, 60, 80, 255))
                .clip_rounded(8.0)
                .on_click_bevy(|world| {
                    let n = {
                        let mut q = world.query::<&Building>();
                        q.iter(world).count()
                    };
                    world.spawn(Building {
                        name: format!("B{}", n + 1),
                    });
                }),
        )
        .child(Text("Spawn via World".to_string())),
        repose_ui::Box(
            Modifier::new()
                .padding(12.0)
                .background(repose_core::Color::from_rgba(80, 60, 60, 255))
                .clip_rounded(8.0)
                .on_click_bevy(|world| {
                    world
                        .resource_mut::<Messages<SpawnBuilding>>()
                        .write(SpawnBuilding("via event".into()));
                }),
        )
        .child(Text("Spawn via Event".to_string())),
        repose_ui::Box(
            Modifier::new()
                .padding(12.0)
                .background(repose_core::Color::from_rgba(60, 80, 100, 255))
                .clip_rounded(8.0)
                .on_click(move || *local_clone.borrow_mut() += 1),
        )
        .child(Text("Local state".to_string())),
    ))
}

fn handle_spawns(mut reader: MessageReader<SpawnBuilding>, mut commands: Commands, mut score: ResMut<Score>) {
    for ev in reader.read() {
        commands.spawn(Building { name: ev.0.clone() });
        score.0 += 10;
    }
}

fn camera_zoom_gated(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    output: Res<ReposeOutput>,
    mut q: Query<&mut Transform, With<Camera2d>>,
) {
    if output.scroll_consumed || output.wants_pointer {
        for _ in wheel.read() {}
        return;
    }
    for ev in wheel.read() {
        for mut t in &mut q {
            let s = if ev.y > 0.0 { 0.9 } else { 1.1 };
            t.scale *= s;
        }
    }
}

fn request_on_change(
    score: Res<Score>,
    added: Query<Entity, Added<Building>>,
    req: Res<ReposeFrameRequest>,
) {
    if score.is_changed() || !added.is_empty() {
        req.request();
    }
}
