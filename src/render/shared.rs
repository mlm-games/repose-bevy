use bevy::prelude::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use parking_lot::Mutex;
use repose_render_wgpu::WgpuSceneRenderer;

use crate::plugin::ReposePluginSettings;

/// Experimental: render Repose using Bevy's wgpu device/queue.
///
/// No CPU readback - renders directly into the Bevy UI `Image` via the GPU.
///
/// **Status:** Incomplete. Needs render-graph integration for proper extract ->
/// prepare -> render scheduling. This skeleton shows the plumbing but does not
/// yet produce visible output - the offscreen path is the recommended default.
pub struct SharedDevicePlugin {
    pub settings: ReposePluginSettings,
}

#[derive(Resource)]
struct SharedSettings {
    clear_alpha: f32,
    msaa_samples: u32,
}

#[derive(Resource)]
struct SharedGpu {
    renderer: Mutex<WgpuSceneRenderer>,
}

impl Plugin for SharedDevicePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SharedSettings {
            clear_alpha: self.settings.clear_alpha,
            msaa_samples: self.settings.msaa_samples.max(1),
        })
        .add_systems(Startup, init_shared_renderer);

        bevy::log::warn!(
            "repose-bevy shared-device: experimental path - \
             renders but is not yet wired into the Bevy render graph. \
             Use the default `offscreen` feature."
        );
    }
}

fn init_shared_renderer(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    settings: Res<SharedSettings>,
    existing: Option<Res<SharedGpu>>,
) {
    if existing.is_some() {
        return;
    }

    let wgpu_queue: wgpu::Queue = (**queue.0).clone();
    let renderer = WgpuSceneRenderer::from_device(
        device.wgpu_device().clone(),
        wgpu_queue,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        settings.msaa_samples,
    );

    info!("repose-bevy shared-device: bound WgpuSceneRenderer to Bevy device");
    commands.insert_resource(SharedGpu {
        renderer: Mutex::new(renderer),
    });
}
