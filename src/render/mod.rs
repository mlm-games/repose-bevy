use bevy::prelude::*;

use crate::plugin::ReposePluginSettings;

mod commands;
pub(crate) use commands::apply_render_commands;

#[cfg(any(feature = "offscreen", feature = "shared-device"))]
mod overlay_material;
#[cfg(any(feature = "offscreen", feature = "shared-device"))]
pub(crate) use overlay_material::{register_overlay_material, ReposeOverlayMaterial};

#[cfg(feature = "offscreen")]
mod offscreen;

#[cfg(feature = "shared-device")]
mod shared;

pub struct ReposeRenderPlugin {
    pub settings: ReposePluginSettings,
}

impl Plugin for ReposeRenderPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(any(feature = "offscreen", feature = "shared-device"))]
        register_overlay_material(app);

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
