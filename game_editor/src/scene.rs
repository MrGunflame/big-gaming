use ahash::HashMap;
use game_common::components::PrimaryCamera;
use game_common::components::{
    DirectionalLight as DirectionalLightComponent, GlobalTransform, MeshInstance,
    PointLight as PointLightComponent, SpotLight as SpotLightComponent,
};
use game_common::entity::EntityId;
use game_common::world::{QueryWrapper, World};
use game_core::modules::Modules;
use game_gizmos::Gizmos;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::entities::{CameraId, DirectionalLightId, PointLightId, SpotLightId};
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::scene::RendererScene;
use game_render::Renderer;
use game_scene::debug::draw_collider_lines;
use game_scene::{InstanceId, SceneId, SceneSpawner};
use game_tasks::TaskPool;
use game_wasm::resource::ResourceId;
use game_window::windows::WindowId;

use crate::state::record::Records;

#[derive(Debug, Default)]
pub struct SceneEntities {
    resource_to_scene: HashMap<ResourceId, SceneState>,
    scene_to_resource: HashMap<SceneId, ResourceId>,
    mesh_instances: HashMap<EntityId, InstantiatedMeshInstance>,
    directional_lights: HashMap<EntityId, DirectionalLightId>,
    point_lights: HashMap<EntityId, PointLightId>,
    spot_lights: HashMap<EntityId, SpotLightId>,
    primary_cameras: HashMap<EntityId, CameraId>,
    spawner: SceneSpawner,
}

impl SceneEntities {
    pub fn update(
        &mut self,
        records: &Records,
        world: &World,
        pool: &TaskPool,
        renderer: &mut Renderer,
        scene_id: game_render::entities::SceneId,
        render_target: RenderTarget,
        gizmos: &Gizmos,
    ) {
        let mut removed_mesh_instances = self.mesh_instances.clone();
        let mut removed_dir_lights = self.directional_lights.clone();
        let mut removed_point_lights = self.point_lights.clone();
        let mut removed_spot_lights = self.spot_lights.clone();
        let mut removed_primary_cameras = self.primary_cameras.clone();

        for (entity, QueryWrapper((GlobalTransform(transform), mesh_instance))) in
            world.query::<QueryWrapper<(GlobalTransform, MeshInstance)>>()
        {
            removed_mesh_instances.remove(&entity);

            match self.mesh_instances.get(&entity) {
                Some(instance) => {
                    // If the instance has changed we must reload the model.
                    // This is currently done by destroying the mesh instance
                    // and then letting it become a new entity in the next
                    // frame.
                    // FIXME: We could also for once do the smart thing and
                    // immediately unload the old model and load the new model
                    // in the current frame.
                    if mesh_instance.model != instance.model {
                        removed_mesh_instances.insert(entity, *instance);
                        continue;
                    }

                    if let Some(id) = instance.instance {
                        self.spawner.set_transform(id, transform);
                    }
                }
                None => match self.resource_to_scene.get_mut(&mesh_instance.model) {
                    Some(state) => {
                        state.instances += 1;
                        let instance = self.spawner.spawn(state.id);
                        self.mesh_instances.insert(
                            entity,
                            InstantiatedMeshInstance {
                                instance: Some(instance),
                                model: mesh_instance.model,
                            },
                        );
                    }
                    None => {
                        let instance = match load_resource(mesh_instance.model, records, world) {
                            Some(data) => {
                                let scene = self.spawner.insert(&data);
                                let instance = self.spawner.spawn(scene);

                                self.resource_to_scene.insert(
                                    mesh_instance.model,
                                    SceneState {
                                        id: scene,
                                        instances: 1,
                                        resource: mesh_instance.model,
                                    },
                                );
                                self.scene_to_resource.insert(scene, mesh_instance.model);

                                Some(instance)
                            }
                            None => None,
                        };

                        self.mesh_instances.insert(
                            entity,
                            InstantiatedMeshInstance {
                                instance,
                                model: mesh_instance.model,
                            },
                        );
                    }
                },
            }
        }

        for (entity, QueryWrapper((GlobalTransform(transform), light))) in
            world.query::<QueryWrapper<(GlobalTransform, DirectionalLightComponent)>>()
        {
            removed_dir_lights.remove(&entity);

            if let Some(id) = self.directional_lights.remove(&entity) {
                renderer.resources().directional_lights().remove(id);
            }

            let id = renderer
                .resources()
                .directional_lights()
                .insert(DirectionalLight {
                    transform,
                    scene: scene_id,
                    color: light.color,
                    illuminance: light.illuminance,
                });

            self.directional_lights.insert(entity, id);
        }

        for (entity, QueryWrapper((GlobalTransform(transform), light))) in
            world.query::<QueryWrapper<(GlobalTransform, PointLightComponent)>>()
        {
            removed_point_lights.remove(&entity);

            if let Some(id) = self.point_lights.remove(&entity) {
                renderer.resources().point_lights().remove(id);
            }

            let id = renderer.resources().point_lights().insert(PointLight {
                transform,
                scene: scene_id,
                color: light.color,
                intensity: light.intensity,
                radius: light.radius,
            });

            self.point_lights.insert(entity, id);
        }

        for (entity, QueryWrapper((GlobalTransform(transform), light))) in
            world.query::<QueryWrapper<(GlobalTransform, SpotLightComponent)>>()
        {
            removed_spot_lights.remove(&entity);

            if let Some(id) = self.spot_lights.remove(&entity) {
                renderer.resources().spot_lights().remove(id);
            }

            let id = renderer.resources().spot_lights().insert(SpotLight {
                transform,
                scene: scene_id,
                color: light.color,
                intensity: light.intensity,
                radius: light.radius,
                inner_cutoff: light.inner_cutoff,
                outer_cutoff: light.outer_cutoff,
            });

            self.spot_lights.insert(entity, id);
        }

        for (entity, QueryWrapper((GlobalTransform(transform), PrimaryCamera))) in
            world.query::<QueryWrapper<(GlobalTransform, PrimaryCamera)>>()
        {
            removed_primary_cameras.remove(&entity);

            if let Some(id) = self.primary_cameras.remove(&entity) {
                renderer.resources().cameras().remove(id);
            }

            // Surface might not yet be ready, defer creation until
            // next frame.
            let Some(size) = renderer.get_surface_size(render_target) else {
                continue;
            };

            let mut camera = Camera {
                transform,
                projection: Projection::default(),
                target: render_target,
                scene: scene_id,
            };
            camera.update_aspect_ratio(size);

            gizmos.update_camera(camera);

            let id = renderer.resources().cameras().insert(camera);
            self.primary_cameras.insert(entity, id);
        }

        for (entity, instance) in removed_mesh_instances {
            self.mesh_instances.remove(&entity);

            let Some(id) = instance.instance else {
                continue;
            };

            let scene = self.spawner.scene_of_instance(id);

            self.spawner.despawn(id);

            let path = self.scene_to_resource.get(&scene).unwrap();
            let scene = self.resource_to_scene.get_mut(path).unwrap();
            scene.instances -= 1;
            if scene.instances == 0 {
                self.spawner.remove(scene.id);
                self.scene_to_resource.remove(&scene.id);
                let res = scene.resource;
                self.resource_to_scene.remove(&res);
            }
        }

        for (entity, id) in removed_dir_lights {
            renderer.resources().directional_lights().remove(id);
            self.directional_lights.remove(&entity);
        }

        for (entity, id) in removed_point_lights {
            renderer.resources().point_lights().remove(id);
            self.point_lights.remove(&entity);
        }

        for (entity, id) in removed_spot_lights {
            renderer.resources().spot_lights().remove(id);
            self.spot_lights.remove(&entity);
        }

        for (entity, id) in removed_primary_cameras {
            renderer.resources().cameras().remove(id);
            self.primary_cameras.remove(&entity);
        }

        self.spawner.update(pool, renderer, scene_id);

        draw_collider_lines(world, gizmos);
    }
}

#[derive(Clone, Debug)]
struct SceneState {
    /// `None` if the scene refers to an invalid resource.
    id: SceneId,
    instances: u64,
    resource: ResourceId,
}

fn load_resource<'a>(id: ResourceId, records: &'a Records, world: &'a World) -> Option<Vec<u8>> {
    match id {
        ResourceId::Record(id) => {
            let record = records.get(id.0.module, id.0.record)?;
            Some(record.data)
        }
        ResourceId::Runtime(id) => world.get_resource(id).map(|v| v.to_vec()),
    }
}

#[derive(Copy, Clone, Debug)]
struct InstantiatedMeshInstance {
    instance: Option<InstanceId>,
    model: ResourceId,
}
