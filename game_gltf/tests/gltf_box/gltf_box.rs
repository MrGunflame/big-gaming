use game_gltf::GltfData;

#[test]
fn gltf_box_gltf() {
    GltfData::open("./tests/gltf_box/gltf_box.gltf").unwrap();
}

#[test]
fn test_box_gltf_embedded() {
    GltfData::open("./tests/gltf_box/gltf_box_embedded.gltf").unwrap();
}

#[test]
fn gltf_box_glb() {
    GltfData::open("./tests/gltf_box/gltf_box.glb").unwrap();
}
