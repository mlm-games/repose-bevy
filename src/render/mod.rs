use bevy::prelude::*;

use crate::plugin::ReposePluginSettings;

#[cfg(feature = "offscreen")]
mod offscreen;

#[cfg(feature = "shared-device")]
mod shared;

pub struct ReposeRenderPlugin {
    pub settings: ReposePluginSettings,
}

pub fn pick_msaa(adapter: &wgpu::Adapter, requested: u32, format: wgpu::TextureFormat) -> u32 {
    repose_render_wgpu::pick_surface_msaa(adapter, format, requested)
}

pub fn overlay_image(
    width: u32,
    height: u32,
    usage: bevy::render::render_resource::TextureUsages,
    sampler: bevy::image::ImageSampler,
) -> bevy::prelude::Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat as BevyTexFormat};
    let mut image = bevy::prelude::Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        BevyTexFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = usage;
    image.sampler = sampler;
    image
}

pub fn spawn_overlay(commands: &mut bevy::prelude::Commands, handle: bevy::prelude::Handle<bevy::prelude::Image>) {
    use bevy::prelude::*;
    use bevy::ui::widget::{ImageNode, NodeImageMode};
    use bevy::ui::{FocusPolicy, Node, ZIndex};
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

impl Plugin for ReposeRenderPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "offscreen")]
        app.add_plugins(offscreen::OffscreenRenderPlugin {
            settings: self.settings.clone(),
        });

        #[cfg(feature = "shared-device")]
        app.add_plugins(shared::SharedDevicePlugin {
            settings: self.settings.clone(),
        });

        #[cfg(not(any(feature = "offscreen", feature = "shared-device")))]
        compile_error!("Enable at least one of: offscreen, shared-device");
    }
}
