pub mod pool;

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
    pub(crate) scenes: Arena<()>,
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
            events: &mut self.events,
        }
    }

    pub fn images(&mut self) -> ImagesMut<'_> {
        ImagesMut {
            images: unsafe { self.resources.images.writer() },
            events: &mut self.events,
        }
    }

    pub fn materials(&mut self) -> MaterialsMut<'_> {
        MaterialsMut {
            materials: unsafe { self.resources.materials.writer() },
            events: &mut self.events,
        }
    }

    pub fn objects(&mut self) -> ObjectsMut<'_> {
        ObjectsMut {
            objects: unsafe { self.resources.objects.writer() },
            events: &mut self.events,
        }
    }
}

#[derive(Debug)]
pub struct MeshesMut<'a> {
    meshes: Writer<'a, Mesh>,
    events: &'a mut Vec<Event>,
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
    events: &'a mut Vec<Event>,
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
    events: &'a mut Vec<Event>,
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

#[derive(Debug)]
pub struct ObjectsMut<'a> {
    objects: Writer<'a, Object>,
    events: &'a mut Vec<Event>,
}

impl<'a> ObjectsMut<'a> {
    pub fn insert(&mut self, object: Object) -> ObjectId {
        let id = self.objects.insert(object);
        ObjectId(id)
    }

    pub fn remove(&mut self, id: ObjectId) {
        self.objects.remove(id.0);
    }
}

pub struct DirectionalLightsMut<'a> {
    lights: Writer<'a, DirectionalLight>,
    events: &'a mut Vec<Event>,
}

impl<'a> DirectionalLightsMut<'a> {
    pub fn insert(&mut self, light: DirectionalLight) -> DirectionalLightId {
        let id = self.lights.insert(light);
        DirectionalLightId(id)
    }

    pub fn remove(&mut self, id: DirectionalLightId) {
        self.lights.remove(id.0);
    }
}

pub struct PointLightsMut<'a> {
    lights: Writer<'a, PointLight>,
    events: &'a mut Vec<Event>,
}

impl<'a> PointLightsMut<'a> {
    pub fn insert(&mut self, light: PointLight) -> PointLightId {
        let id = self.lights.insert(light);
        PointLightId(id)
    }

    pub fn remove(&mut self, id: PointLightId) {
        self.lights.remove(id.0);
    }
}

pub struct SpotLightsMut<'a> {
    lights: Writer<'a, SpotLight>,
    events: &'a mut Vec<Event>,
}

impl<'a> SpotLightsMut<'a> {
    pub fn insert(&mut self, light: SpotLight) -> SpotLightId {
        let id = self.lights.insert(light);
        SpotLightId(id)
    }

    pub fn remove(&mut self, id: SpotLightId) {
        self.lights.remove(id.0);
    }
}

#[derive(Debug)]
pub struct ScenesMut<'a> {
    scenes: &'a mut Arena<()>,
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

#[derive(Clone, Debug)]
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
