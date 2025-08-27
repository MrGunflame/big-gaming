module shared;

import types;

public struct InstanceIn {
    public Mat4 transform;
    public Mat3 normal;
    public u32 material_index;
    public u32 offsets_index;
    public u32 index_offset;
    public u32 index_count;
}

public struct InstanceOut {
    public Mat4 transform;
    public Mat3 normal;
    public u32 material_index;
    public u32 offsets_index;
    public u32 pad1;
    public u32 pad2;
}

public struct MeshOffsets {
    public u32 position;
    public u32 normal;
    public u32 uv;
    public u32 tangent;
    public u32 color;
    u32 _pad0;
    u32 _pad1;
    u32 _pad2;
}

public struct Element {
    public vector<f16, 4> color;
    public f32 depth;
    public u32 next;
}
