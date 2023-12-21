use std::collections::HashMap;

use game_common::collections::arena::{self, Arena};
use game_common::collections::vec_map::VecMap;
use game_common::components::rendering::Color;
use game_common::components::transform::Transform;
use game_render::entities::ObjectId;
use game_render::pbr::material::MaterialId;
use game_render::pbr::mesh::MeshId;
use game_render::texture::ImageId;
use game_render::Renderer;
use game_tracing::trace_span;
use glam::Vec3;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub(crate) arena::Key);

#[derive(Debug)]
pub struct SpawnedScene {
    pub(crate) nodes: Arena<super::scene::Node>,
    pub(crate) children: HashMap<Key, Vec<Key>>,
    pub(crate) parents: VecMap<arena::Key, arena::Key>,
    pub(crate) global_transform: HashMap<Key, Transform>,
    pub(crate) meshes: Vec<MeshId>,
    pub(crate) materials: Vec<MaterialId>,
    pub(crate) images: Vec<ImageId>,
    pub(crate) entities: HashMap<Key, ObjectId>,
}

impl SpawnedScene {
    pub fn new() -> Self {
        Self {
            nodes: Arena::new(),
            children: HashMap::new(),
            parents: VecMap::new(),
            global_transform: HashMap::new(),
            materials: Vec::new(),
            meshes: Vec::new(),
            images: Vec::new(),
            entities: HashMap::new(),
        }
    }

    pub(crate) fn remove(&mut self, key: Key) {
        self.nodes.remove(key.0);
        self.global_transform.remove(&key);

        if let Some(parent) = self.parents.remove(key.0) {
            if let Some(children) = self.children.get_mut(&Key(parent)) {
                children.retain(|id| *id != key);
            }
        }

        if let Some(children) = self.children.remove(&key) {
            for c in children {
                self.remove(c);
            }
        }
    }

    pub(crate) fn append(&mut self, parent: Option<Key>, node: super::scene::Node) -> Key {
        let transform = node.transform;

        let key = Key(self.nodes.insert(node));

        if let Some(parent) = parent {
            debug_assert!(self.nodes.contains_key(parent.0));

            self.parents.insert(key.0, parent.0);
            self.children.entry(parent).or_default().push(key);
        }

        self.global_transform.insert(key, transform);

        key
    }

    pub fn set_transform(&mut self, transform: Transform) {
        // Find root elements, i.e. element without a parent.
        // Note that it is possible that multiple root nodes
        // exist.
        for (key, _) in self.nodes.clone().iter() {
            if self.parents.get(key).is_none() {
                self.nodes.get_mut(key).unwrap().transform = transform;
            }
        }
    }

    pub fn compute_transform(&mut self) {
        let _span = trace_span!("SceneGraph::compute_transform").entered();

        // FIXME: This is a 1:1 copy from the old ECS implementation that is
        // still extreamly inefficient.

        let mut transforms = HashMap::new();
        let mut parents = HashMap::new();

        for (key, node) in &self.nodes {
            if self.parents.get(key).is_none() {
                transforms.insert(key, node.transform);
            }

            if let Some(children) = self.children.get(&Key(key)) {
                for child in children {
                    parents.insert(*child, key);
                }
            }
        }

        while !parents.is_empty() {
            for (child, parent) in parents.clone().iter() {
                if let Some(transform) = transforms.get(parent) {
                    let local_transform = self.nodes.get(child.0).unwrap();
                    parents.remove(child);

                    transforms.insert(child.0, transform.mul_transform(local_transform.transform));
                }
            }
        }

        for (key, transform) in transforms.into_iter() {
            *self.global_transform.get_mut(&Key(key)).unwrap() = transform;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, &super::scene::Node)> {
        self.nodes.iter().map(|(k, v)| (Key(k), v))
    }

    pub fn destroy_resources(self, renderer: &mut Renderer) {
        for id in self.meshes {
            renderer.meshes.remove(id);
        }

        for id in self.materials {
            renderer.materials.remove(id);
        }

        for id in self.images {
            renderer.images.remove(id);
        }
    }
}

// Copied and adapted from`TransformHierarchy` because using it directly
// causes too much trouble.
#[derive(Clone, Debug, Default)]
pub struct SceneGraph {
    nodes: Arena<super::scene::Node>,
    children: HashMap<Key, Vec<Key>>,
    parents: VecMap<arena::Key, arena::Key>,
    removed_nodes: Vec<Key>,
    added_nodes: Vec<Key>,
    global_transform: HashMap<Key, Transform>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            nodes: Arena::new(),
            children: HashMap::new(),
            parents: VecMap::new(),
            removed_nodes: Vec::new(),
            added_nodes: Vec::new(),
            global_transform: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn append(&mut self, parent: Option<Key>, node: super::scene::Node) -> Key {
        let transform = node.transform;

        let key = Key(self.nodes.insert(node));
        self.added_nodes.push(key);

        if let Some(parent) = parent {
            debug_assert!(self.nodes.contains_key(parent.0));

            self.parents.insert(key.0, parent.0);
            self.children.entry(parent).or_default().push(key);
        }

        self.global_transform.insert(key, transform);

        key
    }

    pub fn remove(&mut self, key: Key) {
        self.nodes.remove(key.0);
        self.global_transform.remove(&key);
        self.removed_nodes.push(key);

        if let Some(parent) = self.parents.remove(key.0) {
            if let Some(children) = self.children.get_mut(&Key(parent)) {
                children.retain(|id| *id != key);
            }
        }

        if let Some(children) = self.children.remove(&key) {
            for c in children {
                self.removed_nodes.push(c);
                self.remove(c);
            }
        }
    }

    pub fn get(&self, key: Key) -> Option<&super::scene::Node> {
        self.nodes.get(key.0)
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut super::scene::Node> {
        self.nodes.get_mut(key.0)
    }

    /// Removes all entities.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.children.clear();
        self.parents.clear();
        self.removed_nodes.clear();
        self.added_nodes.clear();
    }

    pub fn contains_key(&self, key: Key) -> bool {
        self.nodes.contains_key(key.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, &super::scene::Node)> + '_ {
        self.nodes.iter().map(|(k, v)| (Key(k), v))
    }

    pub fn values(&self) -> impl Iterator<Item = &super::scene::Node> + '_ {
        self.nodes.values()
    }

    pub fn parent(&self, key: Key) -> Option<(Key, &super::scene::Node)> {
        let parent = self.parents.get(key.0)?;
        Some((Key(*parent), self.nodes.get(*parent).unwrap()))
    }

    pub fn children(
        &self,
        parent: Key,
    ) -> Option<impl Iterator<Item = (Key, &super::scene::Node)> + '_> {
        let children = self.children.get(&parent)?;
        Some(children.iter().map(|key| {
            let node = self.nodes.get(key.0).unwrap();
            (*key, node)
        }))
    }

    pub fn iter_removed(&self) -> impl Iterator<Item = Key> + '_ {
        self.removed_nodes.iter().copied()
    }

    pub fn iter_added(&self) -> impl Iterator<Item = Key> + '_ {
        self.added_nodes.iter().copied()
    }

    pub fn compute_transform(&mut self) {
        let _span = trace_span!("SceneGraph::compute_transform").entered();

        // FIXME: This is a 1:1 copy from the old ECS implementation that is
        // still extreamly inefficient.

        let mut transforms = HashMap::new();
        let mut parents = HashMap::new();

        for (key, node) in &self.nodes {
            if self.parent(Key(key)).is_none() {
                transforms.insert(key, node.transform);
            }

            if let Some(children) = self.children.get(&Key(key)) {
                for child in children {
                    parents.insert(*child, key);
                }
            }
        }

        while !parents.is_empty() {
            for (child, parent) in parents.clone().iter() {
                if let Some(transform) = transforms.get(parent) {
                    let local_transform = self.nodes.get(child.0).unwrap();
                    parents.remove(child);

                    transforms.insert(child.0, transform.mul_transform(local_transform.transform));
                }
            }
        }

        for (key, transform) in transforms.into_iter() {
            *self.global_transform.get_mut(&Key(key)).unwrap() = transform;
        }
    }

    /// Returns an iterator over all entities with an updated transform.
    pub fn iter_changed_global_transform(&self) -> impl Iterator<Item = (Key, Transform)> + '_ {
        self.global_transform.iter().map(|(k, v)| (*k, *v))
    }

    pub fn clear_trackers(&mut self) {
        self.removed_nodes.clear();
        self.added_nodes.clear();
    }
}

/// A node in a scene graph.
#[derive(Clone, Debug)]
pub struct Node {
    pub transform: Transform,
    pub components: Vec<Component>,
}

impl Node {
    /// Creates an empty `Node` with the given transform.
    #[inline]
    pub const fn from_transform(transform: Transform) -> Self {
        Self {
            transform,
            components: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub enum Component {
    MeshInstance(MeshInstance),
    DirectionalLight(DirectionalLight),
    PointLight(PointLight),
    SpotLight(SpotLight),
    Collider(Collider),
}

#[derive(Copy, Clone, Debug)]
pub struct MeshInstance {
    pub mesh: MeshId,
    pub material: MaterialId,
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

#[derive(Copy, Clone, Debug)]
pub struct SpotLight {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}

#[derive(Clone, Debug)]
pub struct Collider {
    // TOOD: More fine-grained control of center of mass, etc..
    pub mass: f32,
    pub friction: f32,
    pub restitution: f32,
    pub shape: ColliderShape,
}

#[derive(Clone, Debug)]
pub enum ColliderShape {
    Cuboid(Cuboid),
    TriMesh(TriMesh),
}

#[derive(Copy, Clone, Debug)]
pub struct Cuboid {
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
}

#[derive(Clone, Debug)]
pub struct TriMesh {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u32>,
}
