//! # Render graphs
//!
//! The [`RenderGraph`] provides the ability to compose a list of render passes ([`Node`]s). New
//! nodes can be registered via [`add_node`], causing them to be placed anywhere in the graph of
//! render passes.
//!
//! If a [`Node`] depends on the effects of another node a dependency chain can be established via
//! [`add_node_dependency`]. All dependencies will be scheduled before the [`Node`] is scheduled.
//!
//! A [`Node`] must make all its resources shared with other [`Node`]s observable via
//! [`add_slot_depdency`] beforehand. A resource (identified by the tuple [`SlotLabel`],
//! [`SlotKind`]) can be marked as either readable or writable.
//!
//! If a [`Node`] registers a resource as readable another [`Node`] that provides this resource
//! as writable will be scheduled before and the [`Node`] with readable access will become
//! implicitly dependent on the [`Node`] with writable access.
//!
//! Note that a [`Node`] may never be called if it outputs some resources that are never used
//! by another [`Node`].
//!
//! [`add_node`]: RenderGraph::add_node
//! [`add_node_dependency`]: RenderGraph::add_node_dependency
//! [`add_slot_dependency`]: RenderGraph::add_slot_dependency

pub(crate) mod scheduler;

use std::collections::HashMap;

use glam::UVec2;
use thiserror::Error;
use wgpu::{Buffer, CommandEncoder, Device, Queue, Texture, TextureFormat, TextureView};

use crate::camera::RenderTarget;
use crate::mipmap::MipMapGenerator;

pub trait Node: Send + Sync + 'static {
    /// Renders the node.
    fn render<'a>(&self, ctx: &'a mut RenderContext<'_, 'a>);
}

/// Context provided to render a [`Node`].
pub struct RenderContext<'a, 'b> {
    pub render_target: RenderTarget,
    pub encoder: &'b mut CommandEncoder,
    pub target: &'a TextureView,
    pub size: UVec2,
    pub format: TextureFormat,
    pub device: &'a Device,
    pub queue: &'a Queue,
    pub mipmap: &'b mut MipMapGenerator,
    pub(crate) resource_permissions: &'a HashMap<SlotLabel, SlotFlags>,
    pub(crate) resources: &'b mut HashMap<SlotLabel, SlotValueInner<'a>>,
}

impl<'a, 'b> RenderContext<'a, 'b> {
    /// Reads the resource with the given [`SlotLabel`].
    ///
    /// The resource must have been previously registered with the [`READ`] flag.
    ///
    /// # Errors
    ///
    /// Returns an error if the slot has not been registered or the requested type
    /// missmatches the type used to register the slot.
    ///
    /// [`READ`]: SlotFlags::READ
    pub fn read<T>(&self, label: SlotLabel) -> Result<&T, SlotError>
    where
        T: SlotValue,
    {
        let Some(flags) = self.resource_permissions.get(&label).copied() else {
            return Err(SlotError::NotRegistered);
        };

        if !flags.contains(SlotFlags::READ) {
            return Err(SlotError::NotRegistered);
        }

        // This resource access is infalliable because we have previously
        // that the permissions are valid, indicating that the scheduler
        // allowed this access.
        let resource = self.resources.get(&label).unwrap();
        T::downcast(resource).ok_or(SlotError::InvalidType)
    }

    /// Writes to the resource with the given [`SlotLabel`].
    ///
    /// The resource must have been previously registered with the [`WRITE`] flag.
    ///
    /// # Errors
    ///
    /// Returns an error if the slot has not been registered or the requested type
    /// missmatches the type used to register the slot.
    ///
    /// [`WRITE`]: SlotFlags::WRITE
    pub fn write<T>(&mut self, label: SlotLabel, value: T) -> Result<(), SlotError>
    where
        T: SlotValue,
    {
        let Some(flags) = self.resource_permissions.get(&label).copied() else {
            return Err(SlotError::NotRegistered);
        };

        if !flags.contains(SlotFlags::WRITE) {
            return Err(SlotError::NotRegistered);
        }

        self.resources.insert(label, value.upcast());
        Ok(())
    }
}

/// A unique identifier for a [`Node`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeLabel(&'static str);

impl NodeLabel {
    /// Creates a new `NodeLabel` from the given string.
    #[inline]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }
}

/// A unique identifier for a slot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlotLabel(&'static str);

impl SlotLabel {
    /// The slot that refers to the final surface that will be presented.
    ///
    /// This slot is always available.
    pub const SURFACE: Self = Self("_SURFACE_TEXTURE");

    /// Creates a new `SlotLabel` from the given string.
    #[inline]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }
}

/// A render graph composed for [`Node`]s.
///
/// Refer to the module documentation for more details.
#[derive(Default)]
pub struct RenderGraph {
    nodes: HashMap<NodeLabel, NodeState>,
    pub(crate) has_changed: bool,
}

impl RenderGraph {
    /// Creates a new, empty `RenderGraph`.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            has_changed: false,
        }
    }

    /// Registers a new [`Node`] in the `RenderGraph`.
    pub fn add_node<T>(&mut self, label: NodeLabel, node: T)
    where
        T: Node,
    {
        self.nodes.insert(
            label,
            NodeState {
                label,
                node: Box::new(node),
                dependencies: Vec::new(),
                permissions: HashMap::new(),
            },
        );
        self.has_changed = true;
    }

    /// Adds a new dependency to the node with the given `node` label.
    ///
    /// This means that the node referred to by `depends_on` must run before the node referred to
    /// by `node` can run.
    ///
    /// # Panics
    ///
    /// Panics if the nodes `node` or `depends_on` don't exist.
    pub fn add_node_dependency(&mut self, node: NodeLabel, depends_on: NodeLabel) {
        if !self.nodes.contains_key(&depends_on) {
            panic!("cannot add dependency: {:?} does not exist", depends_on);
        }

        let Some(node) = self.nodes.get_mut(&node) else {
            panic!("cannot add dependency: {:?} does not exist", node);
        };

        let dependency_exists = node.dependencies.iter().any(|dep| match dep {
            Dependency::Node(label) => *label == depends_on,
            Dependency::Slot(_, _, _) => false,
        });

        // Only create the new dependency if it does not yet exist.
        if !dependency_exists {
            node.dependencies.push(Dependency::Node(depends_on));
            self.has_changed = true;
        }
    }

    /// Adds a new slot to the node with the given `node` label.
    ///
    /// If flags contains [`WRITE`] the node makes the slot with the given [`SlotLabel`] and
    /// [`SlotKind`] available.
    ///
    /// If flags contains [`READ`] another node providing the given slot must be run before the
    /// node referred to by `node` can run.
    ///
    /// [`WRITE`]: SlotFlags::WRITE
    /// [`READ`]: SlotFlags::READ
    pub fn add_slot_dependency(
        &mut self,
        node: NodeLabel,
        label: SlotLabel,
        kind: SlotKind,
        flags: SlotFlags,
    ) {
        let Some(node) = self.nodes.get_mut(&node) else {
            panic!("cannot add slot: {:?} does not exist", node);
        };

        let dependency = node.dependencies.iter_mut().find(|dep| match dep {
            Dependency::Node(_) => false,
            Dependency::Slot(dep_label, dep_kind, _) => *dep_label == label && *dep_kind == kind,
        });

        match dependency {
            Some(dependency) => match dependency {
                Dependency::Slot(_, _, dep_flags) => {
                    let new_flags = *dep_flags | flags;

                    // New flags are unchanged.
                    if new_flags == *dep_flags {
                        return;
                    }

                    *dep_flags = new_flags;
                }
                _ => unreachable!(),
            },
            None => {
                node.dependencies.push(Dependency::Slot(label, kind, flags));
            }
        }

        *node.permissions.entry(label).or_insert(SlotFlags::empty()) |= flags;
        self.has_changed = true;
    }

    /// Returns a reference to the node with the given [`NodeLabel`].
    pub(crate) fn get(&self, node: NodeLabel) -> Option<&NodeState> {
        self.nodes.get(&node)
    }
}

pub(crate) struct NodeState {
    pub(crate) label: NodeLabel,
    pub(crate) node: Box<dyn Node>,
    dependencies: Vec<Dependency>,
    pub(crate) permissions: HashMap<SlotLabel, SlotFlags>,
}

#[derive(Clone, Debug)]
enum Dependency {
    Node(NodeLabel),
    Slot(SlotLabel, SlotKind, SlotFlags),
}

/// Error that can occur on slot access operations.
#[derive(Clone, Debug, Error)]
pub enum SlotError {
    /// The slot could no be accessed because it was not registered beforehand.
    #[error("not registered")]
    NotRegistered,
    /// The slot could not be accessed because it was registered as a different type than it was
    /// requested as.
    #[error("invalid type")]
    InvalidType,
}

/// Types that can be used in a slot.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlotKind {
    Buffer,
    Texture,
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct SlotFlags: u32 {
        /// Marks a slot as readable.
        const READ = 0b01;
        /// Marks a slot as writable.
        const WRITE = 0b10;
    }
}

/// A value that can be used in a slot.
///
/// Refer to [`SlotKind`] for available types.
pub trait SlotValue: private::Sealed {}

#[derive(Debug)]
pub(crate) enum SlotValueInner<'a> {
    Buffer(Buffer),
    Texture(Texture),
    TextureRef(&'a Texture),
}

// This is on purpose, don't leak the internal details.
#[allow(private_interfaces)]
impl private::Sealed for Texture {
    fn upcast(self) -> SlotValueInner<'static> {
        SlotValueInner::Texture(self)
    }

    fn downcast<'a>(value: &'a SlotValueInner<'_>) -> Option<&'a Self> {
        match value {
            SlotValueInner::Texture(v) => Some(v),
            SlotValueInner::TextureRef(v) => Some(*v),
            _ => None,
        }
    }
}

impl SlotValue for Texture {}

// This is on purpose, don't leak the internal details.
#[allow(private_interfaces)]
impl private::Sealed for Buffer {
    fn upcast(self) -> SlotValueInner<'static> {
        SlotValueInner::Buffer(self)
    }

    fn downcast<'a>(value: &'a SlotValueInner<'_>) -> Option<&'a Self> {
        match value {
            SlotValueInner::Buffer(v) => Some(v),
            _ => None,
        }
    }
}

impl SlotValue for Buffer {}

mod private {
    use super::SlotValueInner;

    // This is on purpose, don't leak the internal details.
    #[allow(private_interfaces)]
    pub trait Sealed: Sized {
        fn upcast(self) -> SlotValueInner<'static>;
        fn downcast<'a>(value: &'a SlotValueInner<'_>) -> Option<&'a Self>;
    }
}
