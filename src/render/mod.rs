use bevy::prelude::*;

use crate::plugin::ReposePluginSettings;

#[cfg(feature = "offscreen")]
mod offscreen;

#[cfg(feature = "shared-device")]
mod shared;

pub struct ReposeRenderPlugin {
    pub settings: ReposePluginSettings,
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
