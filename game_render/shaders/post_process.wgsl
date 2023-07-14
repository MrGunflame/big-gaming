@group(0) @binding(0)
var g_texture: texture_2d<f32>;
@group(0) @binding(1)
var g_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = vec4(in.position.x, in.position.y, 0.0, 1.0);
    out.uv = in.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(g_texture, g_sampler, in.uv).rgb;

    color = tonemap(color);
    color = gamma_correct(color);

    return vec4(color, 1.0);
}

fn tonemap(color: vec3<f32>) -> vec3<f32> {
    return color / (color + vec3(1.0));
}

fn gamma_correct(color: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    return pow(color, vec3(1.0 / gamma));
}
