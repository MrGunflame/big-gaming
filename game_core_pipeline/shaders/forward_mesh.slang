import types;
import camera;

static const u32 VERTEX_COUNT = 128;
static const u32 TRIANGLE_COUNT = 256;

struct Meshlet {
    u32 vertex_offset;
    u32 triangle_offset;
    u32 vertex_count;
    u32 triangle_count;
    // FIXME: This is the same for the entire mesh, so maybe we can share this
    // across all meshlets?
    u32 positions_offset;
    u32 normals_offset;
    u32 uvs_offset;
    u32 tangents_offset;
}

struct Instance {
    Mat4 transform;
    Mat3 normal;
    u32 meshlet_offset;
    u32 meshlet_count;
    u32 material_index;
}

struct Vertex {
    Vec4 clip_position: SV_Position;
    [[vk::location(0)]] vector<f32, 3> world_position;
    [[vk::location(1)]] vector<f32, 3> world_normal;
    [[vk::location(2)]] vector<f32, 2> uv;
    [[vk::location(3)]] vector<f32, 4> world_tangent;
    [[vk::location(4)]] u32 material_index;
}

[[vk::push_constant]] Camera camera;

// Buffers storing vertex data.
// These buffers are 
[[vk::binding(0, 0)]] StructuredBuffer<f32> positions;
[[vk::binding(1, 0)]] StructuredBuffer<f32> normals;
[[vk::binding(2, 0)]] StructuredBuffer<f32> uvs;
[[vk::binding(3, 0)]] StructuredBuffer<f32> tangents;

// Buffers storing Meshlet data.
// vertex_indices contains offsets into the vertex data buffers,
// so that vertex data itself does not need to be duplicated for multiple meshlets.
[[vk::binding(4, 0)]] StructuredBuffer<u32> vertex_indices;
[[vk::binding(5, 0)]] StructuredBuffer<u8> triangle_indices;
[[vk::binding(6, 0)]] StructuredBuffer<Meshlet> meshlets;

[[vk::binding(7, 0)]] StructuredBuffer<Instance> instances;

struct Payload {
    u32 instance_index;
}

[shader("mesh")]
[outputtopology("triangle")]
[numthreads(TRIANGLE_COUNT, 1, 1)]
void main(
    u32 tid: SV_GroupThreadID,
    u32 gid: SV_GroupID,
    in payload Payload payload,
    out indices vector<u32, 3> triangles[TRIANGLE_COUNT],
    out vertices Vertex vertices[VERTEX_COUNT]
) {
    let instance = instances[payload.instance_index];
    let meshlet = meshlets[instance.meshlet_offset + gid];
    SetMeshOutputCounts(meshlet.vertex_count, meshlet.triangle_count);

    if (tid < meshlet.triangle_count) {
        let index0 = triangle_indices[meshlet.triangle_offset + tid * 3 + 0];
        let index1 = triangle_indices[meshlet.triangle_offset + tid * 3 + 1];
        let index2 = triangle_indices[meshlet.triangle_offset + tid * 3 + 2];
        triangles[tid] = vector<u32, 3>(index0, index1, index2);
    }

    if (tid < meshlet.vertex_count) {
        let vertex_index = vertex_indices[meshlet.vertex_offset + tid];

        // The meslet offsets are multiple of `f32`s, but `vertex_index` is a multiple
        // of a vertex attribute. The size of the depends on the attribute (vector size).
        let position = fetch_from_start<3>(positions, meshlet.positions_offset + vertex_index * 3);
        let normal = fetch_from_start<3>(normals, meshlet.normals_offset + vertex_index * 3);
        let uv = fetch_from_start<2>(uvs, meshlet.uvs_offset + vertex_index * 2);
        let tangent = fetch_from_start<4>(tangents, meshlet.tangents_offset + vertex_index * 4);

        Vertex vertex;
        vertex.clip_position = mul(mul(camera.view_proj, instance.transform), Vec4(position, 1.0));
        vertex.world_position = mul(instance.transform, Vec4(position, 1.0)).xyz;
        vertex.world_normal = mul(instance.normal, normal);
        vertex.world_tangent = Vec4(mul(instance.normal, tangent.xyz), tangent.w);
        vertex.uv = uv;
        vertex.material_index = instance.material_index;
        vertices[tid] = vertex;
    }
}

// Fetches a vector from a densly packed buffer. The `offset` points to the first element of the dense vector.
func fetch_from_start<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + i];
    }
    return value;
}
