use repose_core::RenderCommand;
use repose_render_wgpu::WgpuSceneRenderer;

pub fn apply_render_commands(renderer: &mut WgpuSceneRenderer, cmds: Vec<RenderCommand>) {
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
                let plane_refs: Vec<&[u8]> = planes.iter().map(|p| p.as_ref()).collect();
                if let Err(e) =
                    renderer.set_image_planes(handle, w, h, pixel_format, &plane_refs, color_info)
                {
                    bevy::log::warn!("repose-bevy: SetImagePlanes({handle}): {e:#}");
                }
            }
            RenderCommand::RemoveImage { handle } => {
                renderer.remove_image(handle);
            }
            _ => {}
        }
    }
}
