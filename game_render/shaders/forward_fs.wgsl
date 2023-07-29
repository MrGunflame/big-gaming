
struct FragInput {
    @builtin(position) clip_position: vec4<f32>,
}

@fragment
fn fs_main(in: FragInput) -> @location(0) vec4<f32> {
    return vec4(1.0, 0.0, 0.0,1.0);
}
