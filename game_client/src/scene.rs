use ahash::HashMap;
use game_common::components::rendering::{
    DirectionalLight as DirectionalLightComponent, MeshInstance, PointLight as PointLightComponent,
    SpotLight as SpotLightComponent,
};
use game_common::components::transform::Transform;
use game_common::entity::EntityId;
use game_common::world::World;
use game_render::entities::{DirectionalLightId, ObjectId, PointLightId, SpotLightId};
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::Renderer;
use game_scene::scene2::{Node, SceneGraph};
use game_scene::SceneSpawner;
use game_tasks::TaskPool;

#[derive(Debug, Default)]
pub struct SceneEntities {
    mesh_instances: HashMap<EntityId, ObjectId>,
    directional_lights: HashMap<EntityId, DirectionalLightId>,
    point_lights: HashMap<EntityId, PointLightId>,
    spot_lights: HashMap<EntityId, SpotLightId>,
    graph: SceneGraph,
    spawner: SceneSpawner,
}

impl SceneEntities {
    pub fn update(&mut self, world: &World, pool: &TaskPool, renderer: &mut Renderer) {
        self.spawner.update(&mut self.graph, pool, Some(renderer));
        self.graph.compute_transform();
        self.graph.clear_trackers();

        let mut removed_mesh_instances = self.mesh_instances.clone();
        let mut removed_dir_lights = self.directional_lights.clone();
        let mut removed_point_lights = self.point_lights.clone();
        let mut removed_spot_lights = self.spot_lights.clone();

        for (entity, (transform, mesh_instance)) in world.query::<(Transform, MeshInstance)>() {
            removed_mesh_instances.remove(&entity);

            match self.mesh_instances.get(&entity) {
                Some(id) => {
                    let mut instance = renderer.entities.objects.get_mut(*id).unwrap();
                    instance.transform = transform;
                }
                None => {
                    let key = self
                        .graph
                        .append(None, Node::from_transform(Transform::default()));
                    self.spawner.spawn(key, mesh_instance.path);
                }
            }
        }

        for (entity, (transform, light)) in world.query::<(Transform, DirectionalLightComponent)>()
        {
            removed_dir_lights.remove(&entity);

            match self.directional_lights.get(&entity) {
                Some(id) => {
                    let mut dir_light = renderer.entities.directional_lights.get_mut(*id).unwrap();
                    dir_light.color = light.color;
                    dir_light.illuminance = light.illuminance;
                    dir_light.transform = transform;
                }
                None => {
                    let mut dir_light = DirectionalLight {
                        color: light.color,
                        illuminance: light.illuminance,
                        transform,
                    };

                    let id = renderer.entities.directional_lights.insert(dir_light);
                    self.directional_lights.insert(entity, id);
                }
            }
        }

        for (entity, (transform, light)) in world.query::<(Transform, PointLightComponent)>() {
            removed_point_lights.remove(&entity);

            match self.point_lights.get(&entity) {
                Some(id) => {
                    let mut point_light = renderer.entities.point_lights.get_mut(*id).unwrap();
                    point_light.color = light.color;
                    point_light.intensity = light.intensity;
                    point_light.radius = light.radius;
                    point_light.transform = transform;
                }
                None => {
                    let mut point_light = PointLight {
                        color: light.color,
                        intensity: light.intensity,
                        radius: light.radius,
                        transform,
                    };

                    let id = renderer.entities.point_lights.insert(point_light);
                    self.point_lights.insert(entity, id);
                }
            }
        }

        for (entity, (transform, light)) in world.query::<(Transform, SpotLightComponent)>() {
            removed_spot_lights.remove(&entity);

            match self.spot_lights.get(&entity) {
                Some(id) => {
                    let mut spot_light = renderer.entities.spot_lights.get_mut(*id).unwrap();
                    spot_light.color = light.color;
                    spot_light.intensity = light.intensity;
                    spot_light.radius = light.radius;
                    spot_light.inner_cutoff = light.inner_cutoff;
                    spot_light.outer_cutoff = light.outer_cutoff;
                    spot_light.transform = transform;
                }
                None => {
                    let mut spot_light = SpotLight {
                        color: light.color,
                        intensity: light.intensity,
                        radius: light.radius,
                        inner_cutoff: light.inner_cutoff,
                        outer_cutoff: light.outer_cutoff,
                        transform,
                    };

                    let id = renderer.entities.spot_lights.insert(spot_light);
                    self.spot_lights.insert(entity, id);
                }
            }
        }

        for (_, id) in removed_dir_lights {
            renderer.entities.directional_lights.remove(id);
        }

        for (_, id) in removed_point_lights {
            renderer.entities.point_lights.remove(id);
        }

        for (_, id) in removed_spot_lights {
            renderer.entities.spot_lights.remove(id);
        }
    }
}
