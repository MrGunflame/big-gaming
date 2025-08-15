import types;
import camera;
import shared;

[[vk::push_constant]] Camera camera;

[[vk::binding(0, 0)]] StructuredBuffer<f32> positions;
[[vk::binding(1, 0)]] StructuredBuffer<f32> normals;
[[vk::binding(2, 0)]] StructuredBuffer<f32> uvs;
[[vk::binding(3, 0)]] StructuredBuffer<f32> tangents;
[[vk::binding(4, 0)]] StructuredBuffer<f32> colors;
[[vk::binding(5, 0)]] StructuredBuffer<MeshOffsets> offsets;
[[vk::binding(6, 0)]] StructuredBuffer<InstanceOut> instances;

struct Input {
    u32 draw_id: SV_DrawIndex;
    u32 vertex_index: SV_VertexID;
}

struct Output {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] Vec3 world_position;
    [[vk::location(1)]] Vec3 world_normal;
    [[vk::location(2)]] Vec2 uv;
    [[vk::location(3)]] Vec4 world_tangent;
    [[vk::location(4)]] u32 material_index;
    [[vk::location(5)]] Vec4 vertex_color;
    [[vk::location(6)]] Vec4 ndc;
}

[shader("vertex")]
Output main(Input input) {
    let instance = instances[input.draw_id];
    let offsets = offsets[instance.offsets_index];

    let position = fetch<3>(positions, offsets.position, input.vertex_index);
    let normal = fetch<3>(normals, offsets.normal, input.vertex_index);
    let uv = fetch<2>(uvs, offsets.uv, input.vertex_index);
    let tangent = fetch<4>(tangents, offsets.tangent, input.vertex_index);
    let color = fetch<4>(colors, offsets.color, input.vertex_index);

    Output output;
    output.clip_position = mul(mul(camera.view_proj, instance.transform), Vec4(position, 1.0));
    output.world_position = mul(instance.transform, Vec4(position, 1.0)).xyz;
    output.world_normal = mul(instance.normal, normal);
    output.world_tangent = Vec4(mul(instance.normal, tangent.xyz), tangent.w);
    output.uv = uv;
    output.material_index = instance.material_index;
    output.vertex_color = color;
    output.ndc = output.clip_position;
    return output;
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}
