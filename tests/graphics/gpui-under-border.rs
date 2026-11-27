//! GPU evaluation of the production quad fragment body with synthetic inputs.
use super::*;
use windows::{Win32::Graphics::Direct3D::Fxc::D3DCompile, core::s};

const SIDE: usize = 16;
const CASES: usize = 4 * 3 * 3 * 2;

#[::core::prelude::v1::test]
fn under_border_shader_preserves_translucent_contour() {
    let original = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders.hlsl"));
    let source = original
        .replace(
            "#include \"alpha_correction.hlsl\"",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/alpha_correction.hlsl"
            )),
        )
        .replace(
            "StructuredBuffer<Quad> quads: register(t1);",
            "StructuredBuffer<Quad> quads: register(t1);\nstatic Quad fixture_quad;",
        )
        .replace(
            "Quad quad = quads[input.quad_id];",
            "Quad quad = fixture_quad;",
        )
        + include_str!("gpui-under-border-test.hlsl");
    let devices = crate::directx_devices::DirectXDevices::new().expect("real Direct3D device");
    let device = &devices.device;
    let context = &devices.device_context;
    let mut bytecode = None;
    let mut errors = None;
    // SAFETY: source and output COM slots remain live for the synchronous compiler.
    let compiled = unsafe {
        D3DCompile(
            source.as_ptr().cast(),
            source.len(),
            None,
            None,
            None,
            s!("under_border_test"),
            s!("cs_5_0"),
            0,
            0,
            &mut bytecode,
            Some(&mut errors),
        )
    };
    if let Err(error) = compiled {
        let detail = errors
            .map(|blob| unsafe {
                String::from_utf8_lossy(slice::from_raw_parts(
                    blob.GetBufferPointer().cast::<u8>(),
                    blob.GetBufferSize(),
                ))
                .into_owned()
            })
            .unwrap_or_default();
        panic!("production fragment compilation: {error}: {detail}");
    }
    let bytecode = bytecode.unwrap();
    let mut shader = None;
    let count = SIDE * SIDE * CASES * 2;
    let bytes = (count * std::mem::size_of::<[f32; 4]>()) as u32;
    let mut output = None;
    let mut readback = None;
    let mut view = None;
    // SAFETY: descriptors match the structured float4 buffer and all COM outputs are owned.
    unsafe {
        let code = slice::from_raw_parts(
            bytecode.GetBufferPointer().cast::<u8>(),
            bytecode.GetBufferSize(),
        );
        device
            .CreateComputeShader(code, None, Some(&mut shader))
            .unwrap();
        device
            .CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: bytes,
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_UNORDERED_ACCESS.0 as u32,
                    MiscFlags: D3D11_RESOURCE_MISC_BUFFER_STRUCTURED.0 as u32,
                    StructureByteStride: 16,
                    ..Default::default()
                },
                None,
                Some(&mut output),
            )
            .unwrap();
        device
            .CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: bytes,
                    Usage: D3D11_USAGE_STAGING,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    ..Default::default()
                },
                None,
                Some(&mut readback),
            )
            .unwrap();
        device
            .CreateUnorderedAccessView(output.as_ref().unwrap(), None, Some(&mut view))
            .unwrap();
        context.CSSetShader(shader.as_ref().unwrap(), None);
        context.CSSetUnorderedAccessViews(0, 1, Some(&view as *const _), None);
        context.Dispatch((SIDE * SIDE / 64) as u32, CASES as u32, 1);
        context.CSSetUnorderedAccessViews(0, 1, Some(&None as *const _), None);
        context.CopyResource(readback.as_ref().unwrap(), output.as_ref().unwrap());
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context
            .Map(
                readback.as_ref().unwrap(),
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut mapped),
            )
            .unwrap();
        let samples = slice::from_raw_parts(mapped.pData.cast::<[f32; 4]>(), count);
        let result = verify_samples(samples);
        context.Unmap(readback.as_ref().unwrap(), 0);
        result.unwrap();
    }
}

fn over(source: [f32; 4], target: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|c| {
        if c == 3 {
            source[3] + target[3] * (1.0 - source[3])
        } else {
            source[c] * source[3] + target[c] * (1.0 - source[3])
        }
    })
}

fn verify_samples(samples: &[[f32; 4]]) -> Result<(), String> {
    let fills = [0.0, 0.36, 0.78, 1.0];
    let borders = [0.0, 0.4, 1.0];
    let scales = [1.0_f32, 1.25, 2.0];
    let targets = [[0.0; 4], [0.12, 0.24, 0.08, 0.6], [0.15, 0.3, 0.7, 1.0]];
    let mut maximum = 0.0_f32;
    for case in 0..CASES {
        let fill = fills[case % 4];
        let border = borders[(case / 4) % 3];
        let scale = scales[(case / 12) % 3];
        let width = (1 + case / 36) as f32 * scale;
        for pixel in 0..SIDE * SIDE {
            let x = (pixel % SIDE) as f32 + 0.5;
            let y = (pixel / SIDE) as f32 + 0.5;
            let radius = 8.0 * scale;
            let dx = radius - x;
            let dy = radius - y;
            let outer_sdf = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius;
            let outer = (0.5 - outer_sdf).clamp(0.0, 1.0);
            let inner_sdf = if dx <= 0.0 || dy <= 0.0 {
                -((width - x).max(width - y))
            } else if width - x > 0.0 || width - y > 0.0 {
                -1.0
            } else {
                -(outer_sdf + width)
            };
            let q = (0.5 - inner_sdf).clamp(0.0, 1.0) * border;
            let denominator = 1.0 - outer * q;
            let coverage = if denominator > 0.0 {
                outer * (1.0 - q) / denominator
            } else {
                0.0
            };
            let background = samples[(case * SIDE * SIDE + pixel) * 2];
            let foreground = samples[(case * SIDE * SIDE + pixel) * 2 + 1];
            let expected_background = [0.18, 0.42, 0.7, fill * coverage];
            let expected_foreground = [0.8, 0.3, 0.12, outer * q];
            for target in targets {
                // The existing child remains between background and foreground passes.
                for child in [[0.0; 4], [0.22, 0.66, 0.38, 0.62]] {
                    let actual = over(foreground, over(child, over(background, target)));
                    let expected = over(
                        expected_foreground,
                        over(child, over(expected_background, target)),
                    );
                    for channel in 0..4 {
                        let error = (actual[channel] - expected[channel]).abs();
                        if !error.is_finite() {
                            return Err(format!("non-finite case={case} pixel={pixel}"));
                        }
                        maximum = maximum.max(error);
                        if error > 0.0001 {
                            return Err(format!(
                                "contour mismatch case={case} pixel={pixel} channel={channel}: {actual:?} != {expected:?}"
                            ));
                        }
                    }
                }
            }
        }
    }
    eprintln!(
        "UnderBorder GPU: {} pixels, 3 target/2 child states, max channel error={maximum}",
        CASES * SIDE * SIDE
    );
    Ok(())
}
