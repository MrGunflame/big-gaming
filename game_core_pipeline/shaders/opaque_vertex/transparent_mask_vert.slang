import types;
import camera;
import shared;

[[vk::push_constant]] Camera camera;

[[vk::binding(0, 0)]] StructuredBuffer<InstanceOut> instances;
[[vk::binding(1, 0)]] StructuredBuffer<MeshOffsets> offsets;
[[vk::binding(2, 0)]] StructuredBuffer<u32> index_buffer;
[[vk::binding(3, 0)]] StructuredBuffer<f32> positions;
[[vk::binding(4, 0)]] StructuredBuffer<f32> uvs;
[[vk::binding(5, 0)]] StructuredBuffer<f32> colors;

struct Input {
    u32 draw_id: SV_DrawIndex;
    // In Slang SV_VertexID is VertexIndex - BaseVertex.
    // We dispatch one vertex shader invocation per index
    // in the index buffer and then do indexing manually.
    // This value refers to current index of the vertex.
    u32 index: SV_VertexID;
}

struct Output {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] u32 instance_id;
    [[vk::location(1)]] u32 vertex_id;
    [[vk::location(2)]] Vec2 uv;
    [[vk::location(3)]] f32 alpha;
}

[shader("vertex")]
func main(input: Input) -> Output {
    let instance = instances[input.draw_id];
    let offsets = offsets[instance.offsets_index];

    let vertex_index = index_buffer[instance.index_offset + input.index];

    let position = fetch<3>(positions, offsets.position, vertex_index);
    let uv = fetch<2>(uvs, offsets.uv, vertex_index);
    let color = fetch<4>(colors, offsets.color, vertex_index);

    Output output;
    output.clip_position = mul(mul(camera.view_proj, instance.transform), Vec4(position, 1.0));
    output.uv = uv;
    output.alpha = color.a;
    output.instance_id = input.draw_id;
    output.vertex_id = input.index / 3;
    return output;
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}
