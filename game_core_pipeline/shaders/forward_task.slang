import types;

static const u32 WAVE_SIZE = 32;

struct Instance {
    Mat4 transform;
    Mat3 normal;
    u32 meshlet_offset;
    u32 meshlet_count;
    u32 material_index;
}

[[vk::binding(7, 0)]] StructuredBuffer<Instance> instances;

struct Payload {
    u32 instance_index;
}

[shader("amplification")]
[numthreads(1, 1, 1)]
void main(
    u32 gid: SV_GroupID,
    u32 tid: SV_GroupThreadID,
) {
    let instance = instances[gid];

    Payload payload;
    payload.instance_index = gid;

    DispatchMesh(instance.meshlet_count, 1, 1, payload);
}
