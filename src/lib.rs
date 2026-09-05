#![allow(clippy::type_complexity)]

pub mod bridge;
pub mod compose;
pub mod cursor;
pub mod input;
pub mod platform;
pub mod plugin;
pub mod render;
pub mod state;

pub mod prelude {
    pub use crate::bridge::{BevyModifierExt, ReposeChannel, ReposeShared};
    pub use crate::plugin::{
        ReposeAppExt, ReposeCorePlugin, ReposePlugin, ReposePluginSettings, bevy_system,
    };
    pub use crate::state::{
        ReposeFrameRequest, ReposeOutput, ReposeState, ReposeUiImage, gating,
        repose_pointer_consumed, repose_scroll_consumed, repose_wants_keyboard,
        repose_wants_pointer, request_repose_frame, request_repose_frame_res,
    };
    pub use repose_app::{FrameOutput, PlatformOutput, ReposeRuntime};
    pub use repose_core::prelude::*;
    pub use repose_core::{RenderContext, Scene, Scheduler, View};
    pub use repose_ui;
}

pub use plugin::*;
pub use state::*;
