
@group(1) @binding(0)
var<uniform> base_color: vec4<f32>;
@group(1) @binding(1)
var color_texture: texture_2d<f32>;
@group(1) @ binding(2)
var color_texture_sampler: sampler;


struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec4<f32> = base_color * textureSample(color_texture, color_texture_sampler, in.uv);

    let ambient_light = (ambient_light() + diffuse_light(in));
    color.x *= ambient_light.x;
    color.y *= ambient_light.y;
    color.z *= ambient_light.z;
    
    return color;
}

fn ambient_light() -> vec3<f32> {
    let strength = 0.1;
    let color = vec3(0.1, 0.1, 0.1) * strength;
    return color;
}

fn diffuse_light(in: VertexOutput) -> vec3<f32> {
    let light_position = vec3(0.0, 0.0, 0.0);
    let light_color = vec3(0.3, 0.3, 0.3);

    let light_dir = normalize(light_position - in.position.xyz);
    let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse_color = light_color * diffuse_strength;

    return diffuse_color;
}
