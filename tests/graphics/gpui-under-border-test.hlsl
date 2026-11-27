
RWStructuredBuffer<float4> fixture_output : register(u0);
[numthreads(64, 1, 1)]
void under_border_test(uint3 id : SV_DispatchThreadID) {
    uint pixel = id.x;
    uint scenario = id.y;
    float fill_alphas[4] = {0.0, 0.36, 0.78, 1.0};
    float border_alphas[3] = {0.0, 0.4, 1.0};
    float scales[3] = {1.0, 1.25, 2.0};
    float fill_alpha = fill_alphas[scenario % 4];
    float border_alpha = border_alphas[(scenario / 4) % 3];
    float scale = scales[(scenario / 12) % 3];
    float width = (1 + scenario / 36) * scale;
    fixture_quad = (Quad)0;
    fixture_quad.bounds.size = float2(60, 40) * scale;
    fixture_quad.border_style = 2;
    fixture_quad.border_widths.top = width;
    fixture_quad.border_widths.left = width;
    fixture_quad.border_widths.right = width;
    fixture_quad.border_widths.bottom = width;
    fixture_quad.corner_radii.top_left = 8 * scale;
    fixture_quad.corner_radii.top_right = 8 * scale;
    fixture_quad.corner_radii.bottom_left = 8 * scale;
    fixture_quad.corner_radii.bottom_right = 8 * scale;
    QuadFragmentInput input = (QuadFragmentInput)0;
    input.position = float4(pixel % 16 + 0.5, pixel / 16 + 0.5, 0, 1);
    input.background_solid = float4(0.18, 0.42, 0.7, fill_alpha);
    input.border_color = float4(0.8, 0.3, 0.12, border_alpha);
    uint index = (scenario * 256 + pixel) * 2;
    if (border_alpha == 0) {
        fixture_quad.border_style = 0;
        fixture_quad.border_widths = (Edges)0;
    }
    fixture_output[index] = fill_alpha > 0 ? quad_fragment(input) : float4(0, 0, 0, 0);
    fixture_quad.border_widths.top = width;
    fixture_quad.border_widths.left = width;
    fixture_quad.border_widths.right = width;
    fixture_quad.border_widths.bottom = width;
    fixture_quad.border_style = 0;
    input.background_solid = float4(input.border_color.rgb, 0);
    fixture_output[index + 1] = border_alpha > 0 ? quad_fragment(input) : float4(0, 0, 0, 0);
}
