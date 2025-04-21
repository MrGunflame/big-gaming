use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender};
use game_common::collections::arena::{Arena, Key};
use game_common::components::Transform;
use game_render::mesh::Mesh;
use game_render::texture::image::MipImage;
use parking_lot::Mutex;

use crate::StandardMaterial;
use crate::camera::Camera;
use crate::lights::Light;

#[derive(Clone, Debug)]
pub struct Entities {
    inner: Arc<Inner>,
}

impl Entities {
    pub(super) fn new() -> (Self, Receiver<Event>) {
        let (tx, rx) = crossbeam::channel::unbounded();
        (
            Self {
                inner: Arc::new(Inner {
                    events: tx,
                    scenes: Mutex::new(Arena::new()),
                    meshes: Mutex::new(Arena::new()),
                    materials: Mutex::new(Arena::new()),
                    images: Mutex::new(Arena::new()),
                    cameras: Mutex::new(Arena::new()),
                    objects: Mutex::new(Arena::new()),
                    lights: Mutex::new(Arena::new()),
                }),
            },
            rx,
        )
    }

    pub fn create_scene(&self) -> SceneHandle {
        let mut scenes = self.inner.scenes.lock();
        let id = SceneId(scenes.insert(1));

        self.inner.events.send(Event::CreateScene(id)).ok();
        SceneHandle {
            inner: self.inner.clone(),
            id,
        }
    }

    pub fn create_mesh(&self, mesh: Mesh) -> MeshHandle {
        let mut meshes = self.inner.meshes.lock();
        let id = MeshId(meshes.insert(1));

        self.inner.events.send(Event::CreateMesh(id, mesh)).ok();
        MeshHandle {
            inner: self.inner.clone(),
            id,
        }
    }

    pub fn create_image<T>(&self, image: T) -> ImageHandle
    where
        T: Into<MipImage>,
    {
        let mut images = self.inner.images.lock();
        let id = ImageId(images.insert(1));

        self.inner
            .events
            .send(Event::CreateImage(id, image.into()))
            .ok();
        ImageHandle {
            inner: self.inner.clone(),
            id,
        }
    }

    pub fn create_material(&self, material: StandardMaterial) -> MaterialHandle {
        let mut materials = self.inner.materials.lock();
        let id = MaterialId(materials.insert(1));

        self.inner
            .events
            .send(Event::CreateMaterial(id, material.clone()))
            .ok();
        MaterialHandle {
            inner: self.inner.clone(),
            id,
            material,
        }
    }

    pub fn create_object(&self, object: Object) -> ObjectHandle {
        let mut objects = self.inner.objects.lock();
        let id = ObjectId(objects.insert(1));

        self.inner
            .events
            .send(Event::CreateObject(id, object.clone()))
            .ok();
        ObjectHandle {
            inner: self.inner.clone(),
            id,
            mesh: object.mesh,
            material: object.material,
        }
    }

    pub fn create_camera(&self, camera: Camera) -> CameraHandle {
        let mut cameras = self.inner.cameras.lock();
        let id = CameraId(cameras.insert(1));

        self.inner.events.send(Event::CreateCamera(id, camera)).ok();
        CameraHandle {
            inner: self.inner.clone(),
            id,
        }
    }

    pub fn create_light(&self, light: Light) -> LightHandle {
        let mut lights = self.inner.lights.lock();
        let id = LightId(lights.insert(1));

        self.inner.events.send(Event::CreateLight(id, light)).ok();
        LightHandle {
            inner: self.inner.clone(),
            id,
        }
    }
}

#[derive(Debug)]
struct Inner {
    events: Sender<Event>,
    // Contains ref counts for all entities.
    scenes: Mutex<Arena<usize>>,
    meshes: Mutex<Arena<usize>>,
    materials: Mutex<Arena<usize>>,
    images: Mutex<Arena<usize>>,
    cameras: Mutex<Arena<usize>>,
    objects: Mutex<Arena<usize>>,
    lights: Mutex<Arena<usize>>,
}

#[derive(Debug)]
pub struct SceneHandle {
    inner: Arc<Inner>,
    id: SceneId,
}

impl SceneHandle {
    pub(crate) fn id(&self) -> SceneId {
        self.id
    }
}

impl Clone for SceneHandle {
    fn clone(&self) -> Self {
        // Increment ref count
        let mut scenes = self.inner.scenes.lock();
        let ref_count = scenes.get_mut(self.id.0).unwrap();
        debug_assert_ne!(*ref_count, usize::MAX);
        *ref_count = ref_count.saturating_add(1);

        Self {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

impl Drop for SceneHandle {
    fn drop(&mut self) {
        let mut scenes = self.inner.scenes.lock();
        let ref_count = scenes.get_mut(self.id.0).unwrap();
        *ref_count -= 1;
        if *ref_count != 0 {
            return;
        }

        self.inner.events.send(Event::DestroyScene(self.id)).ok();
    }
}

#[derive(Debug)]
pub struct MeshHandle {
    inner: Arc<Inner>,
    id: MeshId,
}

impl MeshHandle {
    pub(crate) fn id(&self) -> MeshId {
        self.id
    }
}

impl Clone for MeshHandle {
    fn clone(&self) -> Self {
        let mut meshes = self.inner.meshes.lock();
        let ref_count = meshes.get_mut(self.id.0).unwrap();
        debug_assert_ne!(*ref_count, usize::MAX);
        *ref_count = ref_count.saturating_add(1);

        Self {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

impl Drop for MeshHandle {
    fn drop(&mut self) {
        let mut meshes = self.inner.meshes.lock();
        let ref_count = meshes.get_mut(self.id.0).unwrap();
        *ref_count -= 1;
        if *ref_count != 0 {
            return;
        }

        self.inner.events.send(Event::DestroyMesh(self.id)).ok();
    }
}

#[derive(Debug)]
pub struct ImageHandle {
    inner: Arc<Inner>,
    id: ImageId,
}

impl ImageHandle {
    pub(crate) fn id(&self) -> ImageId {
        self.id
    }
}

impl Clone for ImageHandle {
    fn clone(&self) -> Self {
        let mut images = self.inner.images.lock();
        let ref_count = images.get_mut(self.id.0).unwrap();
        debug_assert_ne!(*ref_count, usize::MAX);
        *ref_count = ref_count.saturating_add(1);

        Self {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

impl Drop for ImageHandle {
    fn drop(&mut self) {
        let mut images = self.inner.images.lock();
        let ref_count = images.get_mut(self.id.0).unwrap();
        *ref_count -= 1;
        if *ref_count != 0 {
            return;
        }

        self.inner.events.send(Event::DestroyImage(self.id)).ok();
    }
}

#[derive(Debug)]
pub struct MaterialHandle {
    inner: Arc<Inner>,
    id: MaterialId,
    // Keep the original material, which contains handles to all images
    // and prevents them from being dropped.
    material: StandardMaterial,
}

impl MaterialHandle {
    pub(crate) fn id(&self) -> MaterialId {
        self.id
    }
}

impl Clone for MaterialHandle {
    fn clone(&self) -> Self {
        let mut materials = self.inner.materials.lock();
        let ref_count = materials.get_mut(self.id.0).unwrap();
        debug_assert_ne!(*ref_count, usize::MAX);
        *ref_count = ref_count.saturating_add(1);

        Self {
            inner: self.inner.clone(),
            id: self.id,
            material: self.material.clone(),
        }
    }
}

impl Drop for MaterialHandle {
    fn drop(&mut self) {
        let mut materials = self.inner.materials.lock();
        let ref_count = materials.get_mut(self.id.0).unwrap();
        *ref_count -= 1;
        if *ref_count != 0 {
            return;
        }

        self.inner.events.send(Event::DestroyMaterial(self.id)).ok();
    }
}

#[derive(Debug)]
pub struct ObjectHandle {
    inner: Arc<Inner>,
    id: ObjectId,
    mesh: MeshHandle,
    material: MaterialHandle,
}

impl Clone for ObjectHandle {
    fn clone(&self) -> Self {
        let mut objects = self.inner.objects.lock();
        let ref_count = objects.get_mut(self.id.0).unwrap();
        debug_assert_ne!(*ref_count, usize::MAX);
        *ref_count = ref_count.saturating_add(1);

        Self {
            inner: self.inner.clone(),
            id: self.id,
            mesh: self.mesh.clone(),
            material: self.material.clone(),
        }
    }
}

impl Drop for ObjectHandle {
    fn drop(&mut self) {
        let mut objects = self.inner.objects.lock();
        let ref_count = objects.get_mut(self.id.0).unwrap();
        *ref_count -= 1;
        if *ref_count != 0 {
            return;
        }

        self.inner.events.send(Event::DestroyObject(self.id)).ok();
    }
}

pub struct CameraHandle {
    inner: Arc<Inner>,
    id: CameraId,
}

impl Clone for CameraHandle {
    fn clone(&self) -> Self {
        let mut cameras = self.inner.cameras.lock();
        let ref_count = cameras.get_mut(self.id.0).unwrap();
        debug_assert_ne!(*ref_count, usize::MAX);
        *ref_count = ref_count.saturating_add(1);

        Self {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

impl Drop for CameraHandle {
    fn drop(&mut self) {
        let mut cameras = self.inner.cameras.lock();
        let ref_count = cameras.get_mut(self.id.0).unwrap();
        *ref_count -= 1;
        if *ref_count != 0 {
            return;
        }

        self.inner.events.send(Event::DestroyCamera(self.id)).ok();
    }
}

pub struct LightHandle {
    inner: Arc<Inner>,
    id: LightId,
}

impl Clone for LightHandle {
    fn clone(&self) -> Self {
        let mut lights = self.inner.lights.lock();
        let ref_count = lights.get_mut(self.id.0).unwrap();
        debug_assert_ne!(*ref_count, usize::MAX);
        *ref_count = ref_count.saturating_add(1);

        Self {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

impl Drop for LightHandle {
    fn drop(&mut self) {
        let mut lights = self.inner.lights.lock();
        let ref_count = lights.get_mut(self.id.0).unwrap();
        *ref_count -= 1;
        if *ref_count != 0 {
            return;
        }

        self.inner.events.send(Event::DestroyLight(self.id)).ok();
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SceneId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MeshId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MaterialId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraId(Key);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LightId(Key);

#[derive(Clone, Debug)]
pub struct Object {
    pub transform: Transform,
    pub scene: SceneHandle,
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
}

#[derive(Debug)]
pub(crate) enum Event {
    CreateScene(SceneId),
    DestroyScene(SceneId),
    CreateMesh(MeshId, Mesh),
    DestroyMesh(MeshId),
    CreateMaterial(MaterialId, StandardMaterial),
    DestroyMaterial(MaterialId),
    CreateImage(ImageId, MipImage),
    DestroyImage(ImageId),
    CreateObject(ObjectId, Object),
    DestroyObject(ObjectId),
    CreateCamera(CameraId, Camera),
    DestroyCamera(CameraId),
    CreateLight(LightId, Light),
    DestroyLight(LightId),
}
