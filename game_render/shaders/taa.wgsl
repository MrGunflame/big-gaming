
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Camera {
    position: vec3<f32>,
    view_proj: mat4x4<f32>,
    viewport_size: vec2<f32>,
}


fn vertex() {
    let delta_width = 1.0 / camera.viewport_size.x;
    let delta_height = 1.0 / camera.viewport_size.y;
    let index = total_frames % num_samples;

    let jitter = vec2(halton_sequence[index].x * delta_width, halton_sequence[index].y * delta_height);
    var new_proj = projection;
    new_proj[3][0] += jitter.x * halton_scale;
    new_proj[3][1] += jitter.y * halton_scale;
}
