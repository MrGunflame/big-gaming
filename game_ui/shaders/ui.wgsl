@group(0) @binding(0)
var<storage> vertices: array<Vertex>;
@group(0) @binding(1)
var sprite_sampler: sampler;
@group(0) @binding(2)
var textures: binding_array<texture_2d<f32>>;

struct Vertex {
    color: vec4<f32>,
    position: vec3<f32>,
    texture_index: u32,
    uv: vec2<f32>,
}

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(3) texture_index: u32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let vertex = vertices[in.instance_index * 4u + in.vertex_index];

    var out: VertexOutput;
    out.clip_position = vec4(vertex.position, 1.0);
    out.uv = vertex.uv;
    out.color = vertex.color;
    out.texture_index = vertex.texture_index;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = in.color * textureSample(textures[in.texture_index], sprite_sampler, in.uv);
    color = vec4(gamma_correct(color.rgb), color.a);
    return color;
}

fn gamma_correct(color: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    return pow(color, vec3(1.0 / gamma));
}
