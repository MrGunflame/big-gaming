use game_gltf::GltfData;
use glam::Vec3;

#[test]
fn nested_nodes_gltf() {
    let data = GltfData::from_file("./tests/nested_nodes/nested_nodes.gltf").unwrap();
    validate_output(&data);
}

#[test]
fn nested_nodes_gltf_embedded() {
    let data = GltfData::from_file("./tests/nested_nodes/nested_nodes_embedded.gltf").unwrap();
    validate_output(&data);
}

#[test]
fn nested_nodes_glb() {
    let data = GltfData::from_file("./tests/nested_nodes/nested_nodes.glb").unwrap();
    validate_output(&data);
}

fn validate_output(data: &GltfData) {
    assert_eq!(data.scenes.len(), 1);

    let nodes = &data.scenes[0].nodes;
    assert_eq!(nodes.len(), 4);

    let root = nodes
        .iter()
        .filter_map(|(key, _)| {
            if nodes.parent(key).is_none() {
                Some(key)
            } else {
                None
            }
        })
        .nth(0)
        .unwrap();

    let first_children = nodes.children(root).unwrap();

    let mut next_key = None;
    // Manually implement indexing instead of `enumerate()` so we can
    // assert that we visited the exact number elements we expect.
    let mut index = 0;
    for (key, node) in first_children {
        let curr_index = index;
        index += 1;

        match curr_index {
            0 => {
                assert_eq!(node.transform.translation, Vec3::new(1.0, 2.0, 3.0));
            }
            1 => {
                next_key = Some(key);
                break;
            }
            _ => unreachable!(),
        }
    }

    assert_eq!(index, 2);
    index = 0;

    let next_children = nodes.children(next_key.unwrap()).unwrap();
    for (_, node) in next_children {
        let curr_index = index;
        index += 1;

        match curr_index {
            0 => {
                assert_eq!(node.transform.translation, Vec3::new(1.0, 2.0, 3.0));
            }
            _ => unreachable!(),
        }
    }

    assert_eq!(index, 1);
}
