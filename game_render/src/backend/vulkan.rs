use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::{c_void, CStr};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::{Bound, Deref, Range, RangeBounds};
use std::ptr::{null_mut, NonNull};
use std::sync::Arc;
use std::time::Duration;

use ash::ext::debug_utils;
use ash::vk::{
    self, AccessFlags, AcquireNextImageInfoKHR, ApplicationInfo, AttachmentDescription,
    AttachmentLoadOp, AttachmentReference, AttachmentStoreOp, BindBufferMemoryInfo, BlendFactor,
    BlendOp, Bool32, BufferCreateInfo, BufferUsageFlags, ClearColorValue, ClearValue,
    ColorComponentFlags, ColorSpaceKHR, CommandBufferAllocateInfo, CommandBufferBeginInfo,
    CommandBufferInheritanceInfo, CommandBufferLevel, CommandBufferUsageFlags,
    CommandPoolCreateFlags, CommandPoolCreateInfo, CommandPoolResetFlags, ComponentMapping,
    ComponentSwizzle, CompositeAlphaFlagsKHR, CullModeFlags, DebugUtilsMessageSeverityFlagsEXT,
    DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT,
    DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT, DependencyFlags,
    DescriptorPoolCreateInfo, DescriptorPoolResetFlags, DescriptorPoolSize,
    DescriptorSetAllocateInfo, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
    DeviceCreateInfo, DeviceQueueCreateInfo, DeviceQueueInfo2, DynamicState, Extent2D,
    FenceCreateInfo, Format, FrontFace, GraphicsPipelineCreateInfo, ImageAspectFlags, ImageLayout,
    ImageMemoryBarrier, ImageSubresourceRange, ImageUsageFlags, ImageViewCreateInfo, ImageViewType,
    InstanceCreateInfo, LogicOp, MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, Offset2D,
    PhysicalDevice, PhysicalDeviceDynamicRenderingFeatures, PhysicalDeviceFeatures,
    PhysicalDeviceType, PipelineBindPoint, PipelineCache, PipelineColorBlendAttachmentState,
    PipelineColorBlendStateCreateInfo, PipelineDynamicStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineLayoutCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineRenderingCreateInfo, PipelineShaderStageCreateInfo, PipelineStageFlags,
    PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
    PresentInfoKHR, PresentModeKHR, PrimitiveTopology, QueueFlags, Rect2D, RenderingAttachmentInfo,
    RenderingFlags, RenderingInfo, ResolveModeFlags, SampleCountFlags, SemaphoreCreateInfo,
    ShaderModuleCreateInfo, ShaderStageFlags, SharingMode, SubmitInfo, SubpassDependency,
    SubpassDescription, SurfaceKHR, SurfaceTransformFlagsKHR, SwapchainCreateInfoKHR, SwapchainKHR,
    Viewport, WriteDescriptorSet, FALSE, WHOLE_SIZE,
};
use ash::Entry;
use bitflags::bitflags;
use glam::UVec2;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::backend::TextureLayout;

use super::{
    AdapterKind, AdapterMemoryProperties, AdapterProperties, AddressMode, BufferUsage, CopyBuffer,
    DescriptorPoolDescriptor, DescriptorSetDescriptor, Face, FilterMode, LoadOp, MemoryHeap,
    MemoryRequirements, MemoryType, MemoryTypeFlags, PipelineBarriers, PipelineDescriptor,
    PipelineStage, PresentMode, QueueCapabilities, QueueFamily, QueueSubmit,
    RenderPassColorAttachment, RenderPassDescriptor, SamplerDescriptor, ShaderStages, StoreOp,
    SwapchainCapabilities, SwapchainConfig, TextureDescriptor, TextureFormat,
    WriteDescriptorResource, WriteDescriptorResources,
};

/// The highest version of Vulkan that we support.
///
/// See <https://registry.khronos.org/vulkan/specs/latest/man/html/VkApplicationInfo.html>
const API_VERSION: u32 = make_api_version(1, 3, 0);

const APPLICATION_NAME: Option<&CStr> = None;
const APPLICATION_VERSION: u32 = 0;
const ENGINE_NAME: Option<&CStr> = None;
const ENGINE_VERSION: u32 = 0;

const VULKAN_VALIDATION_LAYERS: &CStr = c"VK_LAYER_KHRONOS_validation";

#[derive(Copy, Clone, Debug, Default)]
struct InstanceExtensions {
    /// `VK_KHR_surface`
    surface: bool,
    /// `VK_KHR_wayland_surface`
    surface_wayland: bool,
    /// `VK_KHR_xcb_surface`
    surface_xcb: bool,
    /// `VK_KHR_xlib_surface`
    surface_xlib: bool,
    /// `VK_KHR_win32_surface`
    surface_win32: bool,
    /// `VK_EXT_debug_utils`
    debug_utils: bool,
}

impl InstanceExtensions {
    /// Returns the names of all supported extensions.
    fn names(&self) -> Vec<&'static CStr> {
        let mut names = Vec::new();

        for (enabled, name) in [
            (self.surface, vk::KHR_SURFACE_NAME),
            (self.surface_wayland, vk::KHR_WAYLAND_SURFACE_NAME),
            (self.surface_xcb, vk::KHR_XCB_SURFACE_NAME),
            (self.surface_xlib, vk::KHR_XLIB_SURFACE_NAME),
            (self.surface_win32, vk::KHR_WIN32_SURFACE_NAME),
            (self.debug_utils, vk::EXT_DEBUG_UTILS_NAME),
        ] {
            if enabled {
                names.push(name);
            }
        }

        names
    }
}

const DEVICE_EXTENSIONS: &[&CStr] = &[
    // VK_KHR_swapchain
    ash::khr::swapchain::NAME,
    // VK_KHR_dynamic_rendering
    // Core in Vulkan 1.3
    ash::khr::dynamic_rendering::NAME,
    // `VK_KHR_synchronization2`
    // Core in Vulkan 1.3
    ash::khr::synchronization2::NAME,
];

const fn make_api_version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 22) | (minor << 12) | patch
}

#[derive(Debug)]
pub enum Error {
    MissingLayer(&'static CStr),
}

#[derive(Clone, Debug)]
pub struct Instance {
    instance: Arc<InstanceShared>,
    extensions: InstanceExtensions,
}

impl Instance {
    pub fn new() -> Result<Self, Error> {
        let entry = unsafe { Entry::load().unwrap() };

        let mut app = ApplicationInfo::default()
            .application_version(APPLICATION_VERSION)
            .engine_version(ENGINE_VERSION)
            .api_version(API_VERSION);

        if let Some(name) = APPLICATION_NAME {
            app = app.application_name(name);
        }

        if let Some(name) = ENGINE_NAME {
            app = app.engine_name(name);
        }

        let available_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .unwrap()
                .iter()
                .map(|l| l.layer_name)
                .collect::<HashSet<_>>()
        };

        if !available_layers.contains(&cstr_to_fixed_array(VULKAN_VALIDATION_LAYERS)) {
            return Err(Error::MissingLayer(VULKAN_VALIDATION_LAYERS));
        }

        let supported_extensions = Self::get_supported_extensions(&entry);
        assert!(supported_extensions.debug_utils);

        let mut layers = Vec::new();
        layers.push(VULKAN_VALIDATION_LAYERS.as_ptr());

        let extensions = supported_extensions
            .names()
            .iter()
            .map(|v| v.as_ptr())
            .collect::<Vec<_>>();

        let mut info = InstanceCreateInfo::default()
            .application_info(&app)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions);

        let mut debug_info = DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | DebugUtilsMessageSeverityFlagsEXT::INFO
                    | DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            )
            .message_type(
                DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING,
            )
            .pfn_user_callback(Some(debug_callback));
        info = info.push_next(&mut debug_info);

        let instance = unsafe { entry.create_instance(&info, None).unwrap() };

        let messenger = unsafe {
            debug_utils::Instance::new(&entry, &instance)
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap()
        };

        Ok(Self {
            instance: Arc::new(InstanceShared {
                entry,
                instance,
                messenger,
            }),
            extensions: supported_extensions,
        })
    }

    pub fn adapters(&self) -> Vec<Adapter<'_>> {
        let physical_devices = unsafe { self.instance.enumerate_physical_devices().unwrap() };
        physical_devices
            .into_iter()
            .map(|physical_device| Adapter {
                instance: &self,
                physical_device,
            })
            .collect()
    }

    pub unsafe fn create_surface(
        &self,
        display: RawDisplayHandle,
        window: RawWindowHandle,
    ) -> Surface {
        assert!(self.extensions.surface);

        let surface = match (display, window) {
            #[cfg(all(unix, feature = "wayland"))]
            (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
                assert!(self.extensions.surface_wayland);

                let info = vk::WaylandSurfaceCreateInfoKHR::default()
                    .display(display.display.as_ptr())
                    .surface(window.surface.as_ptr());

                let instance =
                    ash::khr::wayland_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_wayland_surface(&info, None).unwrap() }
            }
            #[cfg(all(unix, feature = "x11"))]
            (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
                assert!(self.extensions.surface_xcb);

                let info = vk::XcbSurfaceCreateInfoKHR::default()
                    .connection(display.connection.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    .window(window.window.get());

                let instance =
                    ash::khr::xcb_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_xcb_surface(&info, None).unwrap() }
            }
            #[cfg(all(unix, feature = "x11"))]
            (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
                assert!(self.extensions.surface_xlib);

                let info = vk::XlibSurfaceCreateInfoKHR::default()
                    .dpy(display.display.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    .window(window.window);

                let instance =
                    ash::khr::xlib_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_xlib_surface(&info, None).unwrap() }
            }
            #[cfg(target_os = "windows")]
            (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
                assert!(self.extensions.surface_win32);

                let info = vk::Win32SurfaceCreateInfoKHR::default()
                    .hinstance(window.hinstance.map(|v| v.get()).unwrap_or_default())
                    .hwnd(window.hwnd.get());

                let instance =
                    ash::khr::win32_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_win32_surface(&info, None).unwrap() }
            }
            _ => todo!(),
        };

        Surface {
            instance: self.instance.clone(),
            surface,
        }
    }

    fn get_supported_extensions(entry: &Entry) -> InstanceExtensions {
        let mut extensions = InstanceExtensions::default();

        let ext_props = unsafe { entry.enumerate_instance_extension_properties(None).unwrap() };
        for props in ext_props {
            let name =
                CStr::from_bytes_until_nul(bytemuck::bytes_of(&props.extension_name)).unwrap();

            match name {
                name if name == vk::KHR_SURFACE_NAME => extensions.surface = true,
                name if name == vk::KHR_WAYLAND_SURFACE_NAME => extensions.surface_wayland = true,
                name if name == vk::KHR_XCB_SURFACE_NAME => extensions.surface_xcb = true,
                name if name == vk::KHR_XLIB_SURFACE_NAME => extensions.surface_xlib = true,
                name if name == vk::KHR_WIN32_SURFACE_NAME => extensions.surface_win32 = true,
                name if name == vk::EXT_DEBUG_UTILS_NAME => extensions.debug_utils = true,
                _ => (),
            }
        }

        extensions
    }
}

pub struct Adapter<'a> {
    instance: &'a Instance,
    physical_device: PhysicalDevice,
}

impl<'a> Adapter<'a> {
    pub fn properties(&self) -> AdapterProperties {
        let properties = unsafe {
            self.instance
                .instance
                .get_physical_device_properties(self.physical_device)
        };

        // `device_name` is a null-terminated UTF-8 string.
        let name = unsafe {
            CStr::from_ptr(properties.device_name.as_ptr())
                .to_string_lossy()
                .to_string()
        };

        let kind = match properties.device_type {
            PhysicalDeviceType::DISCRETE_GPU => AdapterKind::DiscreteGpu,
            PhysicalDeviceType::INTEGRATED_GPU => AdapterKind::IntegratedGpu,
            _ => AdapterKind::Other,
        };

        AdapterProperties { name, kind }
    }

    pub fn memory_properties(&self) -> AdapterMemoryProperties {
        let props = unsafe {
            self.instance
                .instance
                .get_physical_device_memory_properties(self.physical_device)
        };

        let heaps = props
            .memory_heaps
            .iter()
            .take(props.memory_heap_count as usize)
            .enumerate()
            .map(|(id, heap)| MemoryHeap {
                id: id as u32,
                size: heap.size,
            })
            .collect();
        let types = props
            .memory_types
            .iter()
            .take(props.memory_type_count as usize)
            .enumerate()
            .map(|(id, typ)| {
                let mut flags = MemoryTypeFlags::empty();
                if typ
                    .property_flags
                    .contains(MemoryPropertyFlags::DEVICE_LOCAL)
                {
                    flags |= MemoryTypeFlags::DEVICE_LOCAL;
                }
                if typ
                    .property_flags
                    .contains(MemoryPropertyFlags::HOST_VISIBLE)
                {
                    flags |= MemoryTypeFlags::HOST_VISIBLE;
                }
                if typ
                    .property_flags
                    .contains(MemoryPropertyFlags::HOST_COHERENT)
                {
                    flags |= MemoryTypeFlags::HOST_COHERENT;
                }

                MemoryType {
                    id: id as u32,
                    heap: typ.heap_index,
                    flags,
                }
            })
            .collect();

        AdapterMemoryProperties { heaps, types }
    }

    pub fn queue_families(&self) -> Vec<QueueFamily> {
        let queue_families = unsafe {
            self.instance
                .instance
                .get_physical_device_queue_family_properties(self.physical_device)
        };

        queue_families
            .into_iter()
            .enumerate()
            .map(|(index, queue)| {
                let mut capabilities = QueueCapabilities::empty();

                if queue.queue_flags.contains(QueueFlags::GRAPHICS) {
                    capabilities |= QueueCapabilities::GRAPHICS;
                }

                if queue.queue_flags.contains(QueueFlags::COMPUTE) {
                    capabilities |= QueueCapabilities::COMPUTE;
                }

                if queue.queue_flags.contains(QueueFlags::TRANSFER) {
                    capabilities |= QueueCapabilities::TRANSFER;
                }

                QueueFamily {
                    id: index as u32,
                    count: queue.queue_count,
                    capabilities,
                }
            })
            .collect()
    }

    pub fn create_device(&self, queue_id: u32) -> Device {
        let queue_priorities = &[1.0];
        let queue_info = DeviceQueueCreateInfo::default()
            .queue_family_index(queue_id)
            .queue_priorities(queue_priorities);
        let queue_infos = [queue_info];

        let mut layers = Vec::new();
        layers.push(VULKAN_VALIDATION_LAYERS.as_ptr());

        let mut extensions = Vec::new();
        extensions.extend(DEVICE_EXTENSIONS.iter().map(|v| v.as_ptr()));

        let features = PhysicalDeviceFeatures::default();

        let mut dynamic_rendering =
            PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);

        let mut synchronization2 =
            vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true);

        let create_info = DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            // Device layers are deprecated, but the Vulkan spec still recommends
            // applications to pass layers.
            // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#extendingvulkan-layers-devicelayerdeprecation
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features)
            .push_next(&mut dynamic_rendering)
            .push_next(&mut synchronization2);

        let device = unsafe {
            self.instance
                .instance
                .create_device(self.physical_device, &create_info, None)
                .unwrap()
        };

        Device {
            physical_device: self.physical_device,
            queue_family_index: queue_id,
            device: Arc::new(DeviceShared {
                instance: self.instance.instance.clone(),
                device,
            }),
            limits: self.device_limits(),
        }
    }

    fn device_limits(&self) -> DeviceLimits {
        let props = unsafe {
            self.instance
                .instance
                .get_physical_device_properties(self.physical_device)
        };

        DeviceLimits {
            max_push_constants_size: props.limits.max_push_constants_size,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    physical_device: vk::PhysicalDevice,
    device: Arc<DeviceShared>,
    queue_family_index: u32,
    limits: DeviceLimits,
}

impl Device {
    pub fn queue(&self) -> Queue {
        let info = DeviceQueueInfo2::default()
            .queue_family_index(self.queue_family_index)
            // Index is always 0 since we only create
            // a single queue for now.
            .queue_index(0);

        let queue = unsafe { self.device.get_device_queue2(&info) };

        Queue {
            device: self.device.clone(),
            queue,
        }
    }

    pub fn create_buffer(&self, size: NonZeroU64, usage: BufferUsage) -> Buffer {
        let mut buffer_usage_flags = BufferUsageFlags::empty();
        if usage.contains(BufferUsage::TRANSFER_SRC) {
            buffer_usage_flags |= BufferUsageFlags::TRANSFER_SRC;
        }
        if usage.contains(BufferUsage::TRANSFER_DST) {
            buffer_usage_flags |= BufferUsageFlags::TRANSFER_DST;
        }
        if usage.contains(BufferUsage::UNIFORM) {
            buffer_usage_flags |= BufferUsageFlags::UNIFORM_BUFFER;
        }
        if usage.contains(BufferUsage::STORAGE) {
            buffer_usage_flags |= BufferUsageFlags::STORAGE_BUFFER;
        }
        if usage.contains(BufferUsage::VERTEX) {
            buffer_usage_flags |= BufferUsageFlags::VERTEX_BUFFER;
        }
        if usage.contains(BufferUsage::INDEX) {
            buffer_usage_flags |= BufferUsageFlags::INDEX_BUFFER;
        }
        if usage.contains(BufferUsage::INDIRECT) {
            buffer_usage_flags |= BufferUsageFlags::INDIRECT_BUFFER;
        }

        assert!(!buffer_usage_flags.is_empty());

        let info = BufferCreateInfo::default()
            // - `size` must be greater than 0.
            .size(size.get())
            // - `usage` must not be 0. (Unless `VkBufferUsageFlags2CreateInfo` is used.)
            // Checked above.
            .usage(buffer_usage_flags)
            .sharing_mode(SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&info, None).unwrap() };
        Buffer {
            buffer,
            device: self.device.clone(),
            memory: None,
            size: size.get(),
        }
    }

    pub fn allocate_memory(&self, size: NonZeroU64, memory_type_index: u32) -> DeviceMemory {
        // TODO: If the protectedMemory feature is not enabled, the VkMemoryAllocateInfo::memoryTypeIndex must not indicate a memory type that reports VK_MEMORY_PROPERTY_PROTECTED_BIT.
        let info = MemoryAllocateInfo::default()
            // - `allocationSize` must be greater than 0.
            .allocation_size(size.get())
            .memory_type_index(memory_type_index);

        let memory = unsafe { self.device.allocate_memory(&info, None).unwrap() };
        DeviceMemory {
            memory,
            device: self.device.clone(),
            size,
        }
    }

    pub fn buffer_memory_requirements(&self, buffer: &Buffer) -> MemoryRequirements {
        let req = unsafe { self.device.get_buffer_memory_requirements(buffer.buffer) };

        // Bit `i` is set iff the memory type at index `i` is
        // supported for this buffer.
        let mut memory_types = Vec::new();
        let mut bits = req.memory_type_bits;
        while bits != 0 {
            let index = bits.trailing_zeros();
            memory_types.push(index);
            bits &= !(1 << index);
        }

        // Since buffer with size 0 are forbidden, the size/align
        // of any buffer is not 0.
        debug_assert!(req.size > 0);
        debug_assert!(req.alignment > 0);

        MemoryRequirements {
            size: unsafe { NonZeroU64::new_unchecked(req.size) },
            align: unsafe { NonZeroU64::new_unchecked(req.alignment) },
            memory_types,
        }
    }

    pub fn image_memory_requirements(&self, texture: &Texture) -> MemoryRequirements {
        let req = unsafe { self.device.get_image_memory_requirements(texture.image) };

        // Bit `i` is set iff the memory type at index `i` is
        // supported for this buffer.
        let mut memory_types = Vec::new();
        let mut bits = req.memory_type_bits;
        while bits != 0 {
            let index = bits.trailing_zeros();
            memory_types.push(index);
            bits &= !(1 << index);
        }

        debug_assert!(req.size > 0);
        debug_assert!(req.alignment > 0);

        MemoryRequirements {
            size: unsafe { NonZeroU64::new_unchecked(req.size) },
            align: unsafe { NonZeroU64::new_unchecked(req.alignment) },
            memory_types,
        }
    }

    pub fn bind_buffer_memory(&self, buffer: &mut Buffer, memory: DeviceMemory) {
        let info = BindBufferMemoryInfo::default()
            .buffer(buffer.buffer)
            .memory(memory.memory);

        unsafe {
            self.device.bind_buffer_memory2(&[info]).unwrap();
        }

        buffer.memory = Some(memory);
    }

    pub fn bind_texture_memory(&self, texture: &mut Texture, memory: DeviceMemory) {
        let info = vk::BindImageMemoryInfo::default()
            .image(texture.image)
            .memory(memory.memory);

        unsafe {
            self.device.bind_image_memory2(&[info]).unwrap();
        }

        texture.memory = Some(memory);
    }

    pub unsafe fn map_memory(&self, memory: &DeviceMemory) -> &mut [u8] {
        let data = unsafe {
            self.device
                .map_memory(memory.memory, 0, memory.size.get(), MemoryMapFlags::empty())
                .unwrap()
        };

        let len = memory.size.get() as usize;
        unsafe { core::slice::from_raw_parts_mut(data.cast::<u8>(), len) }
    }

    pub fn create_texture(&self, descriptor: &TextureDescriptor) -> Texture {
        let extent = vk::Extent3D::default()
            .width(descriptor.size.x)
            .height(descriptor.size.y)
            .depth(1);

        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(descriptor.format.into())
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::empty());

        let image = unsafe { self.device.create_image(&info, None).unwrap() };
        Texture {
            device: self.clone(),
            image,
            format: descriptor.format,
            size: descriptor.size,
            memory: None,
            destroy_on_drop: true,
        }
    }

    pub unsafe fn create_shader(&self, code: &[u32]) -> ShaderModule<'_> {
        // Code size must be greater than 0.
        assert!(code.len() != 0);

        let info = ShaderModuleCreateInfo::default().code(code);

        let shader = unsafe { self.device.create_shader_module(&info, None).unwrap() };
        ShaderModule {
            device: self,
            shader,
        }
    }

    pub fn create_descriptor_layout(
        &self,
        descriptor: &DescriptorSetDescriptor<'_>,
    ) -> DescriptorSetLayout<'_> {
        let mut bindings = Vec::new();
        for binding in descriptor.bindings {
            let info = DescriptorSetLayoutBinding::default()
                .binding(binding.binding)
                .stage_flags(binding.visibility.into())
                .descriptor_count(1)
                .descriptor_type(binding.kind.into());

            bindings.push(info);
        }

        let info = DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let layout = unsafe {
            self.device
                .create_descriptor_set_layout(&info, None)
                .unwrap()
        };

        DescriptorSetLayout {
            device: &self.device,
            layout,
            bindings: descriptor.bindings.to_vec(),
        }
    }

    pub fn create_pipeline(&self, descriptor: &PipelineDescriptor<'_>) -> Pipeline<'_> {
        let descriptors = descriptor
            .descriptors
            .iter()
            .map(|layout| layout.layout)
            .collect::<Vec<_>>();

        let pipeline_layout_info = PipelineLayoutCreateInfo::default().set_layouts(&descriptors);
        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        };

        let mut stages = Vec::new();
        let mut color_attchment_formats: Vec<Format> = Vec::new();

        for stage in descriptor.stages {
            let vk_stage = match stage {
                PipelineStage::Vertex(stage) => PipelineShaderStageCreateInfo::default()
                    .stage(ShaderStageFlags::VERTEX)
                    .module(stage.shader.shader)
                    .name(c"main"),
                PipelineStage::Fragment(stage) => {
                    color_attchment_formats.extend(stage.targets.iter().copied().map(Format::from));

                    PipelineShaderStageCreateInfo::default()
                        .stage(ShaderStageFlags::FRAGMENT)
                        .module(stage.shader.shader)
                        .name(c"main")
                }
            };

            stages.push(vk_stage);
        }

        let vertex_input_state = PipelineVertexInputStateCreateInfo::default();

        let input_assembly_state = PipelineInputAssemblyStateCreateInfo::default()
            .topology(descriptor.topology.into())
            .primitive_restart_enable(false);

        // We use dynamic viewport and scissors, so the actual viewport and scissors
        // pointers are ignored. We still have to enter the correct count of viewport/
        // scissors.
        let viewport_state = PipelineViewportStateCreateInfo::default()
            // - `viewportCount` must be less than or equal to `VkPhysicalDeviceLimits::maxViewports`.
            // - `viewportCount` must not be greater than 1. (If `multiViewport` feature is not enabled.)
            // - `viewportCount` must be greater than 0. (If `VK_DYNAMIC_STATE_VIEWPORT_WITH_COUNT` not set.)
            .viewport_count(1)
            // - `scissorCount` must be less than or eual to `VkPhysicalDeviceLimits::maxViewports`.
            // - `scissorCount` must not be greater than 1. (If `multiViewport` feature is not enabled.)
            // - `scissorCount` must be greater than 0. (If `VK_DYNAMIC_STATE_SCISSOR_WITH_COUNT` not set.)
            .scissor_count(1);

        let cull_mode = match descriptor.cull_mode {
            Some(Face::Front) => CullModeFlags::FRONT,
            Some(Face::Back) => CullModeFlags::BACK,
            None => CullModeFlags::NONE,
        };

        let rasterization_state = PipelineRasterizationStateCreateInfo::default()
            .depth_bias_enable(true)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(cull_mode)
            .front_face(descriptor.front_face.into());

        let multisample_state = PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1);

        let attachment = PipelineColorBlendAttachmentState::default()
            .color_write_mask(ColorComponentFlags::RGBA)
            .blend_enable(false)
            .src_color_blend_factor(BlendFactor::ONE)
            .dst_color_blend_factor(BlendFactor::ZERO)
            .color_blend_op(BlendOp::ADD)
            .src_alpha_blend_factor(BlendFactor::ONE)
            .dst_alpha_blend_factor(BlendFactor::ZERO)
            .alpha_blend_op(BlendOp::ADD);

        let attachments = &[attachment];
        let color_blend_state = PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let dynamic_state = PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[DynamicState::VIEWPORT, DynamicState::SCISSOR]);

        let mut rendering_info = PipelineRenderingCreateInfo::default()
            // - `colorAttachmentCount` must be less than `VkPhysicalDeviceLimits::maxColorAttachments`.
            .color_attachment_formats(&color_attchment_formats);

        let info = GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(pipeline_layout)
            .dynamic_state(&dynamic_state)
            // Not needed since we are using VK_KHR_dynamic_rendering.
            .render_pass(vk::RenderPass::null())
            .subpass(0)
            .push_next(&mut rendering_info);

        let pipelines = unsafe {
            self.device
                .create_graphics_pipelines(PipelineCache::null(), &[info], None)
                .unwrap()
        };

        Pipeline {
            device: self,
            pipeline: pipelines[0],
            pipeline_layout,
        }
    }

    pub fn create_command_pool(&self) -> CommandPool<'_> {
        let info = CommandPoolCreateInfo::default()
            .flags(CommandPoolCreateFlags::empty())
            .queue_family_index(self.queue_family_index);

        let pool = unsafe { self.device.create_command_pool(&info, None).unwrap() };

        let info = CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let buffers = unsafe { self.device.allocate_command_buffers(&info).unwrap() };

        CommandPool {
            device: self,
            pool,
            buffers,
            next_buffer: 0,
        }
    }

    pub fn create_semaphore(&self) -> Semaphore {
        let info = SemaphoreCreateInfo::default();

        let semaphore = unsafe { self.device.create_semaphore(&info, None).unwrap() };

        Semaphore {
            device: self.device.clone(),
            semaphore,
        }
    }

    pub fn create_descriptor_pool(
        &self,
        descriptor: &DescriptorPoolDescriptor,
    ) -> DescriptorPool<'_> {
        let mut sizes = Vec::new();

        for (ty, count) in [
            (
                vk::DescriptorType::UNIFORM_BUFFER,
                descriptor.max_uniform_buffers,
            ),
            (
                vk::DescriptorType::STORAGE_BUFFER,
                descriptor.max_storage_buffers,
            ),
        ] {
            if count == 0 {
                continue;
            }

            // - `descriptorCount` must be greater than 0.
            let size = DescriptorPoolSize::default().ty(ty).descriptor_count(count);

            sizes.push(size);
        }

        let info = DescriptorPoolCreateInfo::default()
            .pool_sizes(&sizes)
            // - `maxSets` must be greater than 0.
            .max_sets(descriptor.max_sets.get());

        let pool = unsafe { self.device.create_descriptor_pool(&info, None).unwrap() };

        DescriptorPool {
            device: &self.device,
            pool,
        }
    }

    pub fn create_fence(&self) -> Fence {
        let info = FenceCreateInfo::default();

        let fence = unsafe { self.device.create_fence(&info, None).unwrap() };
        Fence {
            device: self.device.clone(),
            fence,
        }
    }

    pub fn create_sampler(&self, descriptor: &SamplerDescriptor) -> Sampler {
        let info = vk::SamplerCreateInfo::default()
            .min_filter(descriptor.min_filter.into())
            .mag_filter(descriptor.mag_filter.into())
            .address_mode_u(descriptor.address_mode_u.into())
            .address_mode_v(descriptor.address_mode_v.into())
            .address_mode_w(descriptor.address_mode_w.into())
            // TODO: Add API for this
            .anisotropy_enable(false)
            .max_anisotropy(1.0);

        let sampler = unsafe { self.device.create_sampler(&info, None).unwrap() };
        Sampler {
            device: self.device.clone(),
            sampler,
        }
    }
}

pub struct Queue {
    device: Arc<DeviceShared>,
    queue: vk::Queue,
}

impl Queue {
    pub fn submit<'a, T>(&mut self, buffers: T, cmd: QueueSubmit<'_>)
    where
        T: Iterator<Item = CommandBuffer<'a>>,
    {
        let buffers: Vec<_> = buffers.map(|buf| buf.buffer).collect();
        let wait_semaphores: Vec<_> = cmd
            .wait
            .iter()
            .map(|semaphore| semaphore.semaphore)
            .collect();
        let wait_stages: Vec<_> = std::iter::repeat_n(cmd.wait_stage, cmd.wait.len()).collect();
        let signal_semaphores: Vec<_> = cmd
            .signal
            .iter()
            .map(|semaphore| semaphore.semaphore)
            .collect();

        let info = SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            self.device
                .device
                .queue_submit(self.queue, &[info], vk::Fence::null())
                .unwrap();
        }
    }

    pub fn wait_idle(&mut self) {
        unsafe {
            // - Access to `queue` must be externally synchronized.
            self.device.device.queue_wait_idle(self.queue).unwrap();
        }
    }
}

pub struct Surface {
    instance: Arc<InstanceShared>,
    surface: SurfaceKHR,
}

impl Surface {
    pub fn get_capabilities(&self, device: &Device) -> SwapchainCapabilities {
        let instance =
            ash::khr::surface::Instance::new(&self.instance.entry, &self.instance.instance);

        let is_supported = unsafe {
            instance
                .get_physical_device_surface_support(
                    device.physical_device,
                    device.queue_family_index,
                    self.surface,
                )
                .unwrap()
        };

        if !is_supported {
            todo!()
        }

        let caps = unsafe {
            instance
                .get_physical_device_surface_capabilities(device.physical_device, self.surface)
                .unwrap()
        };
        let formats = unsafe {
            instance
                .get_physical_device_surface_formats(device.physical_device, self.surface)
                .unwrap()
        };
        let present_modes = unsafe {
            instance
                .get_physical_device_surface_present_modes(device.physical_device, self.surface)
                .unwrap()
        };

        // Vulkan spec requires that `maxImageArrayLayers` is at least one.
        debug_assert!(caps.max_image_array_layers >= 1);

        // Vulkan spec requires that `VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT` must be included.
        debug_assert!(caps
            .supported_usage_flags
            .contains(ImageUsageFlags::COLOR_ATTACHMENT));

        // FIXME: This does not seem strictly required by the Vulkan spec?
        // See https://github.com/KhronosGroup/Vulkan-Docs/issues/2440
        assert!(caps.supported_transforms.contains(caps.current_transform));

        SwapchainCapabilities {
            min_extent: UVec2 {
                x: caps.min_image_extent.width,
                y: caps.min_image_extent.height,
            },
            max_extent: UVec2 {
                x: caps.max_image_extent.width,
                y: caps.max_image_extent.height,
            },
            min_images: caps.min_image_count,
            max_images: NonZeroU32::new(caps.max_image_count),
            // FIXME: What to do about color space?
            // It is probably always SRGB_NONLINEAR.
            formats: formats
                .into_iter()
                .filter_map(|v| v.format.try_into().ok())
                .collect(),
            present_modes: present_modes
                .into_iter()
                .filter_map(|v| v.try_into().ok())
                .collect(),
            current_transform: caps.current_transform,
            supported_composite_alpha: caps.supported_composite_alpha,
        }
    }

    pub fn create_swapchain(
        &self,
        device: &Device,
        config: SwapchainConfig,
        caps: &SwapchainCapabilities,
    ) -> Swapchain<'_> {
        // SAFETY: `old_swapchain` is null.
        let (swapchain, images) =
            unsafe { self.create_swapchain_inner(device, &config, &caps, SwapchainKHR::null()) };

        Swapchain {
            surface: self,
            device: device.clone(),
            swapchain,
            images,
            format: config.format,
            extent: config.extent,
        }
    }

    /// Creates a new [`SwapchainKHR`] and returns its images.
    ///
    /// # Safety
    ///
    /// `old_swapchain` must be either null or a non-retired swapchain created by this `Surface`.
    unsafe fn create_swapchain_inner(
        &self,
        device: &Device,
        config: &SwapchainConfig,
        caps: &SwapchainCapabilities,
        old_swapchain: SwapchainKHR,
    ) -> (SwapchainKHR, Vec<vk::Image>) {
        // See https://registry.khronos.org/vulkan/specs/latest/man/html/VkSwapchainCreateInfoKHR.html
        // `imageExtent` members `width` and `height` must both be non-zero.
        assert_ne!(config.extent.x, 0);
        assert_ne!(config.extent.y, 0);
        assert!(config.extent.x >= caps.min_extent.x && config.extent.x <= caps.max_extent.x);
        assert!(config.extent.y >= caps.min_extent.y && config.extent.y <= caps.max_extent.y);

        assert!(config.image_count <= caps.max_images.unwrap_or(NonZeroU32::MAX).get());
        assert!(config.image_count >= caps.min_images);

        // TODO: Handle case where `OPAQUE` is not supported.
        assert!(caps
            .supported_composite_alpha
            .contains(CompositeAlphaFlagsKHR::OPAQUE));

        assert!(caps.present_modes.contains(&config.present_mode));

        assert!(caps.formats.contains(&config.format));

        let queue_family_indices = [device.queue_family_index];

        let info = SwapchainCreateInfoKHR::default()
            // - Surface must be supported. This is checked by the call to `get_capabilities` above.
            .surface(self.surface)
            // - `minImageCount` must be less than or equal to the `maxImageCount`. Checked above.
            // - `minImageCount` must be greater than or equal to `minImageCount`. Checked above.
            .min_image_count(config.image_count)
            // - `imageFormat` must match one of the formats returned by `vkGetPhysicalDeviceSurfaceFormatsKHR`.
            // Checked above.
            .image_format(config.format.into())
            // TODO: Unchecked
            .image_color_space(ColorSpaceKHR::SRGB_NONLINEAR)
            // - `width` and `height` must both ne non-zero. Checked above.
            // - `width` and `height` must be between `minImageExtent` and `maxImageExtent`. Checked above.
            .image_extent(Extent2D {
                width: config.extent.x,
                height: config.extent.y,
            })
            // - `imageArrayLayers` must be at least 1 and less than or equal to `maxImageArrayLayers`.
            // `vkGetPhysicalDeviceSurfaceCapabilitiesKHR` is required to always return at least 1.
            // This means the value `1` is always valid here.
            .image_array_layers(1)
            // - `imageUsage` must be a set of `supportedUsageFlags`.
            // `VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT` must always be included, so this value is always valid.
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices)
            // - `compositeAlpha` must be one bit from `supportedCompositeAlpha`. Checked above.
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            // - `preTransform` must be one bit from `supportedTransforms`.
            .pre_transform(caps.current_transform)
            // - `presentMode` must be one of the values returned by `vkGetPhysicalDeviceSurfacePresentModesKHR`.
            // Checked above.
            .present_mode(config.present_mode.into())
            // Whether Vulkan is allowed to discard pixels of the surface that are not visible.
            // Since we do not need to read back the swapchain images we do not care about the
            // discarded pixels.
            .clipped(true)
            // - `oldSwapchain` must be null or a non-retired swapchain.
            // This is guaranteed by the caller.
            .old_swapchain(old_swapchain);

        let khr_device = ash::khr::swapchain::Device::new(&self.instance.instance, &device.device);
        let swapchain = unsafe { khr_device.create_swapchain(&info, None).unwrap() };

        let images = unsafe { khr_device.get_swapchain_images(swapchain).unwrap() };

        (swapchain, images)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        let instance =
            ash::khr::surface::Instance::new(&self.instance.entry, &self.instance.instance);

        unsafe {
            instance.destroy_surface(self.surface, None);
        }
    }
}

pub struct Swapchain<'a> {
    surface: &'a Surface,
    device: Device,
    swapchain: SwapchainKHR,
    images: Vec<vk::Image>,

    format: TextureFormat,
    extent: UVec2,
}

impl<'a> Swapchain<'a> {
    pub fn recreate(&mut self, config: SwapchainConfig, caps: &SwapchainCapabilities) {
        // SAFETY: `self.swapchain` is a valid swapchain created by `self.surface`.
        // Since this function accepts a mutable reference this swapchain is not used.
        let (swapchain, images) = unsafe {
            self.surface
                .create_swapchain_inner(&self.device, &config, caps, self.swapchain)
        };

        // The swapchain still needs to be destroyed after it has been invalidated.
        unsafe {
            let device = ash::khr::swapchain::Device::new(
                &self.surface.instance.instance,
                &self.device.device,
            );
            device.destroy_swapchain(self.swapchain, None);
        }

        self.swapchain = swapchain;
        self.images = images;
        self.format = config.format;
        self.extent = config.extent;
    }

    // FIXME: Should be &mut self.
    pub fn acquire_next_image(&self, semaphore: &mut Semaphore) -> SwapchainTexture<'_> {
        let device =
            ash::khr::swapchain::Device::new(&self.device.device.instance, &self.device.device);

        let (image_index, suboptimal) = unsafe {
            device
                .acquire_next_image(
                    self.swapchain,
                    u64::MAX,
                    semaphore.semaphore,
                    vk::Fence::null(),
                )
                .unwrap()
        };

        SwapchainTexture {
            texture: Texture {
                device: self.device.clone(),
                image: self.images[image_index as usize],
                format: self.format,
                size: self.extent,
                memory: None,
                destroy_on_drop: false,
            },
            suboptimal,
            index: image_index,
            device: &self.device,
            swapchain: self,
        }
    }

    pub fn present(&self, queue: &Queue, img: u32, wait_semaphore: &Semaphore) {
        let device =
            ash::khr::swapchain::Device::new(&self.device.device.instance, &self.device.device);

        let wait_semaphores = &[wait_semaphore.semaphore];

        let swapchains = &[self.swapchain];
        let image_indices = &[img];
        let info = PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        unsafe {
            device.queue_present(queue.queue, &info).unwrap();
        }
    }
}

impl<'a> Drop for Swapchain<'a> {
    fn drop(&mut self) {
        let device =
            ash::khr::swapchain::Device::new(&self.surface.instance.instance, &self.device.device);
        unsafe {
            device.destroy_swapchain(self.swapchain, None);
        }
    }
}

impl TryFrom<PresentModeKHR> for PresentMode {
    type Error = UnknownEnumValue;

    fn try_from(value: PresentModeKHR) -> Result<Self, Self::Error> {
        match value {
            PresentModeKHR::FIFO => Ok(PresentMode::Fifo),
            PresentModeKHR::IMMEDIATE => Ok(PresentMode::Immediate),
            PresentModeKHR::FIFO_RELAXED => Ok(PresentMode::FifoRelaxed),
            PresentModeKHR::MAILBOX => Ok(PresentMode::Mailbox),
            _ => Err(UnknownEnumValue),
        }
    }
}

impl From<PresentMode> for PresentModeKHR {
    fn from(value: PresentMode) -> Self {
        match value {
            PresentMode::Fifo => PresentModeKHR::FIFO,
            PresentMode::Immediate => PresentModeKHR::IMMEDIATE,
            PresentMode::FifoRelaxed => PresentModeKHR::FIFO_RELAXED,
            PresentMode::Mailbox => PresentModeKHR::MAILBOX,
        }
    }
}

impl TryFrom<Format> for TextureFormat {
    type Error = UnknownEnumValue;

    fn try_from(value: Format) -> Result<Self, Self::Error> {
        match value {
            Format::R8G8B8A8_UNORM => Ok(Self::R8G8B8A8Unorm),
            Format::R8G8B8A8_SRGB => Ok(Self::R8G8B8A8UnormSrgb),
            Format::B8G8R8A8_UNORM => Ok(Self::B8G8R8A8Unorm),
            Format::B8G8R8A8_SRGB => Ok(Self::B8G8R8A8UnormSrgb),
            _ => Err(UnknownEnumValue),
        }
    }
}

impl From<TextureFormat> for Format {
    fn from(value: TextureFormat) -> Self {
        match value {
            TextureFormat::R8G8B8A8Unorm => Self::R8G8B8A8_UNORM,
            TextureFormat::R8G8B8A8UnormSrgb => Self::R8G8B8A8_SRGB,
            TextureFormat::B8G8R8A8Unorm => Self::B8G8R8A8_SNORM,
            TextureFormat::B8G8R8A8UnormSrgb => Self::B8G8R8A8_SRGB,
        }
    }
}

impl TryFrom<PrimitiveTopology> for super::PrimitiveTopology {
    type Error = UnknownEnumValue;

    fn try_from(value: PrimitiveTopology) -> Result<Self, Self::Error> {
        match value {
            PrimitiveTopology::TRIANGLE_LIST => Ok(Self::TriangleList),
            _ => Err(UnknownEnumValue),
        }
    }
}

impl From<super::PrimitiveTopology> for PrimitiveTopology {
    fn from(value: super::PrimitiveTopology) -> Self {
        match value {
            super::PrimitiveTopology::TriangleList => Self::TRIANGLE_LIST,
        }
    }
}

impl From<super::FrontFace> for FrontFace {
    fn from(value: super::FrontFace) -> Self {
        match value {
            super::FrontFace::Cw => Self::CLOCKWISE,
            super::FrontFace::Ccw => Self::COUNTER_CLOCKWISE,
        }
    }
}

impl From<super::DescriptorType> for vk::DescriptorType {
    fn from(value: super::DescriptorType) -> Self {
        match value {
            super::DescriptorType::Uniform => Self::UNIFORM_BUFFER,
            super::DescriptorType::Storage => Self::STORAGE_BUFFER,
            super::DescriptorType::Sampler => Self::SAMPLER,
        }
    }
}

impl From<ShaderStages> for ShaderStageFlags {
    fn from(value: ShaderStages) -> Self {
        let mut flags = ShaderStageFlags::empty();

        if value.contains(ShaderStages::VERTEX) {
            flags |= ShaderStageFlags::VERTEX;
        }
        if value.contains(ShaderStages::FRAGMENT) {
            flags |= ShaderStageFlags::FRAGMENT;
        }

        flags
    }
}

impl From<TextureLayout> for vk::ImageLayout {
    fn from(value: TextureLayout) -> Self {
        match value {
            TextureLayout::Undefined => vk::ImageLayout::UNDEFINED,
            TextureLayout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            TextureLayout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
            TextureLayout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            TextureLayout::ShaderRead => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        }
    }
}

impl From<FilterMode> for vk::Filter {
    fn from(value: FilterMode) -> Self {
        match value {
            FilterMode::Nearest => vk::Filter::NEAREST,
            FilterMode::Linear => vk::Filter::LINEAR,
        }
    }
}

impl From<AddressMode> for vk::SamplerAddressMode {
    fn from(value: AddressMode) -> Self {
        match value {
            AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            AddressMode::MirrorRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            AddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }
}

pub struct ShaderModule<'a> {
    device: &'a Device,
    shader: vk::ShaderModule,
}

impl<'a> Drop for ShaderModule<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_shader_module(self.shader, None);
        }
    }
}

pub struct Pipeline<'a> {
    device: &'a Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl<'a> Drop for Pipeline<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_pipeline(self.pipeline, None);
            self.device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

pub struct CommandPool<'a> {
    device: &'a Device,
    pool: vk::CommandPool,
    buffers: Vec<vk::CommandBuffer>,
    /// Index of the next buffer.
    next_buffer: usize,
}

impl<'a> CommandPool<'a> {
    /// Acquires a new [`CommandEncoder`] from this `CommandPool`.
    pub fn create_encoder(&mut self) -> Option<CommandEncoder<'_>> {
        let inheritance = CommandBufferInheritanceInfo::default();

        let info = CommandBufferBeginInfo::default()
            .flags(CommandBufferUsageFlags::empty())
            .inheritance_info(&inheritance);

        let buffer = *self.buffers.get(self.next_buffer)?;
        self.next_buffer += 1;

        // Move the buffer into the recording state.
        // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#vkBeginCommandBuffer
        // Safety:
        // - Buffer must not be in the initial state.
        // - Access to command buffer and command pool must be externally synchronized. (Asserted by
        // exclusive access.)
        unsafe {
            self.device
                .device
                .begin_command_buffer(buffer, &info)
                .unwrap();
        }

        Some(CommandEncoder {
            device: self.device,
            pool: self,
            buffer,
        })
    }

    /// Resets all command buffers in the pool.
    ///
    /// # Safety
    ///
    /// This operation invalidates all buffers created by [`create_encoder`]. All submissions using
    /// buffers from this `CommandPool` must have completed.
    pub unsafe fn reset(&mut self) {
        // Reset the pool and all buffers.
        // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#vkResetCommandPool
        // Safety:
        // - All buffers must NOT be in the pending state. (Guaranteed by caller.)
        // - Access to command pool must be externally synchronized. (Asserted by exclusive access.)
        unsafe {
            self.device
                .device
                .reset_command_pool(self.pool, CommandPoolResetFlags::empty())
                .unwrap();
        }

        self.next_buffer = 0;
    }
}

impl<'a> Drop for CommandPool<'a> {
    fn drop(&mut self) {
        // Deallocate the command buffers of this pool:
        // https://registry.khronos.org/vulkan/specs/latest/man/html/vkFreeCommandBuffers.html
        // Safety:
        // - All buffers must NOT be in the pending state.
        // - VkDevice and VKCommandPool must valid.
        // - Number of buffers must be greater than 0. (Asserted by the CommandPool constructor.)
        // - Access to all buffers and command pool must be externally synchronized. (Asserted by
        // exclusive access.)
        debug_assert!(self.buffers.len() > 0);
        unsafe {
            self.device
                .device
                .free_command_buffers(self.pool, &self.buffers);
        }

        // Destroy the command pool:
        // https://registry.khronos.org/vulkan/specs/latest/man/html/vkDestroyCommandPool.html
        // Safety:
        // - All buffers allocated with the pool must be pending. (Asserted by previous deallocation
        // of all buffers.)
        // - Access to command pool must be externally synchronized. (Asserted by exclusive access.)
        unsafe {
            self.device.device.destroy_command_pool(self.pool, None);
        }
    }
}

pub struct CommandEncoder<'a> {
    device: &'a Device,
    pool: &'a CommandPool<'a>,
    buffer: vk::CommandBuffer,
}

impl<'a> CommandEncoder<'a> {
    /// Copy `count` bytes from `src` to `dst`.
    pub fn copy_buffer_to_buffer(
        &mut self,
        src: &Buffer,
        src_offset: u64,
        dst: &Buffer,
        dst_offset: u64,
        count: u64,
    ) {
        if count == 0 {
            return;
        }

        if src_offset > src.size || src.size - src_offset < count {
            panic!(
                "invalid copy_buffer op: bad access of {:?} for src buffer {:?}",
                src_offset..src_offset + count,
                0..src.size,
            );
        }

        if dst_offset > dst.size || dst.size - dst_offset < count {
            panic!(
                "invalid copy_buffer op: bad access of {:?} for dst buffer {:?}",
                dst_offset..dst_offset + count,
                0..dst.size,
            );
        }

        // Overlapping values are undefined.
        if src.buffer == dst.buffer {
            let src_end = src_offset + count;
            let dst_end = dst_offset + count;
            if src_offset < dst_end && dst_offset < src_end {
                panic!(
                    "invalid copy_buffer op: overlapping ranges (src={:?}, dst={:?})",
                    src_offset..src_end,
                    dst_offset..dst_end,
                );
            }
        }

        let region = vk::BufferCopy::default()
            .src_offset(src_offset)
            .dst_offset(dst_offset)
            // - `size` must be greater than 0.
            .size(count);

        unsafe {
            self.device
                .device
                .cmd_copy_buffer(self.buffer, src.buffer, dst.buffer, &[region]);
        }
    }

    pub fn copy_buffer_to_texture(&mut self, src: CopyBuffer<'_>, dst: &Texture) {
        assert_ne!(dst.size.x, 0);
        assert_ne!(dst.size.y, 0);

        assert!(src.buffer.memory.is_some());
        assert!(dst.memory.is_some());

        let bytes_to_copy = src.layout.bytes_per_row as u64 * src.layout.rows_per_image as u64;
        assert!(src.buffer.size > src.offset);
        assert!(src.buffer.size - src.offset >= bytes_to_copy);

        let subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1);

        let region = vk::BufferImageCopy2::default()
            .buffer_offset(src.offset)
            // - `bufferRowLength` must be 0, or greater than or equal to `width` of `imageExtent`.
            .buffer_row_length(dst.size.x)
            //.buffer_row_length(0)
            // - `bufferImageHeight` must be 0, or greater than or equal to `height` of `imageExtent`.
            .buffer_image_height(dst.size.y)
            //.buffer_image_height(0)
            .image_subresource(subresource)
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                // - `imageExtent.width` must not be 0.
                width: dst.size.x,
                // - `imageExtent.height` must not be 0.
                height: dst.size.y,
                // - `imageExtent.depth` must not be 0.
                depth: 1,
            });

        let regions = &[region];

        let info = vk::CopyBufferToImageInfo2::default()
            .src_buffer(src.buffer.buffer)
            .dst_image(dst.image)
            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .regions(regions);

        unsafe {
            self.device
                .device
                .cmd_copy_buffer_to_image2(self.buffer, &info);
        }
    }

    pub fn begin_render_pass<'res>(
        &mut self,
        descriptor: &RenderPassDescriptor<'_, 'res>,
    ) -> RenderPass<'_, 'res> {
        let mut extent = UVec2::ZERO;

        let mut color_attachments = Vec::new();
        for attachment in descriptor.color_attachments {
            let load_op = match attachment.load_op {
                LoadOp::Load => AttachmentLoadOp::LOAD,
                LoadOp::Clear(color) => AttachmentLoadOp::CLEAR,
            };

            let store_op = match attachment.store_op {
                StoreOp::Discard => AttachmentStoreOp::NONE,
                StoreOp::Store => AttachmentStoreOp::STORE,
            };

            let clear_value = match attachment.load_op {
                LoadOp::Clear(color) => ClearValue {
                    color: ClearColorValue { float32: color },
                },
                LoadOp::Load => ClearValue::default(),
            };

            let info = RenderingAttachmentInfo::default()
                .image_view(attachment.view.view)
                .image_layout(attachment.layout)
                .resolve_mode(ResolveModeFlags::NONE)
                .load_op(load_op)
                .store_op(store_op)
                .clear_value(clear_value);

            color_attachments.push(info);
            extent = UVec2::max(extent, attachment.size);
        }

        let info = RenderingInfo::default()
            .flags(RenderingFlags::empty())
            .render_area(Rect2D {
                offset: Offset2D { x: 0, y: 0 },
                extent: Extent2D {
                    width: extent.x,
                    height: extent.y,
                },
            })
            .layer_count(1)
            .view_mask(0)
            .color_attachments(&color_attachments);

        unsafe {
            self.device.device.cmd_begin_rendering(self.buffer, &info);
        }

        // Since we have created the pipeline with `VK_DYNAMIC_STATE_VIEWPORT` and
        // `VK_DYNAMIC_STATE_SCISSOR` we must set the the viewport and scissors
        // before any draw opertaions.
        let viewport = Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.x as f32,
            height: extent.y as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = Rect2D {
            offset: Offset2D { x: 0, y: 0 },
            extent: Extent2D {
                width: extent.x,
                height: extent.y,
            },
        };
        unsafe {
            self.device
                .device
                .cmd_set_viewport(self.buffer, 0, &[viewport]);
            self.device
                .device
                .cmd_set_scissor(self.buffer, 0, &[scissor]);
        }

        RenderPass {
            encoder: self,
            _marker: PhantomData,
            pipeline: None,
        }
    }

    pub fn insert_pipeline_barriers(&mut self, barriers: &PipelineBarriers<'_>) {
        let mut image_barriers = Vec::new();
        for barrier in barriers.texture {
            // Images cannot be transitioned into `UNDEFINED`.
            assert_ne!(barrier.new_layout, TextureLayout::Undefined);

            let subresource_range = ImageSubresourceRange::default()
                .aspect_mask(ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let barrier = vk::ImageMemoryBarrier2::default()
                // FIXME: More control over these flags.
                .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .src_access_mask(barrier.src_access_flags)
                .dst_access_mask(barrier.dst_access_flags)
                .old_layout(barrier.old_layout.into())
                .new_layout(barrier.new_layout.into())
                // Do not transfer between queues.
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(barrier.texture.image)
                .subresource_range(subresource_range);
            image_barriers.push(barrier);
        }

        let info = vk::DependencyInfo::default()
            .dependency_flags(DependencyFlags::empty())
            .image_memory_barriers(&image_barriers);

        unsafe {
            self.device.device.cmd_pipeline_barrier2(self.buffer, &info);
        }
    }

    pub fn finish(self) -> CommandBuffer<'a> {
        unsafe {
            self.device.device.end_command_buffer(self.buffer).unwrap();
        }

        CommandBuffer {
            device: self.device,
            buffer: self.buffer,
        }
    }
}

pub struct RenderPass<'encoder, 'resources> {
    encoder: &'encoder CommandEncoder<'encoder>,
    // Marker to indicate that all resources that this render pass
    // may access must not be dropped while this render pass exists.
    _marker: PhantomData<fn() -> &'resources ()>,
    pipeline: Option<&'resources Pipeline<'resources>>,
}

impl<'encoder, 'resources> RenderPass<'encoder, 'resources> {
    pub fn bind_pipeline(&mut self, pipeline: &'resources Pipeline<'resources>) {
        // Bind the pipeline.
        // https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdBindPipeline.html
        // Safety:
        // - Since we are using `GRAPHICS`, the pipeline must be a graphics pipeline.
        // - Since we are using `GRAPHICS`, the command buffer must support graphics
        // operations.
        unsafe {
            self.encoder.device.device.cmd_bind_pipeline(
                self.encoder.buffer,
                PipelineBindPoint::GRAPHICS,
                pipeline.pipeline,
            );
        }

        self.pipeline = Some(&pipeline);
    }

    pub fn bind_descriptor_set(&mut self, slot: u32, descriptor_set: &DescriptorSet<'_>) {
        let pipeline = self.pipeline.as_ref().unwrap();

        unsafe {
            self.encoder.device.device.cmd_bind_descriptor_sets(
                self.encoder.buffer,
                PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout,
                slot,
                &[descriptor_set.set],
                &[],
            );
        }
    }

    pub fn set_push_constants(&mut self, stages: ShaderStages, offset: u32, data: &[u8]) {
        // `offset` must be a multiple of 4.
        assert_eq!(offset % 4, 0);
        // `size` must be a multiple of 4.
        assert_eq!(data.len() % 4, 0);
        // `offset` must be less than `VkPhysicalDeviceLimits::maxPushConstantsSize`.
        assert!(offset < self.encoder.device.limits.max_push_constants_size);
        // `size` must be less than or equal to `VkPhysicalDeviceLimits::maxPushConstantsSize` minus `offset`.
        assert!(data.len() as u32 <= self.encoder.device.limits.max_push_constants_size - offset);
        // `stageFlags` must not be 0.
        assert_ne!(stages, ShaderStages::empty());
        // `size` must be greater than 0.
        assert_ne!(data.len(), 0);

        let pipeline = self.pipeline.as_ref().unwrap();

        unsafe {
            self.encoder.device.device.cmd_push_constants(
                self.encoder.buffer,
                pipeline.pipeline_layout,
                stages.into(),
                0,
                data,
            );
        }
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        unsafe {
            self.encoder.device.device.cmd_draw(
                self.encoder.buffer,
                vertices.len() as u32,
                instances.len() as u32,
                vertices.start,
                instances.start,
            );
        }
    }
}

impl<'encoder, 'resources> Drop for RenderPass<'encoder, 'resources> {
    fn drop(&mut self) {
        unsafe {
            self.encoder
                .device
                .device
                .cmd_end_rendering(self.encoder.buffer);
        }
    }
}

pub struct CommandBuffer<'a> {
    device: &'a Device,
    buffer: vk::CommandBuffer,
}

#[derive(Debug)]
pub struct Semaphore {
    device: Arc<DeviceShared>,
    semaphore: vk::Semaphore,
}

impl Semaphore {}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

pub struct SwapchainTexture<'a> {
    texture: Texture,
    pub suboptimal: bool,
    index: u32,
    device: &'a Device,
    swapchain: &'a Swapchain<'a>,
}

impl<'a> SwapchainTexture<'a> {
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn present(&self, queue: &Queue, wait_semaphore: &Semaphore) {
        let device =
            ash::khr::swapchain::Device::new(&self.device.device.instance, &self.device.device);

        let wait_semaphores = &[wait_semaphore.semaphore];

        let swapchains = &[self.swapchain.swapchain];
        let image_indices = &[self.index];
        let info = PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        unsafe {
            device.queue_present(queue.queue, &info).unwrap();
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    device: Device,
    image: vk::Image,
    format: TextureFormat,
    size: UVec2,
    memory: Option<DeviceMemory>,
    /// Whether to destroy the texture on drop.
    /// This is only used for swapchain textures.
    destroy_on_drop: bool,
}

impl Texture {
    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn create_view(&self) -> TextureView<'_> {
        let components = ComponentMapping::default()
            .r(ComponentSwizzle::IDENTITY)
            .g(ComponentSwizzle::IDENTITY)
            .b(ComponentSwizzle::IDENTITY)
            .a(ComponentSwizzle::IDENTITY);

        let subresource_range = ImageSubresourceRange::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let info = ImageViewCreateInfo::default()
            .image(self.image)
            .view_type(ImageViewType::TYPE_2D)
            .format(self.format.into())
            .subresource_range(subresource_range)
            .components(components);

        let view = unsafe { self.device.device.create_image_view(&info, None).unwrap() };
        TextureView {
            device: &self.device,
            view,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.destroy_on_drop {
            unsafe {
                self.device.device.destroy_image(self.image, None);
            }
        }
    }
}

pub struct TextureView<'a> {
    device: &'a Device,
    view: vk::ImageView,
}

impl<'a> Drop for TextureView<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_image_view(self.view, None);
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    device: Arc<DeviceShared>,
    buffer: vk::Buffer,
    memory: Option<DeviceMemory>,
    size: u64,
}

impl Buffer {
    pub fn slice<R>(&self, range: R) -> super::BufferView<'_>
    where
        R: RangeBounds<u64>,
    {
        let start = match range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start - 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(end) => *end + 1,
            Bound::Excluded(end) => *end,
            Bound::Unbounded => self.size,
        };

        super::BufferView {
            buffer: self,
            view: start..end,
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}

#[derive(Debug)]
pub struct DeviceMemory {
    device: Arc<DeviceShared>,
    memory: vk::DeviceMemory,
    size: NonZeroU64,
}

impl Drop for DeviceMemory {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.memory, None);
        }
    }
}

pub struct DescriptorSetLayout<'a> {
    device: &'a ash::Device,
    layout: vk::DescriptorSetLayout,
    bindings: Vec<super::DescriptorBinding>,
}

impl<'a> DescriptorSetLayout<'a> {
    pub(crate) fn bindings(&self) -> &[super::DescriptorBinding] {
        &self.bindings
    }
}

impl<'a> Drop for DescriptorSetLayout<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

pub struct DescriptorPool<'a> {
    device: &'a ash::Device,
    pool: vk::DescriptorPool,
}

impl<'a> DescriptorPool<'a> {
    pub fn create_descriptor_set(&mut self, layout: &DescriptorSetLayout<'_>) -> DescriptorSet<'_> {
        let layouts = [layout.layout];

        let info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            // - `descriptorSetCount` must be greater than 0.
            .set_layouts(&layouts);

        let sets = unsafe { self.device.allocate_descriptor_sets(&info).unwrap() };
        DescriptorSet {
            pool: self,
            set: sets[0],
        }
    }

    pub unsafe fn reset(&mut self) {
        unsafe {
            // - `flags` must be 0.
            self.device
                .reset_descriptor_pool(self.pool, DescriptorPoolResetFlags::empty())
                .unwrap();
        }
    }
}

impl<'a> Drop for DescriptorPool<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
        }
    }
}

pub struct DescriptorSet<'a> {
    pool: &'a DescriptorPool<'a>,
    set: vk::DescriptorSet,
}

impl<'a> DescriptorSet<'a> {
    pub fn update(&mut self, op: &WriteDescriptorResources<'_>) {
        let mut buffer_infos = Vec::new();
        let mut sampler_infos = Vec::new();
        for binding in op.bindings {
            match &binding.resource {
                WriteDescriptorResource::Buffer(buffer) => {
                    let buffer_info = vk::DescriptorBufferInfo::default()
                        .buffer(buffer.buffer().buffer)
                        .offset(buffer.offset())
                        .range(buffer.len());

                    buffer_infos.push(buffer_info);
                }
                WriteDescriptorResource::Sampler(sampler) => {
                    let info = vk::DescriptorImageInfo::default()
                        .sampler(sampler.sampler)
                        .image_view(vk::ImageView::null());

                    sampler_infos.push(info);
                }
            }
        }

        let mut writes = Vec::new();

        let mut next_buffer = 0;
        let mut next_sampler = 0;
        for binding in op.bindings {
            let mut write = vk::WriteDescriptorSet::default()
                .dst_set(self.set)
                .dst_binding(binding.binding)
                .dst_array_element(0)
                .descriptor_count(1);

            match &binding.resource {
                WriteDescriptorResource::Buffer(_) => {
                    write = write
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                        .buffer_info(core::slice::from_ref(&buffer_infos[next_buffer]));
                    next_buffer += 1;
                }
                WriteDescriptorResource::Sampler(_) => {
                    write = write
                        .descriptor_type(vk::DescriptorType::SAMPLER)
                        .image_info(core::slice::from_ref(&sampler_infos[next_sampler]));
                    next_sampler += 1;
                }
            }

            writes.push(write)
        }

        unsafe {
            self.pool.device.update_descriptor_sets(&writes, &[]);
        }
    }
}

#[derive(Debug)]
pub struct Fence {
    device: Arc<DeviceShared>,
    fence: vk::Fence,
}

impl Fence {
    pub fn wait(&mut self, timeout: Option<Duration>) {
        let timeout = match timeout {
            Some(timeout) => timeout.as_nanos().try_into().unwrap(),
            None => u64::MAX,
        };

        let res = unsafe { self.device.wait_for_fences(&[self.fence], true, timeout) };
        match res {
            Ok(()) => (),
            Err(vk::Result::TIMEOUT) => (),
            Err(err) => todo!(),
        }
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
}

#[derive(Debug)]
pub struct Sampler {
    device: Arc<DeviceShared>,
    sampler: vk::Sampler,
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
        }
    }
}

const fn cstr_to_fixed_array<const N: usize>(s: &CStr) -> [i8; N] {
    assert!(s.count_bytes() < N);

    let mut arr = [0; N];

    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), arr.as_mut_ptr(), s.count_bytes());
    }

    arr
}

extern "system" fn debug_callback(
    severity: DebugUtilsMessageSeverityFlagsEXT,
    typ: DebugUtilsMessageTypeFlagsEXT,
    data: *const DebugUtilsMessengerCallbackDataEXT<'_>,
    _: *mut c_void,
) -> Bool32 {
    let data = unsafe { *data };
    let message = match unsafe { data.message_as_c_str() } {
        Some(msg) => msg.to_string_lossy(),
        None => Cow::Borrowed("(no message)"),
    };

    match severity {
        DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            println!("{:?} {}", typ, message);
        }
        DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            println!("{:?} {}", typ, message);
        }
        DebugUtilsMessageSeverityFlagsEXT::INFO => {
            println!("{:?} {}", typ, message);
        }
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE | _ => {
            println!("{:?} {}", typ, message);
        }
    }

    // The application should always return `VK_FALSE`.
    FALSE
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnknownEnumValue;

#[derive(Clone)]
struct InstanceShared {
    entry: ash::Entry,
    instance: ash::Instance,
    messenger: DebugUtilsMessengerEXT,
}

impl Debug for InstanceShared {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(InstanceShared))
            .finish_non_exhaustive()
    }
}

impl Deref for InstanceShared {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl Drop for InstanceShared {
    fn drop(&mut self) {
        unsafe {
            let instance = debug_utils::Instance::new(&self.entry, &self.instance);
            instance.destroy_debug_utils_messenger(self.messenger, None);
        }

        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

#[derive(Clone)]
struct DeviceShared {
    instance: Arc<InstanceShared>,
    device: ash::Device,
}

impl Debug for DeviceShared {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(DeviceShared))
            .finish_non_exhaustive()
    }
}

impl Deref for DeviceShared {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Drop for DeviceShared {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct DeviceLimits {
    max_push_constants_size: u32,
}
