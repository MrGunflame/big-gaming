const PI: f32 = 3.14159265358979323846264338327950288;

@group(0) @binding(0)
var g_position: texture_2d<f32>;
@group(0) @binding(4)
var g_sampler: sampler;
@group(0) @binding(2)
var g_normal: texture_2d<f32>;
@group(0) @binding(1)
var g_albedo: texture_2d<f32>;
@group(0) @binding(3)
var g_metallic_roughness: texture_2d<f32>;

@group(1) @binding(0)
var<uniform> light: Light;

@group(2) @binding(0)
var<uniform> camera: CameraProjection;

struct Light {
    color: vec3<f32>,
    position: vec3<f32>,
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
    let albedo = textureSample(g_albedo, g_sampler, in.uv).xyz;
    let metallic = textureSample(g_metallic_roughness, g_sampler, in.uv).b;
    let roughness = textureSample(g_metallic_roughness, g_sampler, in.uv).g;

    var l: PointLight;
    l.position = light.position;
    l.color = light.color;
    l.ambient = 0.01;
    l.diffuse = 1.0;
    l.specular = 1.0;
    l.constant = 0.0;
    l.linear = 0.1;
    l.quadratic = 0.0;
    let li = point_light(in, l);

    //let color = albedo.xyz * li;
    return vec4(li, 1.0);

    //return vec4(color, 1.0);
    //return vec4(normal.xyz, 1.0);
}

struct PointLight {
    position: vec3<f32>,
    color: vec3<f32>,
    ambient: f32,
    diffuse: f32,
    specular: f32,
    constant: f32,
    linear: f32,
    quadratic: f32,
}

fn point_light(in: VertexOutput, light: PointLight) -> vec3<f32> {
    let albedo = pow(textureSample(g_albedo, g_sampler, in.uv).xyz, vec3(2.2));
    let pos = textureSample(g_position, g_sampler, in.uv).xyz;
    let normal = textureSample(g_normal, g_sampler, in.uv).xyz;
    let metallic = textureSample(g_metallic_roughness, g_sampler, in.uv).b;
    let roughness = textureSample(g_metallic_roughness, g_sampler, in.uv).g;

    let distance = length(light.position - pos);
    // Don't divide by 0.
    let attenuation = 10.0 / max((light.constant + light.linear * distance + light.quadratic * (distance * distance)), 0.0001);

    let light_dir = normalize(light.position - pos);

    // Diffuse
    //let diffuse_strength = max(dot(normal, light_dir), 0.0);
    //let diffuse = light.color * diffuse_strength;

    // Specular
    let view_dir = normalize(camera.position.xyz - pos);
    let half_dir = normalize(view_dir + light_dir);

    //let specular_strength = pow(max(dot(normal, half_dir), 0.0), 32.0);
    //let specular = light.color * specular_strength;

    let radiance = light.color * attenuation;

    var f0 = vec3(0.04);
    f0 = mix(f0, albedo, metallic);

    // Cook-torrance BRDF
    let ndf = distribution_ggx(normal, half_dir, roughness);
    let g = geometry_smith(normal, view_dir, light_dir, roughness);
    let f = fresnel_schlick(max(dot(half_dir, view_dir), 0.0), f0);

    let ks = f;
    var kd = vec3(1.0) - ks;
    kd *= 1.0 - metallic;

    let numerator = ndf * g * f;
    let denominator = 4.0 * max(dot(normal, view_dir), 0.0) * max(dot(normal, light_dir), 0.0) + 0.0001;
    let specular = numerator / denominator;

    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let lo = (kd * albedo / PI + specular) * radiance * n_dot_l;

    // Ambient
    let ambient = light.color * light.ambient * albedo;

    var color = ambient + lo;

    return color;

    //return vec3(attenuation, 0.0, 1.0);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(n, h), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;

    let num = a2;
    var denom = (n_dot_h2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return num / denom;
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;

    let num = n_dot_v;
    let denom = n_dot_v * (1.0 - k) + k;

    return num / denom;
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32) -> f32 {
    let n_dot_v = max(dot(n, v), 0.0);
    let n_dot_l = max(dot(n, l), 0.0);

    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);

    return ggx1 * ggx2;
}
