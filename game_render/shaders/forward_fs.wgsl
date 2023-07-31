struct MaterialConstants {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Camera {
    position: vec3<f32>,
    view_proj: mat4x4<f32>,
}

@group(2) @binding(0)
var<uniform> constants: MaterialConstants;
@group(2) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(2) @binding(2)
var normal_texture: texture_2d<f32>;
@group(2) @binding(3)
var metallic_roughness_texture: texture_2d<f32>;
@group(2) @binding(4)
var linear_sampler: sampler;

@group(3) @binding(0)
var<storage> directional_lights: array<DirectionalLight>;

struct FragInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@fragment
fn fs_main(in: FragInput) -> @location(0) vec4<f32> {
    let color = constants.base_color * textureSample(base_color_texture, linear_sampler, in.uv);

    var light_strength: vec3<f32> = vec3(0.0);
    for (var i: u32 = 0u; i < arrayLength(&directional_lights); i++) {
        light_strength += compute_directional_light(in, directional_lights[i]);
    }

    return color;
}

fn compute_directional_light(in: FragInput, light: DirectionalLight) -> vec3<f32> {
    let light_dir = normalize(light.position);

    let ambient = light.color * 0.05;

    let diffuse = max(dot(in.world_normal, light_dir), 0.0);

    let view_dir = normalize(camera.position - in.world_position);
    let half_dir = normalize(view_dir + light_dir);

    let specular = pow(max(dot(in.world_normal, half_dir), 0.0), 32.0);

    return ambient + diffuse + specular;
}

struct DirectionalLight {
    position: vec3<f32>,
    color: vec3<f32>,
}
