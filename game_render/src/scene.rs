use glam::UVec2;

use crate::entities::SceneEntities;
use crate::options::MainPassOptions;
use crate::pbr::material::Materials;
use crate::pbr::mesh::Meshes;
use crate::state::Event;
use crate::texture::Images;

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub entities: SceneEntities,
    options: MainPassOptions,
    pub(super) events: Vec<Event>,
}

impl Scene {
    pub fn options(&self) -> &MainPassOptions {
        &self.options
    }

    pub fn set_options(&mut self, options: MainPassOptions) {
        self.options = options.clone();
        self.events.push(Event::UpdateMainPassOptions(options));
    }
}

#[non_exhaustive]
pub struct RendererScene<'a> {
    pub scene: &'a mut Scene,
    pub meshes: &'a mut Meshes,
    pub images: &'a mut Images,
    pub materials: &'a mut Materials,
    pub size: UVec2,
}

impl<'a> RendererScene<'a> {
    pub fn get_surface_size(&self) -> UVec2 {
        self.size
    }
}
