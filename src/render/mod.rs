use bevy::prelude::*;

use crate::plugin::ReposePluginSettings;

#[cfg(feature = "offscreen")]
mod offscreen;

#[cfg(feature = "shared-device")]
mod shared;

pub struct ReposeRenderPlugin {
    pub settings: ReposePluginSettings,
}

/// Pick the MSAA sample count for the offscreen UI target, honoring the
/// requested count and falling back to the largest supported count <= it.
pub fn pick_msaa(adapter: &wgpu::Adapter, requested: u32, format: wgpu::TextureFormat) -> u32 {
    let color_feat = adapter.get_texture_format_features(format);
    let depth_feat = adapter.get_texture_format_features(wgpu::TextureFormat::Depth24PlusStencil8);
    let supported = |n: u32| {
        color_feat.flags.sample_count_supported(n)
            && color_feat
                .flags
                .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
            && depth_feat.flags.sample_count_supported(n)
    };
    let mut candidates = vec![requested];
    for n in [8, 4, 2, 1] {
        if n < requested {
            candidates.push(n);
        }
    }
    let chosen = candidates.into_iter().find(|&n| supported(n)).unwrap_or(1);
    if chosen != requested {
        bevy::log::info!("repose-bevy: requested MSAA x{requested}, using x{chosen}");
    }
    chosen
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
