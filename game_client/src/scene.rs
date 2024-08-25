use ahash::HashMap;
use game_common::components::PrimaryCamera;
use game_common::components::{
    DirectionalLight as DirectionalLightComponent, GlobalTransform, MeshInstance,
    PointLight as PointLightComponent, SpotLight as SpotLightComponent,
};
use game_common::entity::EntityId;
use game_common::world::{QueryWrapper, World};
use game_core::debug::draw_collider_lines;
use game_gizmos::Gizmos;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::entities::{CameraId, DirectionalLightId, PointLightId, SpotLightId};
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::Renderer;
use game_scene::{InstanceId, SceneId, SceneSpawner};
use game_tasks::TaskPool;
use game_window::windows::WindowId;

#[derive(Debug, Default)]
pub struct SceneEntities {
    path_to_scene: HashMap<String, SceneState>,
    scene_to_path: HashMap<SceneId, String>,
    mesh_instances: HashMap<EntityId, InstanceId>,
    directional_lights: HashMap<EntityId, DirectionalLightId>,
    point_lights: HashMap<EntityId, PointLightId>,
    spot_lights: HashMap<EntityId, SpotLightId>,
    primary_cameras: HashMap<EntityId, CameraId>,
    spawner: SceneSpawner,
}

impl SceneEntities {
    pub fn update(
        &mut self,
        world: &World,
        pool: &TaskPool,
        renderer: &mut Renderer,
        window: WindowId,
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
                Some(id) => {
                    self.spawner.set_transform(*id, transform);
                }
                None => match self.path_to_scene.get_mut(&mesh_instance.path) {
                    Some(state) => {
                        state.instances += 1;
                        let instance = self.spawner.spawn(state.id);
                        self.mesh_instances.insert(entity, instance);
                    }
                    None => {
                        let scene = self.spawner.insert_from_file(&mesh_instance.path);
                        let instance = self.spawner.spawn(scene);
                        self.path_to_scene.insert(
                            mesh_instance.path.clone(),
                            SceneState {
                                id: scene,
                                instances: 1,
                                path: mesh_instance.path.clone(),
                            },
                        );
                        self.scene_to_path.insert(scene, mesh_instance.path);
                        self.mesh_instances.insert(entity, instance);
                    }
                },
            }
        }

        for (entity, QueryWrapper((GlobalTransform(transform), light))) in
            world.query::<QueryWrapper<(GlobalTransform, DirectionalLightComponent)>>()
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
                    let dir_light = DirectionalLight {
                        color: light.color,
                        illuminance: light.illuminance,
                        transform,
                    };

                    let id = renderer.entities.directional_lights.insert(dir_light);
                    self.directional_lights.insert(entity, id);
                }
            }
        }

        for (entity, QueryWrapper((GlobalTransform(transform), light))) in
            world.query::<QueryWrapper<(GlobalTransform, PointLightComponent)>>()
        {
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
                    let point_light = PointLight {
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

        for (entity, QueryWrapper((GlobalTransform(transform), light))) in
            world.query::<QueryWrapper<(GlobalTransform, SpotLightComponent)>>()
        {
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
                    let spot_light = SpotLight {
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

        for (entity, QueryWrapper((GlobalTransform(transform), PrimaryCamera))) in
            world.query::<QueryWrapper<(GlobalTransform, PrimaryCamera)>>()
        {
            removed_primary_cameras.remove(&entity);

            match self.primary_cameras.get(&entity) {
                Some(id) => {
                    let mut camera = renderer.entities.cameras.get_mut(*id).unwrap();
                    camera.transform = transform;
                    gizmos.update_camera(*camera);
                }
                None => {
                    // Surface might not yet be ready, defer creation until
                    // next frame.
                    let Some(size) = renderer.get_surface_size(window) else {
                        continue;
                    };

                    let mut camera = Camera {
                        transform,
                        projection: Projection::default(),
                        target: RenderTarget::Window(window),
                    };
                    camera.update_aspect_ratio(size);

                    gizmos.update_camera(camera);

                    let id = renderer.entities.cameras.insert(camera);
                    self.primary_cameras.insert(entity, id);
                }
            }
        }

        for (entity, id) in removed_mesh_instances {
            let scene = self.spawner.scene_of_instance(id);

            self.spawner.despawn(id);
            self.mesh_instances.remove(&entity);

            let path = self.scene_to_path.get(&scene).unwrap();
            let scene = self.path_to_scene.get_mut(path).unwrap();
            scene.instances -= 1;
            if scene.instances == 0 {
                self.spawner.remove(scene.id);
                self.scene_to_path.remove(&scene.id);
                let path = scene.path.clone();
                self.path_to_scene.remove(&path);
            }
        }

        for (entity, id) in removed_dir_lights {
            renderer.entities.directional_lights.remove(id);
            self.directional_lights.remove(&entity);
        }

        for (entity, id) in removed_point_lights {
            renderer.entities.point_lights.remove(id);
            self.point_lights.remove(&entity);
        }

        for (entity, id) in removed_spot_lights {
            renderer.entities.spot_lights.remove(id);
            self.spot_lights.remove(&entity);
        }

        for (entity, id) in removed_primary_cameras {
            renderer.entities.cameras.remove(id);
            self.primary_cameras.remove(&entity);
        }

        self.spawner.update(pool, renderer);

        draw_collider_lines(world, gizmos);
    }
}

#[derive(Clone, Debug)]
struct SceneState {
    id: SceneId,
    instances: u64,
    path: String,
}
