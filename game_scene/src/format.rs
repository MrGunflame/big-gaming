// TODO: We might want to eventaully merge this with our custom
// model format.

use std::collections::{HashMap, HashSet};
use std::str::SplitWhitespace;

use game_common::components::transform::Transform;
use game_core::hierarchy::Hierarchy;
use game_render::Renderer;
use glam::{Quat, Vec3};
use serde::Deserialize;

use crate::scene::Scene;
use crate::scene2;

#[derive(Clone, Debug, Deserialize)]
pub struct SceneRoot {
    nodes: Vec<Node>,
}

impl SceneRoot {
    pub fn spawn(self, renderer: &mut Renderer) {}
}

#[derive(Clone, Debug, Deserialize)]
struct Node {
    parent: Option<usize>,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
    components: Vec<Component>,
}

#[derive(Clone, Debug, Deserialize)]
enum Component {
    MeshInstance(MeshInstance),
    DirectionalLight(DirectionalLight),
    PointLight(PointLight),
    SpotLight(SpotLight),
}

#[derive(Clone, Debug, Deserialize)]
struct MeshInstance {
    path: String,
}

#[derive(Copy, Clone, Debug, Deserialize)]
struct DirectionalLight {
    color: [f32; 3],
    illuminance: f32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
struct PointLight {
    color: [f32; 3],
    intensity: f32,
    radius: f32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
struct SpotLight {
    color: [f32; 3],
    intensity: f32,
    radius: f32,
    inner_cutoff: f32,
    outer_cutoff: f32,
}

pub fn from_slice(buf: &[u8]) -> Result<SceneRoot, Box<dyn std::error::Error>> {
    let root: SceneRoot = serde_json::from_slice(buf)?;

    Ok(root)
}
