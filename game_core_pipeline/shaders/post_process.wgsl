struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let x = i32(in.vertex_index) / 2;
    let y = i32(in.vertex_index) & 1;

    let uv = vec2<f32>(f32(x) * 2.0, f32(y) * 2.0);

    out.clip_position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;

    return out;
}

@group(0) @binding(0)
var t_texture: texture_2d<f32>;
@group(0) @binding(1)
var t_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_texture, t_sampler, in.uv).rgb;

    color = tonemap(color);
    color = gamma_correct(color);

    // Screen space dithering should happen after converting to gamma correction.
    // The bound output texture is already linear, so the GPU will not do a convertion
    // to sRGB.
    color += screen_space_dither(in.clip_position.xy);

    return vec4(color, 1.0);
}

fn tonemap(color: vec3<f32>) -> vec3<f32> {
    // return color / (color + vec3(1.0));
    return aces_fitted(color);
}

fn gamma_correct(color: vec3<f32>) -> vec3<f32> {
    var out_color = vec3(0.0);
    out_color.r = linear_to_srgb(color.r);
    out_color.g = linear_to_srgb(color.g);
    out_color.b = linear_to_srgb(color.b);
    return color;
}

// sRGB transfer functions
// https://en.wikipedia.org/wiki/SRGB#Transformation

fn linear_to_srgb(color: f32) -> f32 {
    if color <= 0.0031308 {
        return color * 12.92;
    } else {
        return (pow(color, 1.0 / 2.4) * 1.055) - 0.055;
    }
}

fn srgb_to_linear(color: f32) -> f32 {
    if color <= 0.04045 {
        return color / 12.92;
    } else {
        return pow((color + 0.055) / 1.055, 2.4);
    }
}

// Screen space dithering
// https://github.com/bevyengine/bevy/blob/main/crates/bevy_core_pipeline/src/tonemapping/tonemapping_shared.wgsl
// https://media.steampowered.com/apps/valve/2015/Alex_Vlachos_Advanced_VR_Rendering_GDC2015.pdf
fn screen_space_dither(uv: vec2<f32>) -> vec3<f32> {
    var dither = vec3<f32>(dot(vec2<f32>(171.0, 231.0), uv)).xxx;
    dither = fract(dither.rgb / vec3<f32>(103.0, 71.0, 97.0));
    return (dither - 0.5) / 255.0;
}

// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl

const ACES_INPUT_MATRIX: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(0.59719, 0.35458, 0.04823),
    vec3<f32>(0.07600, 0.90834, 0.01566),
    vec3<f32>(0.02840, 0.13383, 0.83777)
);

const ACES_OUTPUT_MATRIX: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>( 1.60475, -0.53108, -0.07367),
    vec3<f32>(-0.10208,  1.10813, -0.00605),
    vec3<f32>(-0.00327, -0.07276,  1.07602)
);

fn aces_fitted(color: vec3<f32>) -> vec3<f32> {
    return rtt_and_odt_fit(color * ACES_INPUT_MATRIX) * ACES_OUTPUT_MATRIX;
}

fn rtt_and_odt_fit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a/b;
}
