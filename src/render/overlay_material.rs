use bevy::asset::load_internal_asset;
use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::BlendState;
use bevy::render::render_resource::RenderPipelineDescriptor;
use bevy::shader::Shader;
use bevy::shader::ShaderRef;
use bevy::ui_render::ui_material::UiMaterial;
use bevy::ui_render::ui_material::UiMaterialKey;
use bevy::ui_render::UiMaterialPlugin;

pub const REPOSE_OVERLAY_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("29834758-9234-7500-0000-000000000000");

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct ReposeOverlayMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl UiMaterial for ReposeOverlayMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(REPOSE_OVERLAY_SHADER_HANDLE)
    }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {
        if let Some(frag) = descriptor.fragment.as_mut() {
            if let Some(target) = frag.targets[0].as_mut() {
                target.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
            }
        }
    }
}

pub(crate) fn register_overlay_material(app: &mut App) {
    load_internal_asset!(
        app,
        REPOSE_OVERLAY_SHADER_HANDLE,
        "repose_overlay.wgsl",
        Shader::from_wgsl
    );
    app.add_plugins(UiMaterialPlugin::<ReposeOverlayMaterial>::default());
}
