use bevy::prelude::*;
use repose_bevy::prelude::*;
use repose_core::prelude::Modifier;
use repose_core::Vec2 as RVec2;
use repose_ui::{Column, Text, ViewExt};

#[derive(Component)]
struct Building {
    pos: RVec2,
}

#[derive(Resource, Default)]
struct SelectedEntity(Option<Entity>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "world_inspector".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ReposePlugin::from_system::<
            (Query<(Entity, &Building)>, Res<SelectedEntity>),
            _,
        >(inspector_ui))
        .insert_resource(SelectedEntity::default())
        .add_systems(Startup, setup)
        .add_systems(Update, pick_building)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    for i in 0..5 {
        let pos = RVec2 {
            x: i as f32 * 40.0 - 80.0,
            y: 0.0,
        };
        commands.spawn((
            Building { pos },
            Sprite {
                color: bevy::prelude::Color::srgb(0.6, 0.4, 0.2),
                custom_size: Some(bevy::math::Vec2::new(20.0, 20.0)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, 0.0),
        ));
    }
}

fn inspector_ui(
    (q, selected): (Query<(Entity, &Building)>, Res<SelectedEntity>),
    _sched: &mut Scheduler,
    _ctx: &RenderContext,
) -> View {
    let items: Vec<View> = q
        .iter()
        .map(|(e, b)| {
            let is_sel = selected.0 == Some(e);
            let bg = if is_sel {
                repose_core::Color::from_rgba(80, 120, 80, 255)
            } else {
                repose_core::Color::from_rgba(60, 60, 60, 255)
            };
            repose_ui::Box(
                Modifier::new()
                    .padding(8.0)
                    .background(bg)
                    .on_click_bevy(move |world| {
                        world.resource_mut::<SelectedEntity>().0 = Some(e);
                    }),
            )
            .child(Text(format!("Building {:?} @ {:?}", e, b.pos)))
        })
        .collect();

    Column(Modifier::new().padding(12.0).gap(8.0)).child((
        Text(format!("Buildings: {}", q.iter().count())),
        Text(format!("Selected: {:?}", selected.0)),
        Column(Modifier::new().gap(4.0)).child(items),
    ))
}

fn pick_building(
    mut events: MessageReader<bevy::input::mouse::MouseButtonInput>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    output: Res<ReposeOutput>,
    mut selected: ResMut<SelectedEntity>,
    q: Query<(Entity, &Building)>,
) {
    if output.wants_pointer() {
        for _ in events.read() {}
        return;
    }
    for ev in events.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        let Ok(window) = windows.single() else {
            continue;
        };
        let Some(cursor_pos) = window.cursor_position() else {
            continue;
        };
        let Ok((camera, cam_tf)) = cameras.single() else {
            continue;
        };
        let Ok(world_pos) = camera.viewport_to_world(cam_tf, cursor_pos) else {
            continue;
        };
        let p = world_pos.origin.truncate();
        let mut best: Option<(Entity, f32)> = None;
        for (e, b) in &q {
            let dx = b.pos.x - p.x;
            let dy = b.pos.y - p.y;
            let d2 = dx * dx + dy * dy;
            if d2 < 1600.0 {
                if best.is_none_or(|(_, bd)| d2 < bd) {
                    best = Some((e, d2));
                }
            }
        }
        if let Some((e, _)) = best {
            selected.0 = Some(e);
        }
    }
}
