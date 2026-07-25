use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat as BevyTexFormat};
use bevy::ui::{FocusPolicy, Node, ZIndex};
use bevy::ui::widget::{ImageNode, NodeImageMode};
use parking_lot::Mutex;
use repose_core::RenderCommand;
use repose_render_wgpu::WgpuSceneRenderer;
use std::sync::Arc;

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
    renderer: WgpuSceneRenderer,
    target: Option<TargetTex>,
    clear_alpha: f32,
}

struct TargetTex {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    staging: wgpu::Buffer,
    bytes_per_row: u32,
}

impl Plugin for OffscreenRenderPlugin {
    fn build(&self, app: &mut App) {
        let msaa = self.settings.msaa_samples;
        let clear_alpha = self.settings.clear_alpha;

        app.add_systems(Startup, setup_overlay)
            .add_systems(PostUpdate, render_offscreen_system);

        app.insert_resource(OffscreenGpu {
            inner: Arc::new(Mutex::new(OffscreenInner {
                renderer: create_renderer(msaa),
                target: None,
                clear_alpha,
            })),
        });
    }
}

fn create_renderer(msaa: u32) -> WgpuSceneRenderer {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        },
    ))
    .expect("repose-bevy: no wgpu adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("repose-bevy-offscreen"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            experimental_features: wgpu::ExperimentalFeatures::default(),
            trace: wgpu::Trace::Off,
        },
    ))
    .expect("repose-bevy: device");

    WgpuSceneRenderer::from_device(device, queue, wgpu::TextureFormat::Rgba8UnormSrgb, msaa.max(1))
}

fn setup_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    settings: Res<crate::plugin::ReposeSettingsRes>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        BevyTexFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
        | bevy::render::render_resource::TextureUsages::COPY_DST;

    let handle = images.add(image);
    commands.insert_resource(ReposeUiImage {
        image: handle.clone(),
        width: 1,
        height: 1,
    });

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
                FocusPolicy::Pass,
                ZIndex(i32::MAX),
                Name::new("ReposeOverlay"),
            ))
            .with_children(|p| {
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode {
                        image: handle,
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ));
            });
    }
}

fn ensure_target(inner: &mut OffscreenInner, w: u32, h: u32) {
    let w = w.max(1);
    let h = h.max(1);
    if let Some(t) = &inner.target {
        if t.width == w && t.height == h {
            return;
        }
    }

    inner.renderer.resize(w, h);

    let texture = inner.renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("repose-bevy-target"),
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let unpadded = w * 4;
    let bytes_per_row = (unpadded + align - 1) / align * align;
    let staging_size = (bytes_per_row * h) as u64;

    let staging = inner.renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("repose-bevy-staging"),
        size: staging_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    inner.target = Some(TargetTex {
        texture,
        view,
        width: w,
        height: h,
        staging,
        bytes_per_row,
    });
}

fn apply_render_commands(renderer: &mut WgpuSceneRenderer, cmds: Vec<RenderCommand>) {
    for cmd in cmds {
        match cmd {
            RenderCommand::SetImageEncoded { handle, bytes, srgb } => {
                if let Err(e) = renderer.set_image_from_bytes(handle, &bytes, srgb) {
                    bevy::log::warn!("repose-bevy: SetImageEncoded({handle}): {e:#}");
                }
            }
            RenderCommand::SetImageRgba8 {
                handle,
                w,
                h,
                rgba,
                srgb,
            } => {
                if let Err(e) = renderer.set_image_rgba8(handle, w, h, &rgba, srgb) {
                    bevy::log::warn!("repose-bevy: SetImageRgba8({handle}): {e:#}");
                }
            }
            RenderCommand::SetImageNv12 {
                handle,
                w,
                h,
                y,
                uv,
                color_info,
            } => {
                if let Err(e) = renderer.set_image_nv12(handle, w, h, &y, &uv, color_info) {
                    bevy::log::warn!("repose-bevy: SetImageNv12({handle}): {e:#}");
                }
            }
            RenderCommand::SetImagePlanes {
                handle,
                w,
                h,
                pixel_format,
                planes,
                color_info,
            } => {
                if let Err(e) =
                    renderer.set_image_planes(handle, w, h, pixel_format, &planes, color_info)
                {
                    bevy::log::warn!("repose-bevy: SetImagePlanes({handle}): {e:#}");
                }
            }
            RenderCommand::RemoveImage { handle } => {
                renderer.remove_image(handle);
            }
        }
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
    apply_render_commands(&mut inner.renderer, cmds);
    ensure_target(&mut inner, w, h);

    let mut encoder = inner
        .renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("repose-bevy-enc"),
        });

    let clear = Some([0.0, 0.0, 0.0, inner.clear_alpha as f64]);

    // Use raw pointers to avoid borrow conflicts with MutexGuard
    let (view_ptr, tex_ptr, staging_ptr, bpr) = {
        let t = inner.target.as_ref().unwrap();
        (
            &t.view as *const wgpu::TextureView,
            &t.texture as *const wgpu::Texture,
            &t.staging as *const wgpu::Buffer,
            t.bytes_per_row,
        )
    };

    inner
        .renderer
        .render_scene_to_encoder(&scene, &mut encoder, unsafe { &*view_ptr }, clear);

    let staging = unsafe { &*staging_ptr };
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: unsafe { &*tex_ptr },
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bpr),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );

    inner.renderer.queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = inner
        .renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
    if rx.recv().ok().and_then(|r| r.ok()).is_none() {
        return;
    }

    let data = slice.get_mapped_range().expect("staging buffer mapped");
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as usize {
        let src = &data[y * (bpr as usize)..y * (bpr as usize) + (w as usize * 4)];
        let dst = &mut pixels[y * (w as usize * 4)..(y + 1) * (w as usize * 4)];
        dst.copy_from_slice(src);
    }
    drop(data);
    staging.unmap();

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
