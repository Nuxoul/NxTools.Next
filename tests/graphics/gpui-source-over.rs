//! Included only in the temporary backend used by the standalone preview.
use super::*;

#[::core::prelude::v1::test]
fn transparent_surface_blend_states_use_source_over_alpha() {
    let devices = crate::directx_devices::DirectXDevices::new()
        .expect("create the real preview Direct3D device");
    let states = [
        create_blend_state(&devices.device).expect("ordinary primitive blend state"),
        create_blend_state_for_path_sprite(&devices.device).expect("path sprite blend state"),
    ];
    for state in states {
        let mut description = D3D11_BLEND_DESC::default();
        // SAFETY: the live COM blend state writes one fully allocated descriptor.
        unsafe { state.GetDesc(&mut description) };
        let target = description.RenderTarget[0];
        assert_eq!(target.SrcBlendAlpha, D3D11_BLEND_ONE);
        assert_eq!(target.DestBlendAlpha, D3D11_BLEND_INV_SRC_ALPHA);
        assert_eq!(target.BlendOpAlpha, D3D11_BLEND_OP_ADD);
    }
}
