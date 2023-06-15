@group(0) @binding(0)
var d_texture: texture_2d<f32>;
@group(0) @binding(1)
var d_sampler: sampler;

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
    return textureSample(d_texture, d_sampler, in.uv);
    //let dims = textureDimensions(d_texture);
    //let x: u32 = u32(in.uv.x) * dims.x;
    //let y: u32 = u32(in.uv.y) * dims.y;
    //return textureLoad(d_texture, vec2(x, y), 0);
}
