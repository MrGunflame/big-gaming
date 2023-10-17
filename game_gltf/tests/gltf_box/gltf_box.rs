use game_gltf::GltfData;
use glam::Vec3;

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

const INDICES: &[u32] = &[
    1, 14, 20, 1, 20, 7, 10, 6, 18, 10, 18, 22, 23, 19, 12, 23, 12, 16, 15, 3, 9, 15, 9, 21, 5, 2,
    8, 5, 8, 11, 17, 13, 0, 17, 0, 4,
];

#[test]
fn gltf_box_gltf() {
    let data = GltfData::from_file("./tests/gltf_box/gltf_box.gltf").unwrap();
    validate_output(&data);
}

#[test]
fn test_box_gltf_embedded() {
    let data = GltfData::from_file("./tests/gltf_box/gltf_box_embedded.gltf").unwrap();
    validate_output(&data);
}

#[test]
fn gltf_box_glb() {
    let data = GltfData::from_file("./tests/gltf_box/gltf_box.glb").unwrap();
    validate_output(&data);
}

fn validate_output(data: &GltfData) {
    assert_eq!(data.scenes.len(), 1);

    let nodes = &data.scenes[0].nodes;
    assert_eq!(nodes.len(), 1);

    let node = nodes.values().nth(0).unwrap();

    let mesh = &data.meshes[&node.mesh];

    // TODO: Move cmp to Vec3.
    assert_eq!(
        mesh.positions
            .iter()
            .map(Vec3::to_array)
            .collect::<Vec<_>>(),
        POSITIONS
    );
    assert_eq!(mesh.indices, INDICES);
}
