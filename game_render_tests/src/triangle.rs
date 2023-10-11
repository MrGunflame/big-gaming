use game_render::mesh::Mesh;

pub fn triangle() {
    let mut mesh = Mesh::new();
    mesh.set_positions(vec![[0.0, 0.5, 0.0], [-0.5, -0.5, 0.0], [0.5, -0.5, 0.0]]);
    mesh.set_normals(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]);
    mesh.compute_tangents();
}
