import types;
import lights;
import shared;
import material;
import camera;

import barycentrics;
import pbr;

struct PushConstants {
    Camera camera;
    u32 num_directional_lights;
    u32 num_point_lights;
    u32 num_spot_lights;
    vector<u32, 2> scren_size;
}

[[vk::push_constant]] PushConstants push_constants;

[[vk::binding(0, 0)]] Texture2D<vector<u32, 2>> visbuffer;
[[vk::binding(1, 0)]] RWTexture2D<Vec4> output;

[[vk::binding(0, 1)]] StructuredBuffer<f32> positions;
[[vk::binding(1, 1)]] StructuredBuffer<f32> normals;
[[vk::binding(2, 1)]] StructuredBuffer<f32> uvs;
[[vk::binding(3, 1)]] StructuredBuffer<f32> tangents;
[[vk::binding(4, 1)]] StructuredBuffer<f32> colors;
[[vk::binding(5, 1)]] StructuredBuffer<MeshOffsets> offsets;
[[vk::binding(6, 1)]] StructuredBuffer<InstanceOut> instances;
[[vk::binding(7, 1)]] StructuredBuffer<u32> index_buffer;

[[vk::binding(0, 2)]] StructuredBuffer<DirectionalLight> directional_lights;
[[vk::binding(1, 2)]] StructuredBuffer<PointLight> point_lights;
[[vk::binding(2, 2)]] StructuredBuffer<SpotLight> spot_lights;
[[vk::binding(3, 2)]] StructuredBuffer<Material> materials;
[[vk::binding(4, 2)]] Texture2D<Vec4>[] textures;
[[vk::binding(5, 2)]] SamplerState sampler;

static const u32 NUM_THREADS = 64;

static const u32 TILE_SIZE = 16;

[shader("compute")]
[numthreads(16, 16, 1)]
void main(
    vector<u32, 2> tid: SV_GroupThreadID,
    vector<u32, 2> gid: SV_GroupID,
) {
    let coords = gid * TILE_SIZE + tid;

    if (coords.x >= push_constants.scren_size.x || coords.y >= push_constants.scren_size.y) {
        return;
    }

    let input = visbuffer[coords];
    // Value 0 for InstanceID indicates no instance at this pixel.
    if (input.x == 0) {
        output[coords] = Vec4(0.0);
        return;
    }

    let instance_id = input.x - 1;
    let triangle_id = input.y;

    let instance = instances[instance_id];
    let offsets = offsets[instance.offsets_index];
    let material = materials[instance.material_index];

    let index0 = index_buffer[instance.index_offset + triangle_id * 3 + 0];
    let index1 = index_buffer[instance.index_offset + triangle_id * 3 + 1];
    let index2 = index_buffer[instance.index_offset + triangle_id * 3 + 2];
    let indices: u32[3] = {index0, index1, index2};

    let pos_0 = fetch<3>(positions, offsets.position, index0);
    let pos_1 = fetch<3>(positions, offsets.position, index1);
    let pos_2 = fetch<3>(positions, offsets.position, index2);

    let world_pos_a = mul(instance.transform, Vec4(pos_0, 1.0));
    let world_pos_b = mul(instance.transform, Vec4(pos_1, 1.0));
    let world_pos_c = mul(instance.transform, Vec4(pos_2, 1.0));

    let pos_a = mul(push_constants.camera.view_proj, world_pos_a);
    let pos_b = mul(push_constants.camera.view_proj, world_pos_b);
    let pos_c = mul(push_constants.camera.view_proj, world_pos_c);

    let ndc_x = (f32(coords.x) / f32(push_constants.scren_size.x)) * 2.0 - 1.0;
    let ndc_y = (f32(coords.y) / f32(push_constants.scren_size.y)) * -2.0 + 1.0;
    let ndc = Vec2(ndc_x, ndc_y);

    let barys = Barycentrics(pos_a, pos_b, pos_c, ndc, push_constants.scren_size);

    let world_position = barys.interpolate(world_pos_a.xyz, world_pos_b.xyz, world_pos_c.xyz);

    let uv = load_uv(barys, offsets, indices);
    let normal = load_normal(barys, offsets, indices, material, uv, instance);

    let vert_color_a = fetch<4>(colors, offsets.color, index0);
    let vert_color_b = fetch<4>(colors, offsets.color, index0);
    let vert_color_c = fetch<4>(colors, offsets.color, index0);
    let vert_color = barys.interpolate(vert_color_a, vert_color_b, vert_color_c);

    let base_color = material.base_color * sample_material_texture(material.base_color_texture, uv) * vert_color;

    if ((material.flags & MATERIAL_FLAGS_UNLIT) != 0) {
        output[coords] = Vec4(base_color.rgb, 1.0);
        return;
    }

    let specular_strength = material.specular_strength * sample_material_texture(material.specular_glossiness_texture, uv).r;
    let specular_color = material.specular_color.rgb;

    var roughness = material.roughness * sample_material_texture(material.metallic_roughness_texture, uv).g;
    if ((material.flags & MATERIAL_FLAGS_ROUGHNESS_FROM_GLOSSINESS) != 0) {
        let glossiness = (1.0 - material.roughness) * sample_material_texture(material.specular_glossiness_texture, uv).g;
        roughness = 1.0 - glossiness;
    }

    var metallic = material.metallic * sample_material_texture(material.metallic_roughness_texture, uv).b;
    if ((material.flags & MATERIAL_FLAGS_METALLIC_FROM_SPECULAR) != 0) {
        metallic = compute_metallic_from_specular_color(specular_color * specular_strength);
    }

    // BRDF parameters
    let view_dir = normalize(push_constants.camera.position - world_position);
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
        let light = point_lights[i].eval(FragParams(world_position));
        luminance += pbr::surface_shading(pbr_material, view_dir, light);
    }

    for (var i = 0; i < push_constants.num_spot_lights; i++) {
        let light = spot_lights[i].eval(FragParams(world_position));
        luminance += pbr::surface_shading(pbr_material, view_dir, light);
    }

    output[coords] = Vec4(luminance, 1.0);
}

struct InputParams {
    Barycentrics barys;
    Material material;
    InstanceOut instance;
    MeshOffsets offsets;
}

func fetch<let N: i32>(buffer: StructuredBuffer<f32>, offset: u32, index: u32) -> vector<f32, N> {
    var value = vector<f32, N>(0.0);
    for (var i = 0; i < N; i++) {
        value[i] = buffer[offset + index * N + i];
    }
    return value;
}

Gradient load_uv(Barycentrics barys, MeshOffsets offsets, u32 indices[3]) {
    let uv_a = fetch<2>(uvs, offsets.uv, indices[0]);
    let uv_b = fetch<2>(uvs, offsets.uv, indices[1]);
    let uv_c = fetch<2>(uvs, offsets.uv, indices[2]);
    return barys.interpolate_grad(uv_a, uv_b, uv_c);
}

Vec3 load_normal(Barycentrics barys, MeshOffsets offsets, u32 indices[3], Material material, Gradient uv, InstanceOut instance) {
    let normal_a = fetch<3>(normals, offsets.normal, indices[0]);
    let normal_b = fetch<3>(normals, offsets.normal, indices[1]);
    let normal_c = fetch<3>(normals, offsets.normal, indices[2]);
    var vert_normal = barys.interpolate(normal_a, normal_b, normal_c);
    vert_normal = normalize(mul(instance.normal, vert_normal));

    let tangent_a = fetch<4>(tangents, offsets.tangent, indices[0]);
    let tangent_b = fetch<4>(tangents, offsets.tangent, indices[1]);
    let tangent_c = fetch<4>(tangents, offsets.tangent, indices[2]);
    var tangent = barys.interpolate(tangent_a.xyz, tangent_b.xyz, tangent_c.xyz);
    tangent = normalize(mul(instance.normal, tangent));
    // Tangent sign should be consistent across all vertices.
    let tangent_sign = tangent_a.w;
    let bitangent = cross(vert_normal, tangent) * tangent_sign;

    let tbn = Mat3(tangent, bitangent, vert_normal);


    var normal = sample_material_texture(material.normal_texture, uv).xyz;

    // DirectX normal maps have the Y component flipped.
    if ((material.flags & MATERIAL_FLAGS_FLIP_NORMAL_Y) != 0) {
        normal.y = 1.0 - normal.y;
    }

    // Transform from [0..1] to [-1..1].
    normal = normal * 2.0 - 1.0;

    if ((material.flags & MATERIAL_FLAGS_NORMAL_ENCODING_TWO_COMPONENT) != 0) {
        normal.z = sqrt(clamp(1.0 - dot(normal.xy, normal.xy), 0.0, 1.0));
    }

    normal = normalize(normal);
    normal = normalize(mul(normal, tbn));

    return normal;
}


func compute_metallic_from_specular_color(specular_color: Vec3) -> f32 {
    return max(max(specular_color.r, specular_color.g), specular_color.b);
}

func sample_material_texture(texture_index: u32, uv: Gradient) -> Vec4 {
    // We need to mark the index as non-uniform because a single
    // workgroup may access divergent textures here.
    return textures[NonUniformResourceIndex(texture_index)].SampleGrad(sampler, uv.value, uv.dx, uv.dy);
}
