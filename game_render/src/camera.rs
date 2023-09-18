use std::f32::consts::PI;

use bytemuck::{Pod, Zeroable};
use game_common::components::transform::Transform;
use game_common::math::Ray;
use game_tracing::trace_span;
use game_window::windows::WindowId;
use glam::{Mat4, UVec2, Vec2, Vec3};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferUsages, Device};

#[derive(Clone, Debug)]
pub struct Camera {
    pub transform: Transform,
    pub projection: Projection,
    pub target: RenderTarget,
}

impl Camera {
    pub fn viewport_to_world(
        &self,
        camera_transform: Transform,
        viewport_size: Vec2,
        mut viewport_position: Vec2,
    ) -> Ray {
        viewport_position.y = viewport_size.y - viewport_position.y;
        let ndc = viewport_position * 2.0 / viewport_size - Vec2::ONE;

        let proj_matrix = self.projection.projection_matrix();

        let ndc_to_world = camera_transform.compute_matrix() * proj_matrix.inverse();
        let world_near_plane = ndc_to_world.project_point3(ndc.extend(1.0));
        // EPS instead of 0, otherwise we get NaNs.
        let world_far_plane = ndc_to_world.project_point3(ndc.extend(f32::EPSILON));

        Ray {
            origin: world_near_plane,
            direction: (world_far_plane - world_near_plane).normalize(),
        }
    }

    pub(crate) fn update_aspect_ratio(&mut self, size: UVec2) {
        self.projection.aspect_ratio = size.x as f32 / size.y as f32;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderTarget {
    /// Render to a window surface.
    Window(WindowId),
    // TODO: Add a render-to-texture target.
}

/// Perspective camera projection paramters
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Projection {
    pub aspect_ratio: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Projection {
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            aspect_ratio: 1.0,
            fov: PI / 4.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

pub const OPENGL_TO_WGPU: Mat4 = Mat4::from_cols_array_2d(&[
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 0.5, 0.0],
    [0.0, 0.0, 0.5, 1.0],
]);

#[derive(Debug)]
pub struct CameraBuffer {
    pub transform: Transform,
    pub projection: Projection,
    pub buffer: Buffer,
    pub target: RenderTarget,
}

impl CameraBuffer {
    pub fn new(
        transform: Transform,
        projection: Projection,
        device: &Device,
        target: RenderTarget,
    ) -> Self {
        let _span = trace_span!("CameraBuffer::new").entered();

        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("camera_transform_buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform::new(transform, projection)]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self {
            transform,
            projection,
            buffer,
            target,
        }
    }
}

#[derive(Copy, Clone, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct CameraUniform {
    // We only need `[f32; 3]`, but one word for alignment.
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(transform: Transform, projection: Projection) -> Self {
        let view = Mat4::look_to_rh(
            transform.translation,
            transform.rotation * -Vec3::Z,
            transform.rotation * Vec3::Y,
        );

        let proj = projection.projection_matrix();

        Self {
            view_position: [
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
                0.0,
            ],
            view_proj: (OPENGL_TO_WGPU * proj * view).to_cols_array_2d(),
        }
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new(Transform::default(), Projection::default())
    }
}
