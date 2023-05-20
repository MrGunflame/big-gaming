use game_gltf::GltfData;

const POSITIONS: &[[f32; 3]] = &[
    [1.0, 1.0, -1.0],
    [1.0, 1.0, -1.0],
    [1.0, 1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, -1.0],
    [-1.0, -1.0, -1.0],
    [-1.0, -1.0, -1.0],
    [-1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
    [-1.0, -1.0, 1.0],
    [-1.0, -1.0, 1.0],
    [-1.0, -1.0, 1.0],
];

const INDICES: &[u16] = &[
    1, 14, 20, 1, 20, 7, 10, 6, 18, 10, 18, 22, 23, 19, 12, 23, 12, 16, 15, 3, 9, 15, 9, 21, 5, 2,
    8, 5, 8, 11, 17, 13, 0, 17, 0, 4,
];

#[test]
fn gltf_box_gltf() {
    let data = GltfData::open("./tests/gltf_box/gltf_box.gltf").unwrap();

    validate_meshes(&data);
}

#[test]
fn test_box_gltf_embedded() {
    let data = GltfData::open("./tests/gltf_box/gltf_box_embedded.gltf").unwrap();

    validate_meshes(&data);
}

#[test]
fn gltf_box_glb() {
    let data = GltfData::open("./tests/gltf_box/gltf_box.glb").unwrap();

    validate_meshes(&data);
}

fn validate_meshes(data: &GltfData) {
    let meshes = data.meshes().unwrap();
    assert_eq!(meshes.len(), 1);

    let mesh = &meshes[0].0;
    assert_eq!(mesh.positions(), POSITIONS);
    assert_eq!(mesh.indicies().unwrap().as_u16(), INDICES);
}
