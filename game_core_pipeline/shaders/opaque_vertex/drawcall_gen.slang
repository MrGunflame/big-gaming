import types;
import shared;

static const u32 NUM_THREADS = 64;

[[vk::binding(0, 0)]] StructuredBuffer<InstanceIn> instances_in;
[[vk::binding(1, 0)]] RWStructuredBuffer<InstanceOut> instances_out;
[[vk::binding(2, 0)]] RWStructuredBuffer<DrawIndexedIndirectCommand> draws;

[shader("compute")]
[numthreads(NUM_THREADS, 1, 1)]
void main(
    u32 tid: SV_GroupThreadID,
    u32 gid: SV_GroupID,
) {
    let index = gid * NUM_THREADS + tid;

    if (index >= instances_in.getCount()) {
        return;
    }

    let instance = instances_in[index];

    InstanceOut out_instance;
    out_instance.transform = instance.transform;
    out_instance.normal = instance.normal;
    out_instance.material_index = instance.material_index;
    out_instance.offsets_index = instance.offsets_index;

    DrawIndexedIndirectCommand draw;
    draw.first_instance = 0;
    draw.instance_count = 1;
    draw.vertex_offset = 0;
    draw.first_index = instance.index_offset;
    draw.index_count = instance.index_count;

    instances_out[index] = out_instance;
    draws[index] = draw;
}

struct DrawIndexedIndirectCommand {
    u32 index_count;
    u32 instance_count;
    u32 first_index;
    u32 vertex_offset;
    u32 first_instance;
}
