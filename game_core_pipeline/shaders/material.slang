import types;

public static const u32 MATERIAL_FLAGS_UNLIT = 1 << 0;
public static const u32 MATERIAL_FLAGS_FLIP_NORMAL_Y = 1 << 1;
public static const u32 MATERIAL_FLAGS_METALLIC_FROM_SPECULAR = 1 << 2;
public static const u32 MATERIAL_FLAGS_ROUGHNESS_FROM_GLOSSINESS = 1 << 3;
public static const u32 MATERIAL_FLAGS_NORMAL_ENCODING_TWO_COMPONENT = 1 << 4;

public struct Material {
    public u32 flags;
    public f32 alpha_mask;
    public Vec4 base_color;
    public f32 roughness;
    public f32 metallic;
    public f32 reflectance;
    public f32 specular_strength;
    public Vec4 specular_color;
    public u32 base_color_texture;
    public u32 normal_texture;
    public u32 metallic_roughness_texture;
    public u32 specular_glossiness_texture;
}
