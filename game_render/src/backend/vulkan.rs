use core::arch;
use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::{c_void, CStr};
use std::num::NonZeroU32;
use std::ops::Range;
use std::ptr::null_mut;

use ash::ext::debug_utils;
use ash::vk::{
    self, AccessFlags, AcquireNextImageInfoKHR, ApplicationInfo, AttachmentDescription,
    AttachmentLoadOp, AttachmentReference, AttachmentStoreOp, BlendFactor, BlendOp, Bool32,
    ClearColorValue, ClearValue, ColorComponentFlags, ColorSpaceKHR, CommandBufferAllocateInfo,
    CommandBufferBeginInfo, CommandBufferInheritanceInfo, CommandBufferLevel,
    CommandBufferUsageFlags, CommandPoolCreateFlags, CommandPoolCreateInfo, ComponentMapping,
    ComponentSwizzle, CompositeAlphaFlagsKHR, CullModeFlags, DebugUtilsMessageSeverityFlagsEXT,
    DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT,
    DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT, DependencyFlags, DeviceCreateInfo,
    DeviceQueueCreateInfo, DeviceQueueInfo2, Extent2D, Format, FrontFace,
    GraphicsPipelineCreateInfo, ImageAspectFlags, ImageLayout, ImageMemoryBarrier,
    ImageSubresourceRange, ImageUsageFlags, ImageViewCreateInfo, ImageViewType, InstanceCreateInfo,
    LogicOp, Offset2D, PhysicalDevice, PhysicalDeviceDynamicRenderingFeatures,
    PhysicalDeviceFeatures, PhysicalDeviceType, PipelineBindPoint, PipelineCache,
    PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineLayoutCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineRenderingCreateInfo, PipelineShaderStageCreateInfo, PipelineStageFlags,
    PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
    PresentInfoKHR, PresentModeKHR, PrimitiveTopology, QueueFlags, Rect2D, RenderingAttachmentInfo,
    RenderingFlags, RenderingInfo, ResolveModeFlags, SampleCountFlags, SemaphoreCreateInfo,
    ShaderModuleCreateInfo, ShaderStageFlags, SharingMode, SubmitInfo, SubpassDependency,
    SubpassDescription, SurfaceKHR, SurfaceTransformFlagsKHR, SwapchainCreateInfoKHR, SwapchainKHR,
    Viewport, FALSE,
};
use ash::Entry;
use glam::UVec2;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::{
    AdapterKind, AdapterProperties, LoadOp, PipelineDescriptor, PipelineStage, PresentMode,
    QueueCapabilities, QueueFamily, RenderPassColorAttachment, RenderPassDescriptor, StoreOp,
    SwapchainCapabilities, SwapchainConfig, TextureFormat,
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
        for stage in descriptor.stages {
            let vk_stage = match stage {
                PipelineStage::Vertex(stage) => PipelineShaderStageCreateInfo::default()
                    .stage(ShaderStageFlags::VERTEX)
                    .module(stage.shader.shader)
                    .name(c"main"),
                PipelineStage::Fragment(stage) => PipelineShaderStageCreateInfo::default()
                    .stage(ShaderStageFlags::FRAGMENT)
                    .module(stage.shader.shader)
                    .name(c"main"),
            };

            stages.push(vk_stage);
        }

        let vertex_input_state = PipelineVertexInputStateCreateInfo::default();

        let input_assembly_state = PipelineInputAssemblyStateCreateInfo::default()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(4096.0)
            .height(4096.0)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = Rect2D::default()
            .offset(Offset2D::default())
            .extent(Extent2D {
                width: 4096,
                height: 4096,
            });

        let viewports = [viewport];
        let scissors = [scissor];

        let viewport_state = PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterization_state = PipelineRasterizationStateCreateInfo::default()
            .depth_bias_enable(true)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::CLOCKWISE);

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

        let pipeline_layout_info = PipelineLayoutCreateInfo::default();
        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        };

        let mut rendering_info = PipelineRenderingCreateInfo::default();

        let info = GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(pipeline_layout)
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
        let buffers: Vec<_> = buffers.iter().map(|buf| *buf.buffer).collect();

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
        dbg!(&present_modes);
        dbg!(&formats);
        dbg!(&caps);

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
        }
    }

    pub fn create_swapchain<'b>(
        &'a self,
        device: &'b Device<'b>,
        config: SwapchainConfig,
    ) -> Swapchain<'a, 'b> {
        let queue_family_indices = [device.queue_family_index];

        // See https://registry.khronos.org/vulkan/specs/latest/man/html/VkSwapchainCreateInfoKHR.html
        // `imageExtent` members `width` and `height` must both be non-zero.
        assert_ne!(config.extent.x, 0);
        assert_ne!(config.extent.y, 0);

        let info = SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(config.image_count)
            .image_format(config.format.into())
            .image_color_space(ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(Extent2D {
                width: config.extent.x,
                height: config.extent.y,
            })
            .image_array_layers(1)
            // TODO: Unchecked
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices)
            // TODO: Unchecked
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            // TODO: Unchecked
            .pre_transform(SurfaceTransformFlagsKHR::IDENTITY)
            .present_mode(config.present_mode.into())
            .clipped(true)
            .old_swapchain(SwapchainKHR::null());

        let khr_device = ash::khr::swapchain::Device::new(&self.instance.instance, &device.device);
        let swapchain = unsafe { khr_device.create_swapchain(&info, None).unwrap() };

        let images = unsafe { khr_device.get_swapchain_images(swapchain).unwrap() };

        let image_views = images
            .iter()
            .map(|image| {
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
                    .image(*image)
                    .view_type(ImageViewType::TYPE_2D)
                    .format(config.format.into())
                    .components(components)
                    .subresource_range(subresource_range);

                unsafe { device.device.create_image_view(&info, None).unwrap() }
            })
            .collect();

        Swapchain {
            surface: self,
            swapchain,
            device,
            images,
            image_views,
            extent: config.extent,
            format: config.format,
        }
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
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    format: TextureFormat,
    pub extent: UVec2,
}

impl<'a, 'b> Swapchain<'a, 'b> {
    pub fn acquire_next_image(&mut self, semaphore: &Semaphore<'_>) -> u32 {
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

        image_index
    }

    pub fn present(&mut self, queue: &Queue<'_>, image_index: u32, wait_semaphore: &Semaphore<'_>) {
        let device = ash::khr::swapchain::Device::new(
            &self.device.adapter.instance.instance,
            &self.device.device,
        );

        let wait_semaphores = &[wait_semaphore.semaphore];

        let swapchains = &[self.swapchain];
        let image_indices = &[image_index];
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
            _ => Err(UnknownEnumValue),
        }
    }
}

impl From<TextureFormat> for Format {
    fn from(value: TextureFormat) -> Self {
        match value {
            TextureFormat::R8G8B8A8Unorm => Self::R8G8B8A8_UNORM,
            TextureFormat::R8G8B8A8UnormSrgb => Self::R8G8B8A8_SRGB,
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
}

impl<'a> CommandPool<'a> {
    pub fn create_encoder(&mut self) -> CommandEncoder<'_> {
        let inheritance = CommandBufferInheritanceInfo::default();

        let info = CommandBufferBeginInfo::default()
            .flags(CommandBufferUsageFlags::empty())
            .inheritance_info(&inheritance);

        let buffer = self.buffers[0];

        unsafe {
            self.device
                .device
                .begin_command_buffer(buffer, &info)
                .unwrap();
        }

        CommandEncoder {
            device: self.device,
            buffer: &self.buffers[0],
        }
    }
}

impl<'a> Drop for CommandPool<'a> {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_command_pool(self.pool, None);
        }
    }
}

pub struct CommandEncoder<'a> {
    device: &'a Device<'a>,
    buffer: &'a vk::CommandBuffer,
}

impl<'a> CommandEncoder<'a> {
    pub fn begin_render_pass(&mut self, descriptor: &RenderPassDescriptor<'_>) -> RenderPass<'_> {
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
                .image_view(attachment.view)
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
            self.device.device.cmd_begin_rendering(*self.buffer, &info);
        }

        RenderPass { encoder: self }
    }

    pub fn emit_pipeline_barrier(
        &mut self,
        image: vk::Image,
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
            .image(image)
            .subresource_range(subresource_range);

        unsafe {
            self.device.device.cmd_pipeline_barrier(
                *self.buffer,
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
            self.device.device.end_command_buffer(*self.buffer).unwrap();
        }

        CommandBuffer {
            device: self.device,
            buffer: self.buffer,
        }
    }
}

impl<'a> Drop for CommandEncoder<'a> {
    fn drop(&mut self) {}
}

pub struct RenderPass<'a> {
    encoder: &'a CommandEncoder<'a>,
}

impl<'a> RenderPass<'a> {
    pub fn bind_pipeline(&mut self, pipeline: &Pipeline<'_>) {
        unsafe {
            self.encoder.device.device.cmd_bind_pipeline(
                *self.encoder.buffer,
                PipelineBindPoint::GRAPHICS,
                pipeline.pipeline,
            );
        }
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        unsafe {
            self.encoder.device.device.cmd_draw(
                *self.encoder.buffer,
                vertices.len() as u32,
                instances.len() as u32,
                vertices.start,
                instances.start,
            );
        }
    }
}

impl<'a> Drop for RenderPass<'a> {
    fn drop(&mut self) {
        unsafe {
            self.encoder
                .device
                .device
                .cmd_end_rendering(*self.encoder.buffer);
        }
    }
}

pub struct CommandBuffer<'a> {
    device: &'a Device<'a>,
    buffer: &'a vk::CommandBuffer,
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
