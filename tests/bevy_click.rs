use bevy::prelude::*;
use repose_bevy::bridge::{pending_scope, BevyModifierExt};
use repose_bevy::state::ReposeState;
use repose_core::{RenderContext, Scheduler, View, input::PointerButton};
use repose_core::Vec2 as RVec2;
use repose_ui::{Text, ViewExt};

#[derive(Resource, Default)]
struct Counter(i32);

#[test]
fn on_click_bevy_not_dropped_when_input_outside_compose_scope() {
    let mut state = ReposeState::new_with_world(|_world: &mut World, _sched: &mut Scheduler, _ctx: &RenderContext| {
        repose_ui::Box(
            repose_core::Modifier::new()
                .size(100.0, 100.0)
                .on_click_bevy(|world| {
                    world.resource_mut::<Counter>().0 += 1;
                }),
        )
        .child(Text("Spawn".to_string()))
    });

    let mut world = World::new();
    world.insert_resource(Counter(0));
    state.runtime.set_viewport_and_scale(200, 200, 1.0);
    state.render_ctx = RenderContext::new();

    let pending = std::sync::Arc::clone(&state.pending_bevy_clicks);

    let _out = pending_scope(&pending, || {
        let mut wrapper = |sched: &mut Scheduler, ctx: &RenderContext| {
            (state.root)(&mut world, sched, ctx)
        };
        state.runtime.compose_frame_output(&mut wrapper, &state.render_ctx)
    });
    assert!(
        pending.lock().is_empty(),
        "no click yet"
    );

    let res = state
        .runtime
        .handle_pointer_press(RVec2 { x: 10.0, y: 10.0 }, PointerButton::Primary);
    assert!(res.consumed, "press should be consumed by button");

    let _ = state
        .runtime
        .handle_pointer_release(RVec2 { x: 10.0, y: 10.0 }, PointerButton::Primary);

    let queued = pending.lock().len();
    assert_eq!(
        queued, 1,
        "on_click_bevy must survive input outside compose scope (regression)"
    );

    let cbs = std::mem::take(&mut *pending.lock());
    for cb in cbs {
        cb(&mut world);
    }
    assert_eq!(world.resource::<Counter>().0, 1, "World mutation must happen");
}

#[test]
fn backgroundless_fullscreen_root_does_not_capture_pointer() {
    let mut state = ReposeState::new_with_world(|_world: &mut World, _sched: &mut Scheduler, _ctx: &RenderContext| {
        repose_ui::Box(repose_core::Modifier::new().fill_max_size())
            .child(Text("empty".to_string()))
    });
    state.runtime.set_viewport_and_scale(800, 600, 1.0);
    state.runtime.pointer_inside = true;
    state.render_ctx = RenderContext::new();
    let pending = std::sync::Arc::clone(&state.pending_bevy_clicks);
    let _ = pending_scope(&pending, || {
        let mut w = World::new();
        let mut wrapper = |sched: &mut Scheduler, ctx: &RenderContext| {
            (state.root)(&mut w, sched, ctx)
        };
        state.runtime.compose_frame_output(&mut wrapper, &state.render_ctx)
    });
    let _ = state
        .runtime
        .handle_pointer_move(RVec2 { x: 400.0, y: 300.0 });
    let out = pending_scope(&pending, || {
        let mut w = World::new();
        let mut wrapper = |sched: &mut Scheduler, ctx: &RenderContext| {
            (state.root)(&mut w, sched, ctx)
        };
        state.runtime.compose_frame_output(&mut wrapper, &state.render_ctx)
    });
    assert!(
        !out.wants_pointer,
        "backgroundless root must leave wants_pointer == false over empty space (hover empty)"
    );
    let mut state2 = ReposeState::new_with_world(|_world: &mut World, _sched: &mut Scheduler, _ctx: &RenderContext| {
        repose_ui::Box(
            repose_core::Modifier::new()
                .fill_max_size()
                .background(repose_core::Color::from_rgba(10, 10, 10, 255)),
        )
        .child(Text("opaque".to_string()))
    });
    state2.runtime.set_viewport_and_scale(800, 600, 1.0);
    state2.runtime.pointer_inside = true;
    state2.render_ctx = RenderContext::new();
    let pending2 = std::sync::Arc::clone(&state2.pending_bevy_clicks);
    let _ = pending_scope(&pending2, || {
        let mut w = World::new();
        let mut wrapper = |sched: &mut Scheduler, ctx: &RenderContext| {
            (state2.root)(&mut w, sched, ctx)
        };
        state2.runtime.compose_frame_output(&mut wrapper, &state2.render_ctx)
    });
    let _ = state2
        .runtime
        .handle_pointer_move(RVec2 { x: 400.0, y: 300.0 });
    let out2 = pending_scope(&pending2, || {
        let mut w = World::new();
        let mut wrapper = |sched: &mut Scheduler, ctx: &RenderContext| {
            (state2.root)(&mut w, sched, ctx)
        };
        state2.runtime.compose_frame_output(&mut wrapper, &state2.render_ctx)
    });
    assert!(
        !out2.wants_pointer,
        "opaque non-clickable Box must still be hit-transparent (background != clickable); panel needs explicit on_click to capture"
    );
}

#[test]
fn headless_app_with_core_plugin_steps_click() {
    use bevy::input::ButtonState;
    use bevy::input::mouse::MouseButtonInput;
    use bevy::window::{PrimaryWindow, Window};
    use repose_bevy::plugin::ReposeCorePlugin;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ReposeCorePlugin::new_with_world(
        |_world: &mut World, _sched: &mut Scheduler, _ctx: &RenderContext| {
            repose_ui::Box(
                repose_core::Modifier::new()
                    .size(100.0, 100.0)
                    .on_click_bevy(|world| {
                        world.resource_mut::<Counter>().0 += 10;
                    }),
            )
            .child(Text("headless".to_string()))
        },
    ));
    app.insert_resource(Counter(0));

    let win = app
        .world_mut()
        .spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(200, 200),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();

    app.update();
    assert!(
        app.world().get_non_send::<ReposeState>().is_some(),
        "ReposeState must exist after build"
    );

    app.world_mut()
        .get_mut::<Window>(win)
        .unwrap()
        .set_cursor_position(Some(Vec2::new(10.0, 10.0)));
    app.update(); // pointer_move_system -> runtime.handle_pointer_move(10,10)

    app.world_mut().write_message(MouseButtonInput {
        button: bevy::input::mouse::MouseButton::Left,
        state: ButtonState::Pressed,
        window: win,
    });
    app.update();

    app.world_mut().write_message(MouseButtonInput {
        button: bevy::input::mouse::MouseButton::Left,
        state: ButtonState::Released,
        window: win,
    });
    app.update();

    assert_eq!(
        app.world().resource::<Counter>().0,
        10,
        "Counter == 10 after Bevy chain: mouse_button_system -> handle_pointer_release -> drain"
    );
}
