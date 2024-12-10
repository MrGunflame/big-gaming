use std::f32::consts::PI;

use bytemuck::{Pod, Zeroable};
use game_common::components::Transform;
use game_common::math::Ray;
use game_window::windows::WindowId;
use glam::{Mat4, UVec2, Vec2, Vec3};

use crate::entities::SceneId;
use crate::texture::RenderImageId;

#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub transform: Transform,
    pub projection: Projection,
    pub target: RenderTarget,
    /// The scene that should be rendered with this camera.
    pub scene: SceneId,
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

    pub fn update_aspect_ratio(&mut self, size: UVec2) {
        self.projection.aspect_ratio = size.x as f32 / size.y as f32;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    /// Render to a window surface.
    Window(WindowId),
    /// Render to a GPU internal texture.
    Image(RenderImageId),
}

impl RenderTarget {
    /// Returns `true` if this `RenderTarget` is a `Window`.
    #[inline]
    pub const fn is_window(&self) -> bool {
        matches!(self, Self::Window(_))
    }

    /// Returns `true` if this `RenderTarget` is a `Image`.
    #[inline]
    pub const fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    /// Returns the underlying [`WindowId`] or `None` if this `RenderTarget` is not `Window`.
    #[inline]
    pub const fn as_window(&self) -> Option<&WindowId> {
        match self {
            Self::Window(window) => Some(window),
            Self::Image(_) => None,
        }
    }

    /// Returns the underlying [`RenderImageId`] or `None` if this `RenderTarget` is not `Image`.
    #[inline]
    pub const fn as_image(&self) -> Option<&RenderImageId> {
        match self {
            Self::Image(image) => Some(image),
            Self::Window(_) => None,
        }
    }
}

impl From<WindowId> for RenderTarget {
    #[inline]
    fn from(value: WindowId) -> Self {
        Self::Window(value)
    }
}

impl From<RenderImageId> for RenderTarget {
    #[inline]
    fn from(value: RenderImageId) -> Self {
        Self::Image(value)
    }
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
