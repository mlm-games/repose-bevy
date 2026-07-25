# repose-bevy

Embed [Repose](https://github.com/mlm-games/repose) (Compose-like Rust UI) inside [Bevy](https://bevy.org).

Targets Bevy **main branch** (0.20-dev, wgpu 30) 

## Features

- Declarative Repose UI (`Column`, `Button`, signals, `remember_state`, ...) as a Bevy plugin
- Input bridging (pointer, scroll, keyboard)
- **offscreen** (default): Repose `WgpuSceneRenderer` on separate wgpu device -> RGBA texture -> Bevy `Image` overlay
- **shared-device** (experimental, incomplete): render into Bevy's graph using Bevy's wgpu device (avoids CPU readback)

## Quick start

```toml
[dependencies]
repose-bevy = "0"
bevy = { git = "https://github.com/bevyengine/bevy" }
```

```rust
use bevy::prelude::*;
use repose_bevy::prelude::*;
use repose_core::prelude::*;
use repose_ui::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ReposePlugin::new(|_sched, _ctx| {
            let count = remember_state(|| 0i32);
            Column(modifier::new().padding(24.0).gap(12.0)).child((
                Text(format!("Count: {}", *count.borrow())).size(22.0),
                Button("Increment", {
                    let count = count.clone();
                    move || *count.borrow_mut() += 1
                }),
            ))
        }))
        .run();
}
```

## Architecture

```
Bevy main world
  |- input systems  ->  ReposeRuntime (NonSend)
  |- compose system ->  Scene + hit cache
  |- render path
        offscreen: WgpuSceneRenderer -> staging -> Image asset -> Node/ImageNode
        shared:    extract Scene -> render graph node -> render_scene_to_encoder
```

`ReposeRuntime` is **not Send** (`Rc`/`RefCell`). It lives in a `NonSendMut` resource.
