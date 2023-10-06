use game_common::components::transform::Transform;
use game_render::color::Color;

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub body: NodeBody,
}

#[derive(Clone, Debug)]
pub enum NodeBody {
    Model(),
    DirectionalLight(DirectionalLight),
    PointLight(PointLight),
}

impl NodeBody {
    pub const fn kind(&self) -> NodeKind {
        match self {
            Self::Model() => NodeKind::Model,
            Self::DirectionalLight(_) => NodeKind::DirectionalLight,
            Self::PointLight(_) => NodeKind::PointLight,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Model,
    DirectionalLight,
    PointLight,
    SpotLight,
}

impl NodeKind {
    pub const fn default_name(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::DirectionalLight => "Directional Light",
            Self::PointLight => "Point Light",
            Self::SpotLight => "Spot Light",
        }
    }
}
