pub mod pool;

use game_common::cell::{RefMut, UnsafeRefCell};
use game_common::collections::arena::{Arena, Key};
use game_common::components::Transform;
use pool::{Pool, Writer};

use crate::camera::Camera;
use crate::light::{DirectionalLight, PointLight, SpotLight};
use crate::mesh::Mesh;
use crate::options::MainPassOptions;
use crate::pbr::PbrMaterial;
use crate::texture::Image;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DirectionalLightId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PointLightId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpotLightId(pub(crate) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(pub(crate) Key);

#[derive(Debug, Default)]
pub(crate) struct Resources {
    pub(crate) meshes: Pool<Mesh>,
    pub(crate) images: Pool<Image>,
    pub(crate) materials: Pool<PbrMaterial>,

    pub(crate) cameras: Pool<Camera>,
    pub(crate) objects: Pool<Object>,
    pub(crate) directional_lights: Pool<DirectionalLight>,
    pub(crate) point_lights: Pool<PointLight>,
    pub(crate) spot_lights: Pool<SpotLight>,
    pub(crate) scenes: UnsafeRefCell<Arena<()>>,
}

impl Resources {
    pub(crate) unsafe fn commit(&self) {
        unsafe {
            self.meshes.commit();
            self.images.commit();
            self.materials.commit();
            self.cameras.commit();
            self.objects.commit();
            self.directional_lights.commit();
            self.point_lights.commit();
            self.spot_lights.commit();
        }
    }
}

#[derive(Debug)]
pub struct ResourcesMut<'a> {
    resources: &'a Resources,
    events: &'a mut Vec<Event>,
}

impl<'a> ResourcesMut<'a> {
    /// # Safety
    ///
    /// Calling this function requires all [`Pool`]s in the given [`Resources`] to be writer-free.
    ///
    /// This also implies that no two instances of `ResourceMut` must ever exist with the same
    /// underlying [`Resources`] instance.
    pub(crate) unsafe fn new(resources: &'a Resources, events: &'a mut Vec<Event>) -> Self {
        Self { resources, events }
    }

    pub fn meshes(&mut self) -> MeshesMut<'_> {
        MeshesMut {
            meshes: unsafe { self.resources.meshes.writer() },
        }
    }

    pub fn images(&mut self) -> ImagesMut<'_> {
        ImagesMut {
            images: unsafe { self.resources.images.writer() },
        }
    }

    pub fn materials(&mut self) -> MaterialsMut<'_> {
        MaterialsMut {
            materials: unsafe { self.resources.materials.writer() },
        }
    }

    pub fn cameras(&mut self) -> CamerasMut<'_> {
        CamerasMut {
            cameras: unsafe { self.resources.cameras.writer() },
            events: &mut self.events,
        }
    }

    pub fn objects(&mut self) -> ObjectsMut<'_> {
        ObjectsMut {
            objects: unsafe { self.resources.objects.writer() },
            events: &mut self.events,
        }
    }

    pub fn directional_lights(&mut self) -> DirectionalLightsMut<'_> {
        DirectionalLightsMut {
            lights: unsafe { self.resources.directional_lights.writer() },
            events: &mut self.events,
        }
    }

    pub fn point_lights(&mut self) -> PointLightsMut<'_> {
        PointLightsMut {
            lights: unsafe { self.resources.point_lights.writer() },
            events: &mut self.events,
        }
    }

    pub fn spot_lights(&mut self) -> SpotLightsMut<'_> {
        SpotLightsMut {
            lights: unsafe { self.resources.spot_lights.writer() },
            events: &mut self.events,
        }
    }

    pub fn scenes(&mut self) -> ScenesMut<'_> {
        ScenesMut {
            scenes: unsafe { self.resources.scenes.borrow_mut() },
        }
    }

    pub fn set_main_pass_options(&mut self, options: MainPassOptions) {
        self.events.push(Event::UpdateMainPassOptions(options));
    }
}

#[derive(Debug)]
pub struct MeshesMut<'a> {
    meshes: Writer<'a, Mesh>,
}

impl<'a> MeshesMut<'a> {
    pub fn insert(&mut self, mesh: Mesh) -> MeshId {
        let id = self.meshes.insert(mesh);
        MeshId(id)
    }

    pub fn remove(&mut self, id: MeshId) {
        self.meshes.remove(id.0);
    }
}

#[derive(Debug)]
pub struct ImagesMut<'a> {
    images: Writer<'a, Image>,
}

impl<'a> ImagesMut<'a> {
    pub fn insert(&mut self, image: Image) -> ImageId {
        let id = self.images.insert(image);
        ImageId(id)
    }

    pub fn remove(&mut self, id: ImageId) {
        self.images.remove(id.0);
    }
}

#[derive(Debug)]
pub struct MaterialsMut<'a> {
    materials: Writer<'a, PbrMaterial>,
}

impl<'a> MaterialsMut<'a> {
    pub fn insert(&mut self, material: PbrMaterial) -> MaterialId {
        let id = self.materials.insert(material);
        MaterialId(id)
    }

    pub fn remove(&mut self, id: MaterialId) {
        self.materials.remove(id.0);
    }
}

pub struct CamerasMut<'a> {
    cameras: Writer<'a, Camera>,
    events: &'a mut Vec<Event>,
}

impl<'a> CamerasMut<'a> {
    pub fn insert(&mut self, camera: Camera) -> CameraId {
        let id = CameraId(self.cameras.insert(camera));
        self.events.push(Event::CreateCamera(id));
        id
    }

    pub fn remove(&mut self, id: CameraId) {
        self.cameras.remove(id.0);

        self.events
            .retain(|event| *event != Event::CreateCamera(id));
        self.events.push(Event::DestroyCamera(id));
    }
}

#[derive(Debug)]
pub struct ObjectsMut<'a> {
    objects: Writer<'a, Object>,
    events: &'a mut Vec<Event>,
}

impl<'a> ObjectsMut<'a> {
    pub fn insert(&mut self, object: Object) -> ObjectId {
        let id = ObjectId(self.objects.insert(object));
        self.events.push(Event::CreateObject(id));
        id
    }

    pub fn remove(&mut self, id: ObjectId) {
        self.objects.remove(id.0);

        self.events
            .retain(|event| *event != Event::CreateObject(id));
        self.events.push(Event::DestroyObject(id));
    }
}

pub struct DirectionalLightsMut<'a> {
    lights: Writer<'a, DirectionalLight>,
    events: &'a mut Vec<Event>,
}

impl<'a> DirectionalLightsMut<'a> {
    pub fn insert(&mut self, light: DirectionalLight) -> DirectionalLightId {
        let id = DirectionalLightId(self.lights.insert(light));
        self.events.push(Event::CreateDirectionalLight(id));
        id
    }

    pub fn remove(&mut self, id: DirectionalLightId) {
        self.lights.remove(id.0);

        self.events
            .retain(|event| *event != Event::CreateDirectionalLight(id));
        self.events.push(Event::DestroyDirectionalLight(id));
    }
}

pub struct PointLightsMut<'a> {
    lights: Writer<'a, PointLight>,
    events: &'a mut Vec<Event>,
}

impl<'a> PointLightsMut<'a> {
    pub fn insert(&mut self, light: PointLight) -> PointLightId {
        let id = PointLightId(self.lights.insert(light));
        self.events.push(Event::CreatePointLight(id));
        id
    }

    pub fn remove(&mut self, id: PointLightId) {
        self.lights.remove(id.0);

        self.events
            .retain(|event| *event != Event::CreatePointLight(id));
        self.events.push(Event::DestroyPointLight(id));
    }
}

pub struct SpotLightsMut<'a> {
    lights: Writer<'a, SpotLight>,
    events: &'a mut Vec<Event>,
}

impl<'a> SpotLightsMut<'a> {
    pub fn insert(&mut self, light: SpotLight) -> SpotLightId {
        let id = SpotLightId(self.lights.insert(light));
        self.events.push(Event::CreateSpotLight(id));
        id
    }

    pub fn remove(&mut self, id: SpotLightId) {
        self.lights.remove(id.0);

        self.events
            .retain(|event| *event != Event::CreateSpotLight(id));
        self.events.push(Event::DestroySpotLight(id));
    }
}

#[derive(Debug)]
pub struct ScenesMut<'a> {
    scenes: RefMut<'a, Arena<()>>,
}

impl<'a> ScenesMut<'a> {
    pub fn insert(&mut self) -> SceneId {
        SceneId(self.scenes.insert(()))
    }

    pub fn remove(&mut self, id: SceneId) {
        self.scenes.remove(id.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Object {
    pub transform: Transform,
    pub scene: SceneId,
    pub mesh: MeshId,
    pub material: MaterialId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    CreateCamera(CameraId),
    DestroyCamera(CameraId),
    CreateObject(ObjectId),
    DestroyObject(ObjectId),
    CreateDirectionalLight(DirectionalLightId),
    DestroyDirectionalLight(DirectionalLightId),
    CreatePointLight(PointLightId),
    DestroyPointLight(PointLightId),
    CreateSpotLight(SpotLightId),
    DestroySpotLight(SpotLightId),
    UpdateMainPassOptions(MainPassOptions),
}
