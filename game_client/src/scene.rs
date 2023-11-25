use ahash::HashMap;
use game_render::entities::{DirectionalLightId, Object, ObjectId, PointLightId, SpotLightId};
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::Renderer;
use game_scene::scene2::{Key, NodeBody, SceneGraph};
use game_scene::SceneSpawner;

pub struct SceneState {
    pub graph: SceneGraph,
    pub spawner: SceneSpawner,
}

#[derive(Clone, Debug, Default)]
pub struct SceneEntities {
    mesh_instances: HashMap<Key, ObjectId>,
    directional_lights: HashMap<Key, DirectionalLightId>,
    point_lights: HashMap<Key, PointLightId>,
    spot_lights: HashMap<Key, SpotLightId>,
}

impl SceneEntities {
    pub fn update(&mut self, graph: &mut SceneGraph, renderer: &mut Renderer) {
        // Remove removed node before adding new ones because they are
        // allowed to reuse the same keys.
        for key in graph.iter_removed() {
            if let Some(id) = self.mesh_instances.remove(&key) {
                renderer.entities.objects.remove(id);
            }

            if let Some(id) = self.directional_lights.remove(&key) {
                renderer.entities.directional_lights.remove(id);
            }

            if let Some(id) = self.point_lights.remove(&key) {
                renderer.entities.point_lights.remove(id);
            }

            if let Some(id) = self.spot_lights.remove(&key) {
                renderer.entities.spot_lights.remove(id);
            }
        }

        for key in graph.iter_added() {
            let node = graph.get(key).unwrap();
            match node.body {
                NodeBody::MeshInstance(instance) => {
                    renderer.entities.objects.insert(Object {
                        transform: node.transform,
                        mesh: instance.mesh,
                        material: instance.material,
                    });
                }
                NodeBody::DirectionalLight(light) => {
                    renderer
                        .entities
                        .directional_lights
                        .insert(DirectionalLight {
                            transform: node.transform,
                            color: light.color,
                            illuminance: light.illuminance,
                        });
                }
                NodeBody::PointLight(light) => {
                    renderer.entities.point_lights.insert(PointLight {
                        transform: node.transform,
                        color: light.color,
                        intensity: light.intensity,
                        radius: light.radius,
                    });
                }
                NodeBody::SpotLight(light) => {
                    renderer.entities.spot_lights.insert(SpotLight {
                        transform: node.transform,
                        color: light.color,
                        intensity: light.intensity,
                        radius: light.radius,
                        inner_cutoff: light.inner_cutoff,
                        outer_cutoff: light.outer_cutoff,
                    });
                }
                NodeBody::None | NodeBody::Collider(_) => (),
            }
        }

        for (key, transform) in graph.iter_changed_global_transform() {
            if let Some(id) = self.mesh_instances.get(&key) {
                let mut instance = renderer.entities.objects.get_mut(*id).unwrap();
                instance.transform = transform;
            }

            if let Some(id) = self.directional_lights.get(&key) {
                let mut light = renderer.entities.directional_lights.get_mut(*id).unwrap();
                light.transform = transform;
            }

            if let Some(id) = self.point_lights.get(&key) {
                let mut light = renderer.entities.point_lights.get_mut(*id).unwrap();
                light.transform = transform;
            }

            if let Some(id) = self.spot_lights.get(&key) {
                let mut light = renderer.entities.spot_lights.get_mut(*id).unwrap();
                light.transform = transform;
            }
        }
    }
}
