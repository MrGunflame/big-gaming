import types;
import camera;
import pbr;
import lights;
import material;
import shared;

struct PushConstants {
    Camera camera;
    u32 num_directional_lights;
    u32 num_point_lights;
    u32 num_spot_lights;
}

[[vk::push_constant]] PushConstants push_constants;

[[vk::binding(0, 1)]] StructuredBuffer<DirectionalLight> directional_lights;
[[vk::binding(1, 1)]] StructuredBuffer<PointLight> point_lights;
[[vk::binding(2, 1)]] StructuredBuffer<SpotLight> spot_lights;
[[vk::binding(3, 1)]] StructuredBuffer<Material> materials;
[[vk::binding(4, 1)]] Texture2D<Vec4>[] textures;
[[vk::binding(5, 1)]] SamplerState sampler;

[[vk::binding(0, 2)]] RWStructuredBuffer<Element> elements;
[[vk::binding(1, 2)]] RWTexture2D<u32> heads;
[[vk::binding(2, 2)]] RWStructuredBuffer<Atomic<u32>> frag_counter;
[[vk::binding(3, 2)]] Texture2D<f32> depth;

struct Input {
    Vec4 clip_position: SV_Position;
    bool front_facing: SV_IsFrontFace;
    [[vk::location(0)]] Vec3 world_position;
    [[vk::location(1)]] Vec3 world_normal;
    [[vk::location(2)]] Vec2 uv;
    [[vk::location(3)]] Vec4 world_tangent;
    [[vk::location(4)]] u32 material_index;
    [[vk::location(5)]] Vec4 vertex_color;
}

[shader("fragment")]
func main(input: Input) {
    if (depth[vector<u32, 2>(input.clip_position.xy)] > input.clip_position.z) {
        discard;
    }

    var color = shading(input);

    // Fragments with alpha 0.0 can be completely ignored and discarded.
    // The logic would still be correct without discarding these fragments,
    // but they would have no effect on the blending stage, so we can discard
    // them here to save on memory.
    if (color.a == 0.0) {
        discard;
    }

    // The value 0 indicates termination (i.e. no value following).
    // Note that the when initialized the host sets the atomic value to
    // 1, preventing any thread to read the value 0 here.
    let index = frag_counter[0].add(1);
    if (index >= elements.getCount()) {
        discard;
    }

    var prev = 0;
    InterlockedExchange(heads[vector<u32, 2>(input.clip_position.xy)], index, prev);

    elements[index].color = vector<f16, 4>(color);
    elements[index].depth = input.clip_position.z;
    elements[index].next = prev;
}

func shading(input: Input) -> Vec4 {
    let material = materials[input.material_index];
    let uv = input.uv;

    var base_color = load_base_color(material, uv) * input.vertex_color;

    if ((material.flags & MATERIAL_FLAGS_UNLIT) != 0) {
        return base_color;
    }

    let normal = load_normal(input);

    let specular_strength = material.specular_strength * get_specular_strength(input);
    let specular_color = material.specular_color.rgb;

    var roughness = load_roughness(material, uv);
    if ((material.flags & MATERIAL_FLAGS_ROUGHNESS_FROM_GLOSSINESS) != 0) {
        roughness = get_roughness_from_glossiness(input);
    }

    var metallic = load_metallic(material, uv);
    if ((material.flags & MATERIAL_FLAGS_METALLIC_FROM_SPECULAR) != 0) {
        metallic = compute_metallic_from_specular_color(specular_color * specular_strength);
    }

    // BRDF parameters
    let view_dir = normalize(push_constants.camera.position - input.world_position);
    var pbr_material: pbr::Material;
    pbr_material.base_color = base_color.rgb;
    pbr_material.normal = normal;
    pbr_material.metallic = metallic;
    pbr_material.roughness = roughness;
    pbr_material.reflectance = material.reflectance;
    pbr_material.specular_color = specular_color;
    pbr_material.specular_strength = specular_strength;

    var luminance = Vec3(0.0);

    for (var i = 0; i < push_constants.num_directional_lights; i++) {
        let light = directional_lights[i].eval();
        luminance += pbr::surface_shading(pbr_material, view_dir, light);
    }

    for (var i = 0; i < push_constants.num_point_lights; i++) {
        let light = point_lights[i].eval(FragParams(input.world_position));
        luminance += pbr::surface_shading(pbr_material, view_dir, light);
    }

    for (var i = 0; i < push_constants.num_spot_lights; i++) {
        let light = spot_lights[i].eval(FragParams(input.world_position));
        luminance += pbr::surface_shading(pbr_material, view_dir, light);
    }

    return Vec4(luminance, base_color.a);
}

func load_base_color(material: Material, uv: Vec2) -> Vec4 {
    return material.base_color * textures[material.base_color_texture].Sample(sampler, uv);
}

func load_roughness(material: Material, uv: Vec2) -> f32 {
    return material.roughness * textures[material.metallic_roughness_texture].Sample(sampler, uv).g;
}

func load_metallic(material: Material, uv: Vec2) -> f32 {
    return material.metallic * textures[material.metallic_roughness_texture].Sample(sampler, uv).b;
}

func load_normal(input: Input) -> Vec3 {
    let normal_norm = normalize(input.world_normal);
    let tangent_norm = normalize(input.world_tangent.xyz);
    let bitangent = cross(normal_norm, tangent_norm) * input.world_tangent.w;
    let tbn = Mat3(tangent_norm, bitangent, normal_norm);

    let material = materials[input.material_index];
    var normal = textures[material.normal_texture].Sample(sampler, input.uv).rgb;

    // DirectX normal maps have the Y component flipped.
    if ((material.flags & MATERIAL_FLAGS_FLIP_NORMAL_Y) != 0) {
        normal.g = 1.0 - normal.g;
    }

    // Transform from [0..1] to [-1..1].
    normal = normal * 2.0 - 1.0;

    if ((material.flags & MATERIAL_FLAGS_NORMAL_ENCODING_TWO_COMPONENT) != 0) {
        normal.z = sqrt(clamp(1.0 - dot(normal.xy, normal.xy), 0.0, 1.0));
    }

    normal = normalize(normal);
    normal = normalize(mul(normal, tbn));

    // Invert the normal if the triangle is facing backwards.
    // Without this step the lighting direction will be inversed
    // for back-facing triangles.
    if (input.front_facing) {
        return normal;
    } else {
        return -normal;
    }
}

func get_specular_strength(input: Input) -> f32 {
    let material = materials[input.material_index];
    let texture = textures[material.specular_glossiness_texture];
    return material.specular_strength * texture.Sample(sampler, input.uv).r;
}

func get_roughness_from_glossiness(input: Input) -> f32 {
    let material = materials[input.material_index];
    let texture = textures[material.specular_glossiness_texture];
    
    let glossiness = 1.0 - material.roughness;
    return 1.0 - (glossiness * texture.Sample(sampler, input.uv).g);
}

func compute_metallic_from_specular_color(specular_color: Vec3) -> f32 {
    return max(max(specular_color.r, specular_color.g), specular_color.b);
}

