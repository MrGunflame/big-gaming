module types;

public typealias f16 = float16_t;
public typealias f32 = float32_t;
public typealias f64 = float64_t;

public typealias u8 = uint8_t;
public typealias u16 = uint16_t;
public typealias u32 = uint32_t;
public typealias u64 = uint64_t;

public typealias i8 = int8_t;
public typealias i16 = int16_t;
public typealias i32 = int32_t;
public typealias i64 = int64_t;

public typealias Vec2 = vector<f32, 2>;
public typealias Vec3 = vector<f32, 3>;
public typealias Vec4 = vector<f32, 4>;

public typealias Mat2 = matrix<f32, 2, 2>;
public typealias Mat3 = matrix<f32, 3, 3>;
public typealias Mat4 = matrix<f32, 4, 4>;

// An element that can be submitted as part of a `VkCmdDrawIndexedIndirect` command.
// https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdDrawIndexedIndirect.html
public struct DrawIndexedIndirectCommand {
    public u32 index_count;
    public u32 instance_count;
    public u32 first_index;
    public u32 vertex_offset;
    public u32 first_instance;
}
