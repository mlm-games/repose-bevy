# repose-bevy

Embed [Repose](https://github.com/mlm-games/repose) (Compose-like Rust UI) inside [Bevy](https://bevy.org).

Targets Bevy **main branch** (0.20-dev, wgpu 30) 

## Features

- Declarative Repose UI (`Column`, `Button`, signals, `remember_state`, ...) as a Bevy plugin
- Input bridging (pointer, scroll, keyboard, IME)
- IME support (enable/disable on text fields, cursor area, composition events)
- Clipboard bridging (copy/paste between Repose and system clipboard)
- **shared-device** (default): render into Bevy's UI Image using Bevy's wgpu device (avoids CPU readback)
- **offscreen** (optional): Repose `WgpuSceneRenderer` on separate wgpu device -> RGBA texture -> Bevy `Image` overlay

## Quick start

```toml
[dependencies]
repose-bevy = "0.2"
bevy = { git = "https://github.com/bevyengine/bevy", rev = "fb89a8649d9b359e53ffb6e5492ebb7c059ac8af" }
```

```rust
use bevy::prelude::*;
use repose_bevy::prelude::*;
use repose_core::{Modifier, remember_state};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ReposePlugin::new(|_sched, _ctx| {
            let count = remember_state(|| 0i32);
            repose_ui::Column(Modifier::new().padding(24.0).gap(12.0)).child((
                repose_ui::Text(format!("Count: {}", *count.borrow())).size(22.0),
                repose_ui::Box(
                    Modifier::new()
                        .background(repose_core::Color::from_rgba(60, 60, 60, 255))
                        .padding(8.0)
                        .on_click({
                            let count = count.clone();
                            move || *count.borrow_mut() += 1
                        }),
                )
                .child(repose_ui::Text("Increment".to_string())),
            ))
        }))
        .run();
}
```

> `offscreen` requires `default-features = false`: `repose-bevy = { version = "0.2", default-features = false, features = ["offscreen"] }` — otherwise `shared-device` (default) wins.

## Architecture

```
Bevy main world
  |- input systems  ->  ReposeRuntime (NonSend)
  |- compose system ->  Scene + hit cache
  |- platform       ->  IME window config + clipboard bridging
  |- render extract ->  Scene + commands (Arc<Mutex<>>)

Bevy render world
  |- init_shared_renderer (once)  ->  WgpuSceneRenderer on Bevy device
  |- render_shared_system         ->  render_scene_to_encoder into UI Image
```

`ReposeRuntime` is **not Send** (`Rc`/`RefCell`). It lives in a `NonSendMut` resource.

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `shared-device` | yes | Bevy's `RenderDevice` to direct GPU render into UI `Image` |
| `offscreen` | no | Separate wgpu device + staging buffer -> CPU readback -> Bevy `Image` |
