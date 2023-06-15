
@group(1) @binding(0)
var<uniform> base_color: vec4<f32>;
@group(1) @binding(1)
var color_texture: texture_2d<f32>;
@group(1) @ binding(2)
var color_texture_sampler: sampler;

@group(1) @binding(3)
var normal_texture: texture_2d<f32>;
@group(1) @binding(4)
var normal_sampler: sampler;

@group(0) @binding(0)
var<uniform> camera: CameraProjection;

struct CameraProjection {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent_light_pos: vec3<f32>,
    @location(4) tangent_view_pos: vec3<f32>,
    @location(5) tangent_pos: vec3<f32>,
};

// GBuffer output
struct GBuffer {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) albedo: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> GBuffer {
    var color: vec4<f32> = base_color * textureSample(color_texture, color_texture_sampler, in.uv);
    let normal = textureSample(normal_texture, normal_sampler, in.uv);

    let tangent_normal = normal.xyz * 2.0 - 1.0;

    var l: DirectionalLight;
    l.direction = vec3(1.0, 0.0, 0.0);
    l.color = vec3(1.0, 1.0, 1.0);
    l.ambient = 0.1;
    l.diffuse = 1.0;
    l.specular = 1.0;
    let light = directional_light(in, l, tangent_normal);

    //color *= light;
    
    //return color;
    //return show_normals(in);

    var gbuffer: GBuffer;
    gbuffer.position = in.position;
    gbuffer.normal = vec4(in.world_normal, 1.0);
    gbuffer.albedo = color;
    return gbuffer;
}

struct DirectionalLight {
    direction: vec3<f32>,
    color: vec3<f32>,
    ambient: f32,
    diffuse: f32,
    specular: f32,
}

fn directional_light(in: VertexOutput, light: DirectionalLight, tangent_normal: vec3<f32>) -> vec4<f32> {
    let light_dir = normalize(in.tangent_light_pos - in.tangent_pos);

    // Ambient
    let ambient = light.color * light.ambient;

    // Diffuse
    let diffuse_strength = max(dot(tangent_normal, light_dir), 0.0);
    let diffuse = light.color * diffuse_strength;

    // Specular
    let view_dir = normalize(in.tangent_view_pos - in.tangent_pos);
    let half_dir = normalize(view_dir + light_dir);

    let specular_strength = pow(max(dot(tangent_normal, half_dir), 0.0), 32.0);
    let specular = light.color * specular_strength;

    return vec4(ambient + diffuse + specular, 1.0);
}

fn show_normals(in: VertexOutput) -> vec4<f32> {
    let normal = textureSample(normal_texture, normal_sampler, in.uv).xyz;

    //let tangent_normal = normal.xyz * 2.0 - 1.0;
    //let n = normal * 2.0 - 1.0;
    let n = (in.tangent_light_pos - in.tangent_pos) * normal;

    return vec4(n, 1.0);
}
