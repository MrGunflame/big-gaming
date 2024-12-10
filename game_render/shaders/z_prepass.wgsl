struct PushConstants {
    camera: Camera,
}

struct Camera {
    position: vec3<f32>,
    view_proj: mat4x4<f32>,
}

var <push_constant> push_constants: PushConstants;
