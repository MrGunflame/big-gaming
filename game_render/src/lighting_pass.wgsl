@group(0) @binding(0)
var g_position: texture_2d<f32>;
@group(0) @binding(1)
var g_sampler: sampler;
@group(0) @binding(2)
var g_normal: texture_2d<f32>;
@group(0) @binding(3)
var g_albedo: texture_2d<f32>;
@group(0) @binding(4)
var<uniform> camera: CameraProjection;

@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var shadow_texture: texture_depth_2d;
@group(2) @binding(1)
var shadow_sampler: sampler_comparison;

struct Light {
    color: vec3<f32>,
    position: vec3<f32>,
    space_matrix: mat4x4<f32>,
}

struct CameraProjection {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
};

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
    let pos = textureSample(g_position, g_sampler, in.uv);
    let normal = textureSample(g_normal, g_sampler, in.uv);
    let albedo = textureSample(g_albedo, g_sampler, in.uv);

    var l: DirectionalLight;
    l.direction = -light.position;
    l.color = light.color;
    l.ambient = 0.01;
    l.diffuse = 1.0;
    l.specular = 1.0;
    l.space_matrix = light.space_matrix;
    let li = directional_light(in, l);

    let color = albedo.xyz * li;

    return vec4(color, 1.0);
    //return normal;
}

struct DirectionalLight {
    direction: vec3<f32>,
    color: vec3<f32>,
    ambient: f32,
    diffuse: f32,
    specular: f32,
    space_matrix: mat4x4<f32>,
}

fn directional_light(in: VertexOutput, light: DirectionalLight) -> vec3<f32> {
    let pos = textureSample(g_position, g_sampler, in.uv).xyz;
    let normal = textureSample(g_normal, g_sampler, in.uv).xyz;

    let light_space_position = light.space_matrix * vec4(pos, 1.0);
    
    //let light_pos = normalize(-light.direction);

    //let light_dir = normalize(in.tangent_light_pos - in.tangent_pos);
    let light_dir = normalize(light.direction);

    // Ambient
    let ambient = light.color * light.ambient;

    // Diffuse
    let diffuse_strength = max(dot(normal, light_dir), 0.0);
    let diffuse = light.color * diffuse_strength;

    // Specular
    let view_dir = normalize(camera.position.xyz - pos);
    let half_dir = normalize(view_dir + light_dir);

    let specular_strength = pow(max(dot(normal, half_dir), 0.0), 32.0);
    let specular = light.color * specular_strength;

    // Shadow mapping
    let shadow = shadow_occlusion(light_space_position);

    return shadow * (ambient + diffuse + specular);
}

fn shadow_occlusion(position: vec4<f32>) -> f32 {
    if (position.w <= 0.0) {
        return 0.0;
    }

    let flip_corretion = vec2<f32>(0.5, -0.5);

    let proj_correction = 1.0 / position.w;

    let light_local = position.xy * flip_corretion * proj_correction * vec2<f32>(0.5, 0.5);

    //var proj_coords = position.xyz / position.w;
    //proj_coords = proj_coords * 0.5 + 0.5;

    //let closest_depth = textureSample(shadow_texture, shadow_sampler, proj_coords.xy);

    //let current_depth = proj_coords.z;

    //if current_depth > closest_depth {
        //return 1.0;
    //} else {
        //return 0.0;
    //}



    return textureSampleCompareLevel(shadow_texture, shadow_sampler, light_local, position.w * proj_correction);
}
