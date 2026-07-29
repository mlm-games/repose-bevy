#![allow(clippy::type_complexity)]

pub mod compose;
pub use compose::compose_repose_system;
pub mod cursor;
pub mod input;
pub mod platform;
pub mod plugin;
pub mod render;
pub mod state;

pub mod prelude {
    pub use crate::plugin::{ReposePlugin, ReposePluginSettings};
    pub use crate::state::{ReposeOutput, ReposeState, ReposeUiImage};
    pub use repose_app::{FrameOutput, PlatformOutput, ReposeRuntime};
    pub use repose_core::prelude::*;
    pub use repose_core::{RenderContext, Scene, Scheduler, View};
    pub use repose_ui;
}

pub use plugin::*;
pub use state::*;
