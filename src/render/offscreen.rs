use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat as BevyTexFormat,
};
use parking_lot::Mutex;
use std::sync::Arc;

use crate::compose::compose_repose_system;
use crate::plugin::ReposePluginSettings;
use crate::state::{ReposeState, ReposeUiImage};

pub struct OffscreenRenderPlugin {
    pub settings: ReposePluginSettings,
}

#[derive(Resource)]
struct OffscreenGpu {
    inner: Arc<Mutex<OffscreenInner>>,
}

struct OffscreenInner {
    offscreen: repose_render_wgpu::offscreen::OffscreenRenderer,
    clear_alpha: f32,
    sampler: bevy::image::ImageSampler,
}

impl Plugin for OffscreenRenderPlugin {
    fn build(&self, app: &mut App) {
        let msaa = self.settings.msaa_samples;
        let clear_alpha = self.settings.clear_alpha;
        let sampler = self.settings.sampler.clone();

        app.add_systems(Startup, setup_overlay).add_systems(
            PostUpdate,
            render_offscreen_system.after(compose_repose_system),
        );

        app.insert_resource(OffscreenGpu {
            inner: Arc::new(Mutex::new(OffscreenInner {
                offscreen: create_renderer(msaa),
                clear_alpha,
                sampler,
            })),
        });
    }
}

fn create_renderer(msaa: u32) -> repose_render_wgpu::offscreen::OffscreenRenderer {
    repose_render_wgpu::offscreen::OffscreenRenderer::new_blocking(1, 1, msaa)
        .expect("repose-bevy: offscreen device")
}

fn setup_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    settings: Res<crate::plugin::ReposeSettingsRes>,
) {
    let usage = bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
        | bevy::render::render_resource::TextureUsages::COPY_DST;
    let image = super::overlay_image(1, 1, usage, settings.0.sampler.clone());
    let handle = images.add(image);
    commands.insert_resource(ReposeUiImage {
        image: handle.clone(),
        width: 1,
        height: 1,
    });
    if settings.0.overlay {
        super::spawn_overlay(&mut commands, handle);
    }
}

fn render_offscreen_system(
    gpu: Res<OffscreenGpu>,
    state: NonSendMut<ReposeState>,
    mut images: ResMut<Assets<Image>>,
    mut ui_image: ResMut<ReposeUiImage>,
) {
    let w = state.fb_width.max(1);
    let h = state.fb_height.max(1);

    let cmds = state.render_ctx.drain();
    let scene = state.scene.clone();

    let mut inner = gpu.inner.lock();
    repose_render_wgpu::apply_render_commands(inner.offscreen.renderer_mut(), cmds);
    let _ = inner.offscreen.ensure_size(w, h);

    let clear = Some([0.0, 0.0, 0.0, inner.clear_alpha as f64]);
    let pixels = match inner.offscreen.render_rgba(&scene, clear) {
        Ok(p) => p,
        Err(_) => return,
    };

    if ui_image.width != w || ui_image.height != h {
        let mut image = Image::new_fill(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &pixels,
            BevyTexFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage =
            bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
                | bevy::render::render_resource::TextureUsages::COPY_DST;
        image.sampler = inner.sampler.clone();
        if let Some(mut img) = images.get_mut(&ui_image.image) {
            *img = image;
        }
        ui_image.width = w;
        ui_image.height = h;
    } else if let Some(mut img) = images.get_mut(&ui_image.image) {
        if let Some(data) = img.data.as_mut() {
            data.copy_from_slice(&pixels);
        } else {
            img.data = Some(pixels);
        }
    }
}
