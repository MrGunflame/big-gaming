use game_gltf::GltfData;
use game_render::pbr::AlphaMode;

const ALPHA_MODE: AlphaMode = AlphaMode::Opaque;

const BASE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BASE_COLOR_TEXTURE: Option<&[u8]> = Some(include_bytes!("test_texture.png"));

const METALLIC: f32 = 0.0;
const ROUGHNESS: f32 = 0.5;
const METALLIC_ROUGHNESS_TEXTURE: Option<&[u8]> = None;

#[test]
fn basic_material_gltf() {
    let data = GltfData::open("./tests/basic_material/basic_material.gltf").unwrap();
    validate_meshes(&data);
}

#[test]
fn basic_material_glb() {
    let data = GltfData::open("./tests/basic_material/basic_material.glb").unwrap();
    validate_meshes(&data);
}

#[test]
fn basic_material_gltf_embedded() {
    let data = GltfData::open("./tests/basic_material/basic_material_embedded.gltf").unwrap();
    validate_meshes(&data);
}

fn validate_meshes(data: &GltfData) {
    let meshes = data.meshes().unwrap();

    let material = &meshes[0].1;
    assert_eq!(material.alpha_mode, ALPHA_MODE);
    assert_eq!(material.base_color, BASE_COLOR);
    assert_eq!(
        material.base_color_texture.as_ref().map(|buf| buf.as_ref()),
        BASE_COLOR_TEXTURE
    );
    assert_eq!(material.metallic, METALLIC);
    assert_eq!(material.roughness, ROUGHNESS);
    assert_eq!(
        material
            .metallic_roughness_texture
            .as_ref()
            .map(|buf| buf.as_ref()),
        METALLIC_ROUGHNESS_TEXTURE
    );
}
