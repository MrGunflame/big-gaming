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
    validate_output(&data);
}

#[test]
fn test_box_gltf_embedded() {
    let data = GltfData::open("./tests/gltf_box/gltf_box_embedded.gltf").unwrap();
    validate_output(&data);
}

#[test]
fn gltf_box_glb() {
    let data = GltfData::open("./tests/gltf_box/gltf_box.glb").unwrap();
    validate_output(&data);
}

fn validate_output(data: &GltfData) {
    let scenes = data.scenes().unwrap();
    assert_eq!(scenes.len(), 1);

    let nodes = &scenes[0].nodes;
    assert_eq!(nodes.len(), 1);

    let primitives = &nodes[0].mesh.as_ref().unwrap().primitives;
    assert_eq!(primitives.len(), 1);

    let mesh = &primitives[0].mesh;

    assert_eq!(mesh.positions(), POSITIONS);
    assert_eq!(mesh.indicies().unwrap().as_u16(), INDICES);
}
