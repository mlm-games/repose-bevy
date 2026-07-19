use bevy::prelude::*;
use crate::plugin::ReposePluginSettings;

pub struct SharedDevicePlugin {
    pub settings: ReposePluginSettings,
}

impl Plugin for SharedDevicePlugin {
    fn build(&self, app: &mut App) {
        bevy::log::warn!(
            "repose-bevy shared-device: enable on Bevy with wgpu 30. \
             Implement RenderDevice::wgpu_device().clone() -> WgpuSceneRenderer::from_device."
        );

        let _ = &self.settings;
        let _ = app;
    }
}
