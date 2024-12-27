use core::arch;
use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::{c_void, CStr};
use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::Range;
use std::ptr::{null_mut, NonNull};

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
    DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT, DependencyFlags, DeviceCreateInfo,
    DeviceQueueCreateInfo, DeviceQueueInfo2, DynamicState, Extent2D, Format, FrontFace,
    GraphicsPipelineCreateInfo, ImageAspectFlags, ImageLayout, ImageMemoryBarrier,
    ImageSubresourceRange, ImageUsageFlags, ImageViewCreateInfo, ImageViewType, InstanceCreateInfo,
    LogicOp, MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, Offset2D, PhysicalDevice,
    PhysicalDeviceDynamicRenderingFeatures, PhysicalDeviceFeatures, PhysicalDeviceType,
    PipelineBindPoint, PipelineCache, PipelineColorBlendAttachmentState,
    PipelineColorBlendStateCreateInfo, PipelineDynamicStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineLayoutCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineRenderingCreateInfo, PipelineShaderStageCreateInfo, PipelineStageFlags,
    PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
    PresentInfoKHR, PresentModeKHR, PrimitiveTopology, QueueFlags, Rect2D, RenderingAttachmentInfo,
    RenderingFlags, RenderingInfo, ResolveModeFlags, SampleCountFlags, SemaphoreCreateInfo,
    ShaderModuleCreateInfo, ShaderStageFlags, SharingMode, SubmitInfo, SubpassDependency,
    SubpassDescription, SurfaceKHR, SurfaceTransformFlagsKHR, SwapchainCreateInfoKHR, SwapchainKHR,
    Viewport, FALSE, WHOLE_SIZE,
};
use ash::Entry;
use glam::UVec2;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use wgpu::hal::auxil::db;

use super::{
    AdapterKind, AdapterMemoryProperties, AdapterProperties, BufferUsage, Face, LoadOp, MemoryHeap,
    MemoryRequirements, MemoryType, MemoryTypeFlags, PipelineDescriptor, PipelineStage,
    PresentMode, QueueCapabilities, QueueFamily, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, SwapchainCapabilities, SwapchainConfig, TextureFormat,
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

const EXTENSIONS: &[&CStr] = &[
    // Required to create any surface.
    ash::vk::KHR_SURFACE_NAME,
    // Wayland
    #[cfg(target_os = "linux")]
    ash::vk::KHR_WAYLAND_SURFACE_NAME,
    // X11
    #[cfg(target_os = "linux")]
    ash::vk::KHR_XCB_SURFACE_NAME,
    #[cfg(target_os = "linux")]
    ash::vk::KHR_XLIB_SURFACE_NAME,
    // Windows
    #[cfg(target_os = "windows")]
    ash::vk::KHR_WIN32_SURFACE_NAME,
    ash::vk::EXT_DEBUG_UTILS_NAME,
];

const DEVICE_EXTENSIONS: &[&CStr] = &[
    // VK_KHR_swapchain
    ash::khr::swapchain::NAME,
    // VK_KHR_dynamic_rendering
    // Core in Vulkan 1.3
    ash::khr::dynamic_rendering::NAME,
];

const fn make_api_version(major: u32, minor: u32, patch: u32) -> u32 {
    (major << 22) | (minor << 12) | patch
}

#[derive(Debug)]
pub enum Error {
    MissingLayer(&'static CStr),
}

pub struct Instance {
    entry: Entry,
    instance: ash::Instance,
    messenger: DebugUtilsMessengerEXT,
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

        let mut layers = Vec::new();
        layers.push(VULKAN_VALIDATION_LAYERS.as_ptr());

        let mut extensions = Vec::new();
        extensions.extend(EXTENSIONS.iter().map(|v| v.as_ptr()));

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
            entry,
            instance,
            messenger,
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

    pub fn create_surface(
        &self,
        display: RawDisplayHandle,
        window: RawWindowHandle,
    ) -> Surface<'_> {
        let surface = match (display, window) {
            #[cfg(target_os = "linux")]
            (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
                let info = ash::vk::WaylandSurfaceCreateInfoKHR::default()
                    .display(display.display.as_ptr())
                    .surface(window.surface.as_ptr());

                let instance =
                    ash::khr::wayland_surface::Instance::new(&self.entry, &self.instance);
                unsafe { instance.create_wayland_surface(&info, None).unwrap() }
            }
            #[cfg(target_os = "linux")]
            (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
                let info = ash::vk::XcbSurfaceCreateInfoKHR::default()
                    .connection(display.connection.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    .window(window.window.get());

                let instance = ash::khr::xcb_surface::Instance::new(&self.entry, &self.instance);
                unsafe { instance.create_xcb_surface(&info, None).unwrap() }
            }
            #[cfg(target_os = "linux")]
            (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
                let info = ash::vk::XlibSurfaceCreateInfoKHR::default()
                    .dpy(display.display.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    .window(window.window);

                let instance = ash::khr::xlib_surface::Instance::new(&self.entry, &self.instance);
                unsafe { instance.create_xlib_surface(&info, None).unwrap() }
            }
            #[cfg(target_os = "windows")]
            (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
                let info = ash::vk::Win32SurfaceCreateInfoKHR::default()
                    .hinstance(window.hinstance.map(|v| v.get()).unwrap_or_default())
                    .hwnd(window.hwnd.get());

                let instance = ash::khr::win32_surface::Instance::new(&self.entry, &self.instance);
                unsafe { instance.create_win32_surface(&info, None).unwrap() }
            }
            _ => todo!(),
        };

        Surface {
            instance: self,
            surface,
        }
    }

    fn destroy(&mut self) {
        unsafe {
            self.debug_utils()
                .destroy_debug_utils_messenger(self.messenger, None);
        }

        unsafe {
            self.instance.destroy_instance(None);
        }
    }

    fn debug_utils(&self) -> debug_utils::Instance {
        debug_utils::Instance::new(&self.entry, &self.instance)
    }

    fn khr_surface(&self) -> ash::khr::surface::Instance {
        ash::khr::surface::Instance::new(&self.entry, &self.instance)
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.destroy();
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

    pub fn create_device(&self, queue_id: u32) -> Device<'_> {
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

        let create_info = DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            // Device layers are deprecated, but the Vulkan spec still recommends
            // applications to pass layers.
            // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#extendingvulkan-layers-devicelayerdeprecation
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features)
            .push_next(&mut dynamic_rendering);

        let device = unsafe {
            self.instance
                .instance
                .create_device(self.physical_device, &create_info, None)
                .unwrap()
        };

        Device {
            adapter: self,
            device,
            queue_family_index: queue_id,
        }
    }
}

pub struct Device<'a> {
    adapter: &'a Adapter<'a>,
    device: ash::Device,
    queue_family_index: u32,
}

impl<'a> Device<'a> {
    pub fn queue(&self) -> Queue<'_> {
        let info = DeviceQueueInfo2::default()
            .queue_family_index(self.queue_family_index)
            // Index is always 0 since we only create
            // a single queue for now.
            .queue_index(0);

        let queue = unsafe { self.device.get_device_queue2(&info) };

        Queue {
            device: self,
            queue,
        }
    }

    pub fn create_buffer(&self, size: NonZeroU64, usage: BufferUsage) -> Buffer<'_> {
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
            device: &self.device,
            memory: None,
        }
    }

    pub fn allocate_memory(&self, size: NonZeroU64, memory_type_index: u32) -> DeviceMemory<'_> {
        // TODO: If the protectedMemory feature is not enabled, the VkMemoryAllocateInfo::memoryTypeIndex must not indicate a memory type that reports VK_MEMORY_PROPERTY_PROTECTED_BIT.
        let info = MemoryAllocateInfo::default()
            // - `allocationSize` must be greater than 0.
            .allocation_size(size.get())
            .memory_type_index(memory_type_index);

        let memory = unsafe { self.device.allocate_memory(&info, None).unwrap() };
        DeviceMemory {
            memory,
            device: &self.device,
            size,
        }
    }

    pub fn buffer_memory_requirements(&self, buffer: &Buffer<'_>) -> MemoryRequirements {
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

    pub fn bind_buffer_memory<'mem>(&self, buffer: &mut Buffer<'mem>, memory: DeviceMemory<'mem>) {
        let info = BindBufferMemoryInfo::default()
            .buffer(buffer.buffer)
            .memory(memory.memory);

        unsafe {
            self.device.bind_buffer_memory2(&[info]).unwrap();
        }

        buffer.memory = Some(memory);
    }

    pub unsafe fn map_memory(&self, memory: &DeviceMemory<'_>) -> &mut [u8] {
        let data = unsafe {
            self.device
                .map_memory(memory.memory, 0, memory.size.get(), MemoryMapFlags::empty())
                .unwrap()
        };

        let len = memory.size.get() as usize;
        unsafe { core::slice::from_raw_parts_mut(data.cast::<u8>(), len) }
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

    pub fn create_pipeline(&self, descriptor: &PipelineDescriptor<'_>) -> Pipeline<'_> {
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

        let pipeline_layout_info = PipelineLayoutCreateInfo::default();
        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        };

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

    pub fn create_semaphore(&self) -> Semaphore<'_> {
        let info = SemaphoreCreateInfo::default();

        let semaphore = unsafe { self.device.create_semaphore(&info, None).unwrap() };

        Semaphore {
            device: self,
            semaphore,
        }
    }
}

impl<'a> Drop for Device<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}

pub struct Queue<'a> {
    device: &'a Device<'a>,
    queue: vk::Queue,
}

impl<'a> Queue<'a> {
    pub fn submit(
        &self,
        buffers: &[CommandBuffer<'_>],
        wait_semaphore: &Semaphore<'_>,
        wait_stages: PipelineStageFlags,
        signal_semaphore: &Semaphore<'_>,
    ) {
        let buffers: Vec<_> = buffers.iter().map(|buf| buf.buffer).collect();

        let wait_semaphores = &[wait_semaphore.semaphore];
        let wait_stages = &[wait_stages];
        let signal_semaphores = &[signal_semaphore.semaphore];

        let info = SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_stages)
            .command_buffers(&buffers)
            .signal_semaphores(signal_semaphores);

        unsafe {
            self.device
                .device
                .queue_submit(self.queue, &[info], vk::Fence::null())
                .unwrap();
        }
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.device.device.queue_wait_idle(self.queue).unwrap();
        }
    }
}

pub struct Surface<'a> {
    instance: &'a Instance,
    surface: SurfaceKHR,
}

impl<'a> Surface<'a> {
    pub fn get_capabilities(&self, device: &Device<'_>) -> SwapchainCapabilities {
        let instance =
            ash::khr::surface::Instance::new(&self.instance.entry, &self.instance.instance);

        let is_supported = unsafe {
            instance
                .get_physical_device_surface_support(
                    device.adapter.physical_device,
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
                .get_physical_device_surface_capabilities(
                    device.adapter.physical_device,
                    self.surface,
                )
                .unwrap()
        };
        let formats = unsafe {
            instance
                .get_physical_device_surface_formats(device.adapter.physical_device, self.surface)
                .unwrap()
        };
        let present_modes = unsafe {
            instance
                .get_physical_device_surface_present_modes(
                    device.adapter.physical_device,
                    self.surface,
                )
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

    pub fn create_swapchain<'b>(
        &'a self,
        device: &'b Device<'b>,
        config: SwapchainConfig,
        caps: &SwapchainCapabilities,
    ) -> Swapchain<'a, 'b> {
        // SAFETY: `old_swapchain` is null.
        let (swapchain, images) =
            unsafe { self.create_swapchain_inner(device, &config, &caps, SwapchainKHR::null()) };

        Swapchain {
            surface: self,
            device: device,
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
    unsafe fn create_swapchain_inner<'b>(
        &'a self,
        device: &'b Device<'b>,
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

impl<'a> Drop for Surface<'a> {
    fn drop(&mut self) {
        let instance =
            ash::khr::surface::Instance::new(&self.instance.entry, &self.instance.instance);

        unsafe {
            instance.destroy_surface(self.surface, None);
        }
    }
}

pub struct Swapchain<'a, 'b> {
    surface: &'a Surface<'a>,
    device: &'b Device<'b>,
    swapchain: SwapchainKHR,
    images: Vec<vk::Image>,

    format: TextureFormat,
    extent: UVec2,
}

impl<'a, 'b> Swapchain<'a, 'b> {
    pub fn recreate(&mut self, config: SwapchainConfig, caps: &SwapchainCapabilities) {
        // SAFETY: `self.swapchain` is a valid swapchain created by `self.surface`.
        // Since this function accepts a mutable reference this swapchain is not used.
        let (swapchain, images) = unsafe {
            self.surface
                .create_swapchain_inner(self.device, &config, caps, self.swapchain)
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

    pub fn acquire_next_image(&self, semaphore: &Semaphore<'_>) -> SwapchainTexture<'_> {
        let device = ash::khr::swapchain::Device::new(
            &self.device.adapter.instance.instance,
            &self.device.device,
        );

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
                device: self.device,
                image: self.images[image_index as usize],
                format: self.format,
                size: self.extent,
            },
            suboptimal,
            index: image_index,
            device: self.device,
            swapchain: self,
        }
    }

    pub fn present(&self, queue: &Queue<'_>, img: u32, wait_semaphore: &Semaphore<'_>) {
        let device = ash::khr::swapchain::Device::new(
            &self.device.adapter.instance.instance,
            &self.device.device,
        );

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

impl<'a, 'b> Drop for Swapchain<'a, 'b> {
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

pub struct ShaderModule<'a> {
    device: &'a Device<'a>,
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
    device: &'a Device<'a>,
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
    device: &'a Device<'a>,
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
    device: &'a Device<'a>,
    pool: &'a CommandPool<'a>,
    buffer: vk::CommandBuffer,
}

impl<'a> CommandEncoder<'a> {
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
        }
    }

    pub fn emit_pipeline_barrier(
        &mut self,
        image: &Texture<'_>,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src: PipelineStageFlags,
        dst: PipelineStageFlags,
        src_mask: AccessFlags,
        dst_mask: AccessFlags,
    ) {
        let subresource_range = ImageSubresourceRange::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let barrier = ImageMemoryBarrier::default()
            .src_access_mask(src_mask)
            .dst_access_mask(dst_mask)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .image(image.image)
            .subresource_range(subresource_range);

        unsafe {
            self.device.device.cmd_pipeline_barrier(
                self.buffer,
                src,
                dst,
                DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
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
}

impl<'encoder, 'resources> RenderPass<'encoder, 'resources> {
    pub fn bind_pipeline(&mut self, pipeline: &Pipeline<'_>) {
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
    device: &'a Device<'a>,
    buffer: vk::CommandBuffer,
}

pub struct Semaphore<'a> {
    device: &'a Device<'a>,
    semaphore: vk::Semaphore,
}

impl<'a> Semaphore<'a> {}

impl<'a> Drop for Semaphore<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

pub struct SwapchainTexture<'a> {
    pub texture: Texture<'a>,
    pub suboptimal: bool,
    index: u32,
    device: &'a Device<'a>,
    swapchain: &'a Swapchain<'a, 'a>,
}

impl<'a> SwapchainTexture<'a> {
    pub fn present(&self, queue: &Queue<'_>, wait_semaphore: &Semaphore<'_>) {
        let device = ash::khr::swapchain::Device::new(
            &self.device.adapter.instance.instance,
            &self.device.device,
        );

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

pub struct Texture<'a> {
    device: &'a Device<'a>,
    image: vk::Image,
    format: TextureFormat,
    size: UVec2,
}

impl<'a> Texture<'a> {
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
            device: self.device,
            view,
        }
    }
}

pub struct TextureView<'a> {
    device: &'a Device<'a>,
    view: vk::ImageView,
}

impl<'a> Drop for TextureView<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_image_view(self.view, None);
        }
    }
}

pub struct Buffer<'a> {
    buffer: vk::Buffer,
    device: &'a ash::Device,
    memory: Option<DeviceMemory<'a>>,
}

impl<'a> Buffer<'a> {}

impl<'a> Drop for Buffer<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}

pub struct DeviceMemory<'a> {
    memory: vk::DeviceMemory,
    device: &'a ash::Device,
    size: NonZeroU64,
}

impl<'a> Drop for DeviceMemory<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.free_memory(self.memory, None);
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
