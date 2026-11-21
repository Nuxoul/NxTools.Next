// Screen-space SDF pixel footprint: L1 norm of its gradient.
// https://blog.pkh.me/p/44-perfecting-anti-aliasing-on-signed-distance-functions.html
// Rounded-box arcs have circle normals; straight edges have axis normals.
float quad_pixel_width(float2 corner_center_to_point, float radius) {
    float2 normal = max(corner_center_to_point, float2(0.0, 0.0));
    float normal_squared = dot(normal, normal);
    if (radius <= 0.0 || normal_squared == 0.0) {
        return 1.0;
    }
    return max(1.0, (normal.x + normal.y) * rsqrt(normal_squared));
}

float quad_pixel_coverage(float distance, float width) {
    return saturate(0.5 - distance / width);
}
