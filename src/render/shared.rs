use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue};
use bevy::render::texture::GpuImage;
use bevy::render::{ExtractSchedule, RenderApp};
use bevy::ui_render::ui_material::MaterialNode;

use parking_lot::Mutex;
use repose_core::RenderCommand;
use repose_render_wgpu::WgpuSceneRenderer;

use super::apply_render_commands;
use super::ReposeOverlayMaterial;
use crate::plugin::{ReposePluginSettings, ReposeSettingsRes};
use crate::state::{ReposeState, ReposeUiImage};

pub struct SharedDevicePlugin {
    pub settings: ReposePluginSettings,
}

#[derive(Resource, Clone)]
struct SharedSettings {
    clear_alpha: f32,
    msaa_samples: u32,
}

#[derive(Resource, Clone)]
struct ReposeExtractedFrame {
    scene: repose_core::Scene,
    cmds: Arc<std::sync::Mutex<Vec<RenderCommand>>>,
    width: u32,
    height: u32,
    clear_alpha: f32,
    image: Handle<Image>,
    frame_gen: u64,
}

impl ExtractResource<RenderApp> for ReposeExtractedFrame {
    type Source = ReposeExtractedFrame;
    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

struct PrivateTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

#[derive(Resource)]
struct SharedGpu {
    renderer: Mutex<WgpuSceneRenderer>,
    target: Mutex<Option<PrivateTarget>>,
    blitter: Mutex<Option<wgpu::util::TextureBlitter>>,
}

fn make_overlay_image(w: u32, h: u32) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::linear();
    image
}

impl Plugin for SharedDevicePlugin {
    fn build(&self, app: &mut App) {
        let settings = SharedSettings {
            clear_alpha: self.settings.clear_alpha,
            msaa_samples: self.settings.msaa_samples.max(1),
        };

        app.insert_resource(settings.clone())
            .insert_resource(ReposeExtractedFrame {
                scene: repose_core::Scene::default(),
                cmds: Arc::new(std::sync::Mutex::new(Vec::new())),
                width: 1,
                height: 1,
                clear_alpha: settings.clear_alpha,
                image: Handle::default(),
                frame_gen: 0,
            })
            .add_plugins(ExtractResourcePlugin::<ReposeExtractedFrame>::default())
            .add_systems(Startup, setup_overlay)
            .add_systems(PostUpdate, prepare_extract_frame);

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(settings)
            .add_systems(ExtractSchedule, init_shared_renderer)
            .add_systems(
                RenderGraph,
                render_shared_system.in_set(RenderGraphSystems::Begin),
            );
    }
}

fn setup_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ReposeOverlayMaterial>>,
    settings: Res<ReposeSettingsRes>,
    mut frame: ResMut<ReposeExtractedFrame>,
) {
    let handle = images.add(make_overlay_image(1, 1));
    let mat = materials.add(ReposeOverlayMaterial {
        texture: handle.clone(),
    });
    commands.insert_resource(ReposeUiImage {
        image: handle.clone(),
        width: 1,
        height: 1,
    });
    frame.image = handle;

    if settings.0.overlay {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                bevy::ui::FocusPolicy::Pass,
                bevy::ui::ZIndex(i32::MAX),
                Name::new("ReposeOverlay"),
            ))
            .with_children(|p| {
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    bevy::ui::FocusPolicy::Pass,
                    MaterialNode::<ReposeOverlayMaterial>(mat),
                ));
            });
    }
}

fn prepare_extract_frame(
    mut commands: Commands,
    state: NonSendMut<ReposeState>,
    settings: Res<SharedSettings>,
    mut frame: ResMut<ReposeExtractedFrame>,
    ui_image: Res<ReposeUiImage>,
    mut images: ResMut<Assets<Image>>,
) {
    let w = state.fb_width.max(1);
    let h = state.fb_height.max(1);

    if ui_image.width != w || ui_image.height != h {
        if let Some(mut img) = images.get_mut(&ui_image.image) {
            *img = make_overlay_image(w, h);
        }
        commands.insert_resource(ReposeUiImage {
            image: ui_image.image.clone(),
            width: w,
            height: h,
        });
    }

    let next_gen = frame.frame_gen.wrapping_add(1);
    info!(
        "[repose] extract gen={next_gen} scene_nodes={} size={w}x{h}",
        state.scene.nodes.len()
    );

    frame.scene = state.scene.clone();
    *frame.cmds.lock().unwrap() = state.render_ctx.drain();
    frame.width = w;
    frame.height = h;
    frame.clear_alpha = settings.clear_alpha;
    frame.image = ui_image.image.clone();
    frame.frame_gen = next_gen;
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

    let blitter = wgpu::util::TextureBlitter::new(
        &device.wgpu_device(),
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );

    info!("repose-bevy shared-device: bound WgpuSceneRenderer to Bevy device");
    commands.insert_resource(SharedGpu {
        renderer: Mutex::new(renderer),
        target: Mutex::new(None),
        blitter: Mutex::new(Some(blitter)),
    });
}

fn render_shared_system(
    mut render_context: RenderContext,
    frame: Option<Res<ReposeExtractedFrame>>,
    gpu: Option<Res<SharedGpu>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    let Some(frame) = frame else {
        return;
    };
    let Some(gpu) = gpu else {
        return;
    };
    let Some(gpu_image) = gpu_images.get(&frame.image) else {
        return;
    };

    let w = frame.width.max(1);
    let h = frame.height.max(1);

    let mut renderer = gpu.renderer.lock();
    let mut slot = gpu.target.lock();
    let device = renderer.device.clone();

    let recreate = slot.as_ref().map_or(true, |t| t.width != w || t.height != h);
    if recreate {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("repose-private-rt"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        *slot = Some(PrivateTarget {
            texture,
            view,
            width: w,
            height: h,
        });
    }
    let target = slot.as_ref().unwrap();

    let cmds = frame.cmds.lock().unwrap().drain(..).collect::<Vec<_>>();
    apply_render_commands(&mut renderer, cmds);
    if renderer.output_width != w || renderer.output_height != h {
        renderer.resize(w, h);
    }

    let encoder = render_context.command_encoder();
    let clear = Some([0.0, 0.0, 0.0, frame.clear_alpha as f64]);

    renderer.render_scene_to_encoder(&frame.scene, encoder, &target.view, clear);

    let blitter = gpu.blitter.lock();
    if let Some(blitter) = blitter.as_ref() {
        let gval = frame.frame_gen;
        info!("[repose] blit gen={gval} {w}x{h} private_rt -> ui_image");
        blitter.copy(&device, encoder, &target.view, &gpu_image.texture_view);
        info!("[repose] blit done");
    }
}
