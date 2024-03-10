use ahash::HashMap;
use game_common::components::{
    DirectionalLight as DirectionalLightComponent, MeshInstance, PointLight as PointLightComponent,
    SpotLight as SpotLightComponent,
};
use game_common::components::{PrimaryCamera, Transform};
use game_common::entity::EntityId;
use game_common::world::{QueryWrapper, World};
use game_gizmos::Gizmos;
use game_render::camera::{Camera, Projection, RenderTarget};
use game_render::entities::{CameraId, DirectionalLightId, PointLightId, SpotLightId};
use game_render::light::{DirectionalLight, PointLight, SpotLight};
use game_render::Renderer;
use game_scene::scene2::SceneGraph;
use game_scene::{SceneId, SceneSpawner};
use game_tasks::TaskPool;
use game_wasm::components::Component;
use game_window::windows::WindowId;

#[derive(Debug, Default)]
pub struct SceneEntities {
    mesh_instances: HashMap<EntityId, SceneId>,
    directional_lights: HashMap<EntityId, DirectionalLightId>,
    point_lights: HashMap<EntityId, PointLightId>,
    spot_lights: HashMap<EntityId, SpotLightId>,
    primary_cameras: HashMap<EntityId, CameraId>,
    graph: SceneGraph,
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
        self.spawner.update(pool, renderer);
        self.graph.compute_transform();
        self.graph.clear_trackers();

        let mut removed_mesh_instances = self.mesh_instances.clone();
        let mut removed_dir_lights = self.directional_lights.clone();
        let mut removed_point_lights = self.point_lights.clone();
        let mut removed_spot_lights = self.spot_lights.clone();
        let mut removed_primary_cameras = self.primary_cameras.clone();

        for (entity, QueryWrapper((transform, mesh_instance))) in
            world.query::<QueryWrapper<(Transform, MeshInstance)>>()
        {
            removed_mesh_instances.remove(&entity);

            match self.mesh_instances.get(&entity) {
                Some(id) => {
                    self.spawner.set_transform(renderer, transform, *id);
                }
                None => {
                    let id = self.spawner.spawn(mesh_instance.path);
                    self.mesh_instances.insert(entity, id);
                }
            }
        }

        for (entity, QueryWrapper((transform, light))) in
            world.query::<QueryWrapper<(Transform, DirectionalLightComponent)>>()
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

        for (entity, QueryWrapper((transform, light))) in
            world.query::<QueryWrapper<(Transform, PointLightComponent)>>()
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

        for (entity, QueryWrapper((transform, light))) in
            world.query::<QueryWrapper<(Transform, SpotLightComponent)>>()
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

        for (entity, QueryWrapper((transform, camera))) in
            world.query::<QueryWrapper<(Transform, PrimaryCamera)>>()
        {
            removed_primary_cameras.remove(&entity);

            match self.primary_cameras.get(&entity) {
                Some(id) => {
                    let mut camera = renderer.entities.cameras.get_mut(*id).unwrap();
                    camera.transform = transform;
                }
                None => {
                    let size = renderer.get_surface_size(window).unwrap();

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
            self.spawner.despawn(renderer, id);
            self.mesh_instances.remove(&entity);
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
    }
}
