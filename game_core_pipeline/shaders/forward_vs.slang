import types;

struct Camera {
    vector<f32, 3> position;
    matrix<f32, 4, 4> view_proj;
}

struct ObjectData {
    matrix<f32, 4, 4> transform;
    matrix<f32, 3, 3> normal;
    u32 material_index;
}

[vk::push_constant]
Camera camera;

[[vk::binding(0, 1)]]
StructuredBuffer<f32> positions;
[[vk::binding(1, 1)]]
StructuredBuffer<f32> normals;
[[vk::binding(2, 1)]]
StructuredBuffer<f32> tangents;
[[vk::binding(3, 1)]]
StructuredBuffer<f32> uvs;

[[vk::binding(0, 0)]]
ConstantBuffer<ObjectData> model;

struct Input {
    u32 vertex_index : SV_VertexID;
}

struct Output {
    vector<f32, 4> clip_position : SV_Position;
    [[vk::location(0)]]
    vector<f32, 3> world_position;
    [[vk::location(1)]]
    vector<f32, 3> world_normal;
    [[vk::location(2)]]
    vector<f32, 2> uv;
    [[vk::location(3)]]
    vector<f32, 4> world_tangent;
    [[vk::location(4)]]
    u32 material_index;
}

[shader("vertex")]
func vs_main(input: Input) -> Output {
    let position = fetch<3>(positions, input.vertex_index);
    let normal = fetch<3>(normals, input.vertex_index);
    let uv = fetch<2>(uvs, input.vertex_index);
    let tangent = fetch<4>(tangents, input.vertex_index);

    Output output = {};
    output.clip_position = mul(mul(camera.view_proj, model.transform), Vec4(position, 1.0));
    output.world_position = mul(model.transform, Vec4(position, 1.0)).xyz;
    output.world_normal = mul(model.normal, normal);
    output.world_tangent = Vec4(mul(model.normal, tangent.xyz), tangent.w);
    output.uv = uv;
    output.material_index = model.material_index;
    return output;
}

func fetch<let N : int>(buffer: StructuredBuffer<f32>, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var offset = 0; offset < N; offset++) {
        value[offset] = buffer.Load(index * N + offset);
    }
    return value;
}
