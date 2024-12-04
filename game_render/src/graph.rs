pub mod scheduler;

use std::collections::HashMap;

use glam::UVec2;
use wgpu::hal::auxil::db;
use wgpu::{Buffer, CommandEncoder, Device, Queue, Texture, TextureFormat, TextureView};

use crate::camera::RenderTarget;
use crate::mipmap::MipMapGenerator;

pub trait Node: Send + Sync + 'static {
    fn render<'a, 'b>(&self, ctx: &'b mut RenderContext<'a, 'b>);
}

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
    pub fn read<T>(&self, label: SlotLabel) -> Result<&T, InputSlotError>
    where
        T: SlotValue,
    {
        let Some(flags) = self.resource_permissions.get(&label).copied() else {
            return Err(InputSlotError::NotRegistered);
        };

        if !flags.contains(SlotFlags::READ) {
            return Err(InputSlotError::NotRegistered);
        }

        // This resource access is infalliable because we have previously
        // that the permissions are valid, indicating that the scheduler
        // allowed this access.
        let resource = self.resources.get(&label).unwrap();
        T::downcast(resource).ok_or(InputSlotError::InvalidType)
    }

    pub fn write<T>(&mut self, label: SlotLabel, value: T) -> Result<(), OutputSlotError>
    where
        T: SlotValue,
    {
        let Some(flags) = self.resource_permissions.get(&label).copied() else {
            return Err(OutputSlotError::NotRegistered);
        };

        if !flags.contains(SlotFlags::WRITE) {
            return Err(OutputSlotError::NotRegistered);
        }

        self.resources.insert(label, value.upcast());
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeLabel(&'static str);

impl NodeLabel {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SlotLabel(&'static str);

impl SlotLabel {
    pub const SURFACE: Self = Self("_SURFACE_TEXTURE");

    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }
}

#[derive(Default)]
pub struct RenderGraph {
    nodes: HashMap<NodeLabel, NodeState>,
    pub(crate) has_changed: bool,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            has_changed: false,
        }
    }

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

    pub fn add_node_dependency(&mut self, node: NodeLabel, depends_on: NodeLabel) {
        let node = self.nodes.get_mut(&node).unwrap();
        node.dependencies.push(Dependency::Node(depends_on));
        self.has_changed = true;
    }

    pub fn add_slot_dependency(
        &mut self,
        node: NodeLabel,
        label: SlotLabel,
        kind: SlotKind,
        flags: SlotFlags,
    ) {
        let node = self.nodes.get_mut(&node).unwrap();
        node.dependencies.push(Dependency::Slot(label, kind, flags));
        *node.permissions.entry(label).or_insert(SlotFlags::empty()) |= flags;
        self.has_changed = true;
    }

    pub(crate) fn get(&self, node: NodeLabel) -> Option<&NodeState> {
        self.nodes.get(&node)
    }
}

pub(crate) struct NodeState {
    pub(crate) label: NodeLabel,
    pub(crate) node: Box<dyn Node>,
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) permissions: HashMap<SlotLabel, SlotFlags>,
}

#[derive(Clone, Debug)]
pub enum Dependency {
    Node(NodeLabel),
    Slot(SlotLabel, SlotKind, SlotFlags),
}

#[derive(Clone, Debug)]
pub enum InputSlotError {
    NotRegistered,
    InvalidType,
}

#[derive(Clone, Debug)]
pub enum OutputSlotError {
    NotRegistered,
    InvalidType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlotKind {
    Buffer,
    Texture,
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct SlotFlags: u32 {
        const READ = 0b01;
        const WRITE = 0b10;
    }
}

pub trait SlotValue: Sized {
    fn upcast(self) -> SlotValueInner<'static>;

    fn downcast<'a>(value: &'a SlotValueInner<'_>) -> Option<&'a Self>;
}

#[derive(Debug)]
pub(crate) enum SlotValueInner<'a> {
    Buffer(Buffer),
    Texture(Texture),
    TextureRef(&'a Texture),
}

impl SlotValue for Texture {
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

impl SlotValue for Buffer {
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
