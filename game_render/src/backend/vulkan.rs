use std::backtrace::Backtrace;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::{Bound, Deref, Range, RangeBounds};
use std::ptr::{null_mut, NonNull};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ash::ext::debug_utils;
use ash::vk::Handle;
use ash::{vk, Entry};
use game_common::collections::scratch_buffer::ScratchBuffer;
use glam::UVec2;
use parking_lot::Mutex;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use thiserror::Error;

use crate::backend::{mip_level_size_2d, DescriptorType, SurfaceFormat, TextureLayout};

use super::shader::{self, BindingInfo, Shader};
use super::{
    AccessFlags, AdapterKind, AdapterMemoryProperties, AdapterProperties, AddressMode, BufferUsage,
    BufferView, ColorSpace, CompareOp, CopyBuffer, DescriptorPoolDescriptor,
    DescriptorSetDescriptor, Face, FilterMode, FrontFace, IndexFormat, LoadOp, MemoryHeap,
    MemoryHeapFlags, MemoryRequirements, MemoryType, MemoryTypeFlags, PipelineBarriers,
    PipelineDescriptor, PipelineStage, PresentMode, PrimitiveTopology, QueueCapabilities,
    QueueFamily, QueueFamilyId, QueueSubmit, RenderPassDescriptor, SamplerDescriptor, ShaderStage,
    ShaderStages, StoreOp, SwapchainCapabilities, SwapchainConfig, TextureDescriptor,
    TextureFormat, TextureUsage, TextureViewDescriptor, WriteDescriptorResource,
    WriteDescriptorResources,
};

/// The highest version of Vulkan that we support.
///
/// See <https://registry.khronos.org/vulkan/specs/latest/man/html/VkApplicationInfo.html>
const API_VERSION: u32 = make_api_version(1, 3, 0);

const APPLICATION_NAME: Option<&CStr> = None;
const APPLICATION_VERSION: u32 = 0;
const ENGINE_NAME: Option<&CStr> = None;
const ENGINE_VERSION: u32 = 0;

#[derive(Copy, Clone, Debug, Default)]
struct InstanceLayers {
    /// `VK_LAYER_KHRONOS_validation`
    validation: bool,
}

impl InstanceLayers {
    const VALIDATION: &CStr = c"VK_LAYER_KHRONOS_validation";

    fn names(&self) -> Vec<&'static CStr> {
        [(self.validation, Self::VALIDATION)]
            .iter()
            .filter_map(|(enabled, name)| enabled.then_some(*name))
            .collect()
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error("device lost")]
    DeviceLost,
    #[error("out of host memory")]
    OutOfHostMemory,
    #[error("out of device memory")]
    OutOfDeviceMemory,
    #[error("out of pool memory")]
    OutOfPoolMemory,
    #[error("no allocations left")]
    NoAllocationsLeft,
    #[error("no queue left")]
    NoQueueLeft,
    #[error("missing layer: {0:?}")]
    MissingLayer(&'static CStr),
    #[error("missing extension: {0:?}")]
    MissingExtension(&'static CStr),
    #[error("unsupported surface")]
    UnsupportedSurface,
    #[error("invalidated swapchain")]
    InvalidatedSwapchain,
    #[error("command pool exhausted")]
    CommandPoolExhausted,
    #[error(transparent)]
    Other(vk::Result),
}

impl From<vk::Result> for Error {
    fn from(res: vk::Result) -> Self {
        match res {
            vk::Result::ERROR_DEVICE_LOST => Self::DeviceLost,
            vk::Result::ERROR_OUT_OF_HOST_MEMORY => Self::OutOfHostMemory,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => Self::OutOfDeviceMemory,
            vk::Result::ERROR_OUT_OF_POOL_MEMORY => Self::OutOfPoolMemory,
            _ => Self::Other(res),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Enabled valiation layers if available.
    pub validation: bool,
}

/// Entrypoint for the Vulkan API.
#[derive(Clone, Debug)]
pub struct Instance {
    instance: Arc<InstanceShared>,
    extensions: InstanceExtensions,
}

impl Instance {
    /// Creates a new `Instance`.
    pub fn new(config: Config) -> Result<Self, Error> {
        let entry = unsafe { Entry::load().unwrap() };

        let mut app = vk::ApplicationInfo::default()
            .application_version(APPLICATION_VERSION)
            .engine_version(ENGINE_VERSION)
            .api_version(API_VERSION);

        if let Some(name) = APPLICATION_NAME {
            app = app.application_name(name);
        }

        if let Some(name) = ENGINE_NAME {
            app = app.engine_name(name);
        }

        let supported_layers = Self::get_supported_layers(&entry)?;
        if config.validation && !supported_layers.validation {
            return Err(Error::MissingLayer(InstanceLayers::VALIDATION));
        }

        let supported_extensions = Self::get_supported_extensions(&entry);
        if config.validation && !supported_extensions.debug_utils {
            return Err(Error::MissingExtension(vk::EXT_DEBUG_UTILS_NAME));
        }

        let mut enabled_layers = InstanceLayers::default();
        enabled_layers.validation = config.validation;
        let enabled_layers = enabled_layers
            .names()
            .iter()
            .map(|v| v.as_ptr())
            .collect::<Vec<_>>();

        // For now we just enable all extensions that we have queried support
        // for and that are available.
        let enabled_extensions = supported_extensions
            .names()
            .iter()
            .map(|v| v.as_ptr())
            .collect::<Vec<_>>();

        let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback));

        const TRUE: &[u8] = &vk::TRUE.to_ne_bytes();

        // Refer to VkLayer_khronos_validation.json for list of
        // options.
        let mut settings = Vec::new();
        for (key, value) in [
            (c"validate_core", TRUE),
            (c"check_image_layout", TRUE),
            (c"check_command_buffer", TRUE),
            (c"check_object_in_use", TRUE),
            (c"check_query", TRUE),
            (c"check_shaders", TRUE),
            (c"check_shaders_caching", TRUE),
            (c"unique_handles", TRUE),
            (c"object_lifetime", TRUE),
            (c"stateless_param", TRUE),
            (c"thread_safety", TRUE),
            (c"validate_sync", TRUE),
            (c"validate_best_practices", TRUE),
        ] {
            settings.push(
                vk::LayerSettingEXT::default()
                    .layer_name(InstanceLayers::VALIDATION)
                    .setting_name(&key)
                    .ty(vk::LayerSettingTypeEXT::BOOL32)
                    .values(value),
            );
        }

        let mut layer_settings = vk::LayerSettingsCreateInfoEXT::default().settings(&settings);

        let mut info = vk::InstanceCreateInfo::default()
            .application_info(&app)
            .enabled_layer_names(&enabled_layers)
            .enabled_extension_names(&enabled_extensions);

        if config.validation {
            info = info.push_next(&mut debug_info);
            info = info.push_next(&mut layer_settings);
        }

        let instance = unsafe { entry.create_instance(&info, None)? };

        let messenger = if config.validation {
            let instance_d = debug_utils::Instance::new(&entry, &instance);
            match unsafe { instance_d.create_debug_utils_messenger(&debug_info, None) } {
                Ok(messenger) => Some(messenger),
                Err(err) => {
                    // We must manually destroy the instance if an error occurs,
                    // otherwise the vkInstance would leak.
                    unsafe {
                        instance.destroy_instance(None);
                    }

                    return Err(err.into());
                }
            }
        } else {
            None
        };

        Ok(Self {
            instance: Arc::new(InstanceShared {
                config,
                entry,
                instance,
                messenger,
            }),
            extensions: supported_extensions,
        })
    }

    /// Returns a list of all physical [`Adapter`]s available.
    pub fn adapters(&self) -> Result<Vec<Adapter>, Error> {
        let physical_devices = unsafe { self.instance.enumerate_physical_devices()? };
        Ok(physical_devices
            .into_iter()
            .map(|physical_device| Adapter {
                instance: self.instance.clone(),
                physical_device,
            })
            .collect())
    }

    /// Creates a new [`Surface`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if `display` or `window` reference an unsupported handle type or
    /// creation of the [`Surface`] fails.
    ///
    /// # Safety
    ///
    /// - The passed `display` and `window` handles must be valid until the [`Surface`] is dropped.
    pub unsafe fn create_surface(
        &self,
        display: RawDisplayHandle,
        window: RawWindowHandle,
    ) -> Result<Surface, Error> {
        if !self.extensions.surface {
            return Err(Error::UnsupportedSurface);
        }

        let surface = match (display, window) {
            #[cfg(all(unix, feature = "wayland"))]
            (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
                if !self.extensions.surface_wayland {
                    return Err(Error::UnsupportedSurface);
                }

                let info = vk::WaylandSurfaceCreateInfoKHR::default()
                    // - `display` must be a valid Wayland `wl_display`.
                    .display(display.display.as_ptr())
                    // - `surface` must be a valid Wayland `wl_surface`.
                    .surface(window.surface.as_ptr())
                    // - `flags` must be `0`.
                    .flags(vk::WaylandSurfaceCreateFlagsKHR::empty());

                let instance =
                    ash::khr::wayland_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_wayland_surface(&info, None)? }
            }
            #[cfg(all(unix, feature = "x11"))]
            (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
                if !self.extensions.surface_xcb {
                    return Err(Error::UnsupportedSurface);
                }

                let info = vk::XcbSurfaceCreateInfoKHR::default()
                    // - `connection` must point to a valid X11 `xcb_connection_t`.
                    .connection(display.connection.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    // - `window` must be a valid X11 `xcb_window_t`.
                    .window(window.window.get())
                    // - `flags` must be `0`.
                    .flags(vk::XcbSurfaceCreateFlagsKHR::empty());

                let instance =
                    ash::khr::xcb_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_xcb_surface(&info, None)? }
            }
            #[cfg(all(unix, feature = "x11"))]
            (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
                if !self.extensions.surface_xlib {
                    return Err(Error::UnsupportedSurface);
                }

                let info = vk::XlibSurfaceCreateInfoKHR::default()
                    // - `dpy` must point to a valid Xlib `Display`.
                    .dpy(display.display.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    // - `window` must point to a valid Xlib `Window`.
                    .window(window.window)
                    // - `flags` must be `0`.
                    .flags(vk::XlibSurfaceCreateFlagsKHR::empty());

                let instance =
                    ash::khr::xlib_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_xlib_surface(&info, None)? }
            }
            #[cfg(target_os = "windows")]
            (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
                if !self.extensions.surface_win32 {
                    return Err(Error::UnsupportedSurface);
                }

                let info = vk::Win32SurfaceCreateInfoKHR::default()
                    // - `hinstance` must be a valid Win32 `HINSTANCE`.
                    .hinstance(window.hinstance.map(|v| v.get()).unwrap_or_default())
                    // - `hwnd` must be a valid Win32 `HWND`.
                    .hwnd(window.hwnd.get())
                    // - `flags` must be `0`.
                    .flags(vk::Win32SurfaceCreateFlagsKHR::empty());

                let instance =
                    ash::khr::win32_surface::Instance::new(&self.instance.entry, &self.instance);
                unsafe { instance.create_win32_surface(&info, None)? }
            }
            _ => return Err(Error::UnsupportedSurface),
        };

        Ok(Surface {
            shared: Arc::new(SurfaceShared {
                instance: self.instance.clone(),
                surface,
            }),
        })
    }

    fn get_supported_layers(entry: &Entry) -> Result<InstanceLayers, Error> {
        let mut layers = InstanceLayers::default();

        let layer_props = unsafe { entry.enumerate_instance_layer_properties()? };
        for props in layer_props {
            let name = CStr::from_bytes_until_nul(bytemuck::bytes_of(&props.layer_name)).unwrap();

            match name {
                name if name == InstanceLayers::VALIDATION => layers.validation = true,
                _ => (),
            }
        }

        Ok(layers)
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

/// A physical graphics device.
#[derive(Debug)]
pub struct Adapter {
    instance: Arc<InstanceShared>,
    physical_device: vk::PhysicalDevice,
}

impl Adapter {
    /// Queries and returns general metadata about this `Adapter`.
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
            vk::PhysicalDeviceType::DISCRETE_GPU => AdapterKind::DiscreteGpu,
            vk::PhysicalDeviceType::INTEGRATED_GPU => AdapterKind::IntegratedGpu,
            vk::PhysicalDeviceType::CPU => AdapterKind::Cpu,
            _ => AdapterKind::Other,
        };

        AdapterProperties { name, kind }
    }

    /// Queries and returns information about this `Adapter`'s memories.
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
            .map(|(id, heap)| {
                let mut flags = MemoryHeapFlags::empty();
                if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) {
                    flags |= MemoryHeapFlags::DEVICE_LOCAL;
                }

                MemoryHeap {
                    id: id as u32,
                    size: heap.size,
                    flags,
                }
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
                    .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                {
                    flags |= MemoryTypeFlags::DEVICE_LOCAL;
                }
                if typ
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::HOST_VISIBLE)
                {
                    flags |= MemoryTypeFlags::HOST_VISIBLE;
                }
                if typ
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::HOST_COHERENT)
                {
                    flags |= MemoryTypeFlags::HOST_COHERENT;
                }
                if typ
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::PROTECTED)
                {
                    flags |= MemoryTypeFlags::_VK_PROTECTED;
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

    /// Queries and returns the queue families that can be created on this `Adapter`.
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

                if queue.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    capabilities |= QueueCapabilities::GRAPHICS;
                    // Graphics queues always have transfer capabilities.
                    capabilities |= QueueCapabilities::TRANSFER;
                }

                if queue.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                    capabilities |= QueueCapabilities::COMPUTE;
                    // Compute queues always have transfer capabilities.
                    capabilities |= QueueCapabilities::TRANSFER;
                }

                if queue.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                    capabilities |= QueueCapabilities::TRANSFER;
                }

                QueueFamily {
                    id: QueueFamilyId(index as u32),
                    count: queue.queue_count,
                    capabilities,
                }
            })
            .collect()
    }

    /// Creates a new [`Device`] with the given queues.
    pub fn create_device(&self, queue_families: &[QueueFamily]) -> Result<Device, Error> {
        let valid_queue_families = self.queue_families();

        for (i1, q1) in queue_families.iter().enumerate() {
            let Some(valid_queue) = valid_queue_families
                .iter()
                .find(|family| family.id == q1.id)
            else {
                panic!("Queue {:?} does not exist on this Device", queue_families);
            };

            assert!(
                q1.count <= valid_queue.count,
                "Cannot create more queues of family than exist"
            );

            assert_eq!(q1.capabilities, valid_queue.capabilities);

            // Every element in `queue_families` must be unique.
            for (i2, q2) in queue_families.iter().enumerate() {
                if i1 == i2 {
                    continue;
                }

                assert_ne!(
                    q1.id, q2.id,
                    "queue {:?} used multiple times in create_device",
                    q1,
                );
            }
        }

        // Use a default priority of 1.0 for every queue.
        let queue_count = queue_families
            .iter()
            .map(|family| family.count as usize)
            .max()
            .unwrap_or_default();
        let queue_priorities = vec![1.0; queue_count];

        let queue_infos = queue_families
            .iter()
            .map(|family| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(family.id.0)
                    .queue_priorities(&queue_priorities[..family.count as usize])
                    // - Must be equal to the flags used in `create_queue`.
                    .flags(vk::DeviceQueueCreateFlags::empty())
            })
            .collect::<Vec<_>>();

        let mut layers = Vec::new();
        if self.instance.config.validation {
            layers.push(InstanceLayers::VALIDATION.as_ptr());
        }

        let mut extensions = Vec::new();
        extensions.extend(DEVICE_EXTENSIONS.iter().map(|v| v.as_ptr()));

        let features = vk::PhysicalDeviceFeatures::default();

        let mut dynamic_rendering =
            vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);

        let mut synchronization2 =
            vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true);

        let mut descriptor_indexing = vk::PhysicalDeviceDescriptorIndexingFeatures::default()
            .shader_input_attachment_array_dynamic_indexing(true)
            .shader_uniform_texel_buffer_array_dynamic_indexing(true)
            .shader_storage_texel_buffer_array_dynamic_indexing(true)
            .shader_uniform_buffer_array_non_uniform_indexing(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .shader_storage_buffer_array_non_uniform_indexing(true)
            .shader_storage_image_array_non_uniform_indexing(true)
            .shader_input_attachment_array_non_uniform_indexing(true)
            .descriptor_binding_uniform_buffer_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_variable_descriptor_count(true)
            .runtime_descriptor_array(true);

        // Allow passing deprecated `enabled_layer_names`.
        #[allow(deprecated)]
        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            // Device layers are deprecated, but the Vulkan spec still recommends
            // applications to pass layers.
            // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#extendingvulkan-layers-devicelayerdeprecation
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features)
            .push_next(&mut dynamic_rendering)
            .push_next(&mut synchronization2)
            .push_next(&mut descriptor_indexing);

        let device = unsafe {
            self.instance
                .instance
                .create_device(self.physical_device, &create_info, None)?
        };

        let queues = queue_families
            .iter()
            .flat_map(|family| {
                (0..family.count).map(|index| QueueSlot {
                    id: family.id,
                    index,
                    used: Mutex::new(false),
                    caps: family.capabilities,
                })
            })
            .collect::<Vec<_>>()
            .into();

        Ok(Device {
            physical_device: self.physical_device,
            device: Arc::new(DeviceShared {
                instance: self.instance.clone(),
                device,
                limits: self.device_limits(),
                memory_properties: self.memory_properties(),
                num_allocations: Arc::new(AtomicU32::new(0)),
                queues,
            }),
        })
    }

    fn device_limits(&self) -> DeviceLimits {
        let props = unsafe {
            self.instance
                .instance
                .get_physical_device_properties(self.physical_device)
        };

        DeviceLimits {
            max_push_constants_size: props.limits.max_push_constants_size,
            max_bound_descriptor_sets: props.limits.max_bound_descriptor_sets,
            max_memory_allocation_count: props.limits.max_memory_allocation_count,
            buffer_image_granularity: props.limits.buffer_image_granularity,
            max_per_stage_descriptor_samplers: props.limits.max_per_stage_descriptor_samplers,
            max_per_stage_descriptor_uniform_buffers: props
                .limits
                .max_per_stage_descriptor_uniform_buffers,
            max_per_stage_descriptor_storage_buffers: props
                .limits
                .max_per_stage_descriptor_storage_buffers,
            max_per_stage_descriptor_sampled_images: props
                .limits
                .max_per_stage_descriptor_sampled_images,
            max_per_stage_resources: props.limits.max_per_stage_resources,
            max_descriptor_set_sampled_images: props.limits.max_descriptor_set_sampled_images,
            max_descriptor_set_samplers: props.limits.max_descriptor_set_samplers,
            max_descriptor_set_storage_buffers: props.limits.max_descriptor_set_storage_buffers,
            max_descriptor_set_uniform_buffers: props.limits.max_descriptor_set_uniform_buffers,
            max_color_attachments: props.limits.max_color_attachments,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    physical_device: vk::PhysicalDevice,
    device: Arc<DeviceShared>,
}

impl Device {
    /// Creates a new [`Queue`], bound to this `Device`.
    pub fn create_queue(&mut self, family: QueueFamilyId) -> Result<Queue, Error> {
        // Find a queue with the fitting `id` that is not yet
        // marked as `used`.
        let queue_index = self.device.queues.iter().find_map(|queue| {
            if queue.id != family {
                return None;
            }

            let mut used = queue.used.lock();
            if !*used {
                *used = true;
                Some(queue.index)
            } else {
                None
            }
        });
        let Some(queue_index) = queue_index else {
            return Err(Error::NoQueueLeft);
        };

        let info = vk::DeviceQueueInfo2::default()
            .queue_family_index(family.0)
            // Index is always 0 since we only create
            // a single queue for now.
            .queue_index(queue_index)
            // - Must be equal to the flags used when calling `vkCreateDevice`.
            .flags(vk::DeviceQueueCreateFlags::empty());

        let queue = unsafe { self.device.get_device_queue2(&info) };

        Ok(Queue {
            device: self.device.clone(),
            queue,
            queue_family: family,
            queue_index,
        })
    }

    /// Creates a new [`Buffer`] with the given size and usage flags.
    pub fn create_buffer(&self, size: NonZeroU64, usage: BufferUsage) -> Result<Buffer, Error> {
        let mut buffer_usage_flags = vk::BufferUsageFlags::empty();
        if usage.contains(BufferUsage::TRANSFER_SRC) {
            buffer_usage_flags |= vk::BufferUsageFlags::TRANSFER_SRC;
        }
        if usage.contains(BufferUsage::TRANSFER_DST) {
            buffer_usage_flags |= vk::BufferUsageFlags::TRANSFER_DST;
        }
        if usage.contains(BufferUsage::UNIFORM) {
            buffer_usage_flags |= vk::BufferUsageFlags::UNIFORM_BUFFER;
        }
        if usage.contains(BufferUsage::STORAGE) {
            buffer_usage_flags |= vk::BufferUsageFlags::STORAGE_BUFFER;
        }
        if usage.contains(BufferUsage::VERTEX) {
            buffer_usage_flags |= vk::BufferUsageFlags::VERTEX_BUFFER;
        }
        if usage.contains(BufferUsage::INDEX) {
            buffer_usage_flags |= vk::BufferUsageFlags::INDEX_BUFFER;
        }
        if usage.contains(BufferUsage::INDIRECT) {
            buffer_usage_flags |= vk::BufferUsageFlags::INDIRECT_BUFFER;
        }

        assert!(!buffer_usage_flags.is_empty());

        let info = vk::BufferCreateInfo::default()
            // - `size` must be greater than 0.
            .size(size.get())
            // - `usage` must not be 0. (Unless `VkBufferUsageFlags2CreateInfo` is used.)
            // Checked above.
            .usage(buffer_usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&info, None)? };
        Ok(Buffer {
            buffer,
            device: self.device.clone(),
            size: size.get(),
            usages: buffer_usage_flags,
        })
    }

    /// Allocates some [`DeviceMemory`] from the memory type given by `memory_type_index`.
    pub fn allocate_memory(
        &self,
        size: NonZeroU64,
        memory_type_index: u32,
    ) -> Result<DeviceMemory, Error> {
        let heap = self.device.memory_properties.types[memory_type_index as usize].heap;

        assert!(
            !self.device.memory_properties.types[memory_type_index as usize]
                .flags
                .contains(MemoryTypeFlags::_VK_PROTECTED),
        );

        let info = vk::MemoryAllocateInfo::default()
            // - `allocationSize` must be greater than 0.
            .allocation_size(size.get())
            // - memoryTypeIndex must not indicate a memory type that reports `VK_MEMORY_PROPERTY_PROTECTED_BIT`.
            .memory_type_index(memory_type_index);

        assert!(
            size.get() <= u64::from(self.device.memory_properties.heaps[heap as usize].size),
            "attempted to allocate more than heap size: heap size = {}, allocation = {}",
            self.device.memory_properties.heaps[heap as usize].size,
            size,
        );

        if let Err(_) = self.device.num_allocations.fetch_update(
            Ordering::Release,
            Ordering::Acquire,
            |count| {
                // Unreachable using the CAS logic.
                debug_assert!(count <= self.device.limits.max_memory_allocation_count);

                // Increase the allocation count by one, but don't go over
                // `max_memory_allocation_count`.
                count
                    .checked_add(1)
                    .filter(|count| *count <= self.device.limits.max_memory_allocation_count)
            },
        ) {
            return Err(Error::NoAllocationsLeft);
        }

        let res = unsafe {
            // - `allocationSize` must be less than or equal to `memoryHeaps[heap].size`.
            // - `memoryTypeIndex` must be less than `VkPhysicalDeviceMemoryProperties::memoryTypeCount`.
            // - There must be less than `VkPhysicalDeviceLimits::maxMemoryAllocationCount` active.
            self.device.allocate_memory(&info, None)
        };

        match res {
            Ok(memory) => Ok(DeviceMemory {
                memory,
                device: self.device.clone(),
                size,
                flags: self.device.memory_properties.types[memory_type_index as usize].flags,
                memory_type: memory_type_index,
                is_mapped: false,
            }),
            Err(err) => {
                // If the allocation does not succeed it does not count
                // towards the active allocation count.
                // Since we have incremented the count by one this decrement
                // will never overflow.
                self.device.num_allocations.fetch_sub(1, Ordering::Release);
                Err(err.into())
            }
        }
    }

    /// Returns the [`MemoryRequirements`] for a [`Buffer`].
    pub fn buffer_memory_requirements(&self, buffer: &Buffer) -> MemoryRequirements {
        // - `buffer` must have been created from the same `device`.
        assert!(self.device.same(&buffer.device));

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

        // See https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#resources-association
        // - The `alignment` member is a power of two.
        debug_assert!(req.alignment.is_power_of_two());

        MemoryRequirements {
            size: unsafe { NonZeroU64::new_unchecked(req.size) },
            align: unsafe { NonZeroU64::new_unchecked(req.alignment) },
            memory_types,
        }
    }

    /// Returns the [`MemoryRequirements`] for a [`Texture`].
    pub fn image_memory_requirements(&self, texture: &Texture) -> MemoryRequirements {
        assert!(self.device.same(&texture.device));

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

        // See https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#resources-association
        // - The `alignment` member is a power of two.
        debug_assert!(req.alignment.is_power_of_two());

        // To handle `bufferImageGranularity` we just overalign all images
        // to `bufferImageGranularity`. This means the image will always
        // start on a fresh "page".
        // To ensure that the next resource is a new "page" we grow the size
        // to the next multiple of `bufferImageGranularity`.
        // This is usually not a problem, since images already have a big
        // alignment and size and `bufferImageGranularity` is usually relatively small.
        let buffer_image_granularity = self.device.limits.buffer_image_granularity;
        let align = u64::max(req.alignment, buffer_image_granularity);
        // size + (size % align) = (size + align - 1) & !(align - 1)
        let size = (req.size + buffer_image_granularity - 1) & !(buffer_image_granularity - 1);

        debug_assert_eq!(align % self.device.limits.buffer_image_granularity, 0);
        debug_assert_eq!(size % self.device.limits.buffer_image_granularity, 0);

        MemoryRequirements {
            size: unsafe { NonZeroU64::new_unchecked(size) },
            align: unsafe { NonZeroU64::new_unchecked(align) },
            memory_types,
        }
    }

    /// Binds memory to a [`Buffer`] object.
    ///
    /// # Safety
    ///
    /// - The memory range bound to the [`Buffer`] must not be bound to any other resource for the
    /// entire lifetime of the [`Buffer`] object.
    /// - The same memory range must not be bound to any other resource.
    pub unsafe fn bind_buffer_memory(
        &self,
        buffer: &mut Buffer,
        memory: DeviceMemorySlice<'_>,
    ) -> Result<(), Error> {
        // - `buffer` must have been created from the same device.
        // - `memory` must have been created from the same device.
        assert!(self.device.same(&buffer.device));
        assert!(self.device.same(&memory.memory.device));

        let reqs = self.buffer_memory_requirements(buffer);

        // `memoryOffset` must be less than the size of `memory`.
        assert!(memory.offset <= memory.memory.size.get());
        // - `memoryOffset` must be an integer multiple of the `alignment`.
        assert!(memory.offset % reqs.align.get() == 0);
        // - `size` must be less than or equal to the size of `memory` minus `memoryOffset`.
        assert!(memory.size <= memory.memory.size.get() - memory.offset);
        // - `memory` must have been allocated using one of the memory types.
        assert!(reqs.memory_types.contains(&memory.memory.memory_type));

        // https://registry.khronos.org/vulkan/specs/latest/man/html/VkBindBufferMemoryInfo.html
        let info = vk::BindBufferMemoryInfo::default()
            // - `buffer` must not have been bound to a memory object.
            // - `buffer` must not have been created with any sparse memory binding flags.
            .buffer(buffer.buffer)
            .memory(memory.memory.memory)
            .memory_offset(memory.offset);

        unsafe {
            self.device.bind_buffer_memory2(&[info])?;
        }

        Ok(())
    }

    /// Binds memory to a [`Texture`] object.
    ///
    /// # Safety
    ///
    /// - The memory range described by [`DeviceMemorySlice`] must not have been bound to any other
    /// live object yet.
    /// - [`Texture`] must not have been bound yet.
    pub unsafe fn bind_texture_memory(
        &self,
        texture: &mut Texture,
        memory: DeviceMemorySlice<'_>,
    ) -> Result<(), Error> {
        // - `image` must have been created from the same device.
        // - `memory` must have been created from the same device.
        assert!(self.device.same(&texture.device));
        assert!(self.device.same(&memory.memory.device));

        let reqs = self.image_memory_requirements(&texture);

        // `memoryOffset` must be less than the size of `memory`.
        assert!(memory.offset <= memory.memory.size.get());
        // `memory` must have been allocated using one of the memory types.
        assert!(reqs.memory_types.contains(&memory.memory.memory_type));
        // `memoryOffset` must be an integer multiple of `alignment`.
        assert!(memory.offset % reqs.align.get() == 0);

        let info = vk::BindImageMemoryInfo::default()
            // - `image` must not have been bound to a memory object.
            // - `image` must not have been created with any sparse memory binding flags.
            .image(texture.image)
            .memory(memory.memory.memory)
            .memory_offset(memory.offset);

        unsafe {
            self.device.bind_image_memory2(&[info])?;
        }

        Ok(())
    }

    /// Creates a new [`Texture`].
    ///
    /// The returned texture will have empty [`AccessFlags`].
    pub fn create_texture(&self, descriptor: &TextureDescriptor) -> Result<Texture, Error> {
        assert_ne!(descriptor.size.x, 0);
        assert_ne!(descriptor.size.y, 0);

        let extent = vk::Extent3D {
            // - `width` must be greater than 0.
            width: descriptor.size.x,
            // - `height` must be greater than 0.
            height: descriptor.size.y,
            // - `depth` must be greater than 0.
            // - `depth` must be 1, since `imageType` is always `VK_IMAGE_TYPE_2D`.
            depth: 1,
        };

        let mut usages: vk::ImageUsageFlags = descriptor.usage.into();
        if descriptor.usage.contains(TextureUsage::RENDER_ATTACHMENT) {
            if descriptor.format.is_depth() {
                usages |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
            } else {
                usages |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
            }
        }

        assert!(!usages.is_empty());

        let format_info = vk::PhysicalDeviceImageFormatInfo2::default()
            .format(descriptor.format.into())
            .ty(vk::ImageType::TYPE_2D)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usages)
            .flags(vk::ImageCreateFlags::empty());
        let mut format_out = vk::ImageFormatProperties2::default();
        unsafe {
            self.device
                .instance
                .get_physical_device_image_format_properties2(
                    self.physical_device,
                    &format_info,
                    &mut format_out,
                )?;
        }

        // - `extent.width` must be less than or equal to `imageCreateMaxExtent.width`.
        // - `extent.height` must be less than or equal to `imageCreateMaxExtent.height`.
        // - `extent.depth` must be less than or equal to `imageCreateMaxExtent.depth`.
        assert!(extent.width <= format_out.image_format_properties.max_extent.width);
        assert!(extent.height <= format_out.image_format_properties.max_extent.height);
        assert!(extent.depth <= format_out.image_format_properties.max_extent.depth);
        // - `mipLevels` must be less than or equal to `imageCreateMaxMipLevels`.
        assert!(descriptor.mip_levels <= format_out.image_format_properties.max_mip_levels);
        // - `arrayLayers` must be less than or equal to `imageCreateMaxArrayLayers`.
        assert!(1 <= format_out.image_format_properties.max_array_layers);

        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(descriptor.mip_levels)
            .array_layers(1)
            .format(descriptor.format.into())
            .tiling(vk::ImageTiling::OPTIMAL)
            // - `initialLayout` must be `VK_IMAGE_LAYOUT_UNDEFINED`.
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usages)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1)
            .flags(vk::ImageCreateFlags::empty());

        let image = unsafe { self.device.create_image(&info, None)? };
        Ok(Texture {
            device: self.device.clone(),
            image,
            format: descriptor.format,
            size: descriptor.size,
            destroy_on_drop: true,
            mip_levels: descriptor.mip_levels,
        })
    }

    unsafe fn create_shader(&self, code: &[u32]) -> ShaderModule {
        // Code size must be greater than 0.
        debug_assert!(code.len() != 0);

        let info = vk::ShaderModuleCreateInfo::default().code(code);

        let shader = unsafe { self.device.create_shader_module(&info, None).unwrap() };
        ShaderModule {
            device: self.device.clone(),
            shader,
        }
    }

    /// Creates a new [`DescriptorSetLayout`].
    pub fn create_descriptor_layout(
        &self,
        descriptor: &DescriptorSetDescriptor<'_>,
    ) -> Result<DescriptorSetLayout, Error> {
        let mut bindings = Vec::new();
        let mut flags = Vec::new();

        for binding in descriptor.bindings {
            let info = vk::DescriptorSetLayoutBinding::default()
                .binding(binding.binding)
                .stage_flags(binding.visibility.into())
                .descriptor_count(binding.count.get())
                .descriptor_type(binding.kind.into());

            bindings.push(info);
            flags.push(vk::DescriptorBindingFlags::PARTIALLY_BOUND);
        }

        let mut flags =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&flags);

        let info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings)
            .push_next(&mut flags)
            .flags(vk::DescriptorSetLayoutCreateFlags::empty());
        let layout = unsafe { self.device.create_descriptor_set_layout(&info, None)? };

        Ok(DescriptorSetLayout {
            device: self.device.clone(),
            layout,
            bindings: descriptor.bindings.to_vec(),
        })
    }

    /// Creates a new [`Pipeline`].
    pub fn create_pipeline(&self, descriptor: &PipelineDescriptor<'_>) -> Result<Pipeline, Error> {
        let descriptors = descriptor
            .descriptors
            .iter()
            .map(|layout| layout.layout)
            .collect::<Vec<_>>();

        let mut samplers = 0;
        let mut uniform_buffers = 0;
        let mut storage_buffers = 0;
        let mut textures = 0;
        for layout in descriptor.descriptors {
            for binding in &layout.bindings {
                let count = binding.count.get();
                match binding.kind {
                    // Equivalent to SAMPLER
                    DescriptorType::Sampler => samplers += count,
                    // Equivalent to UNIFORM_BUFFER
                    DescriptorType::Uniform => uniform_buffers += count,
                    // Equivalent to STORAGE_BUFFER
                    DescriptorType::Storage => storage_buffers += count,
                    // Equivalent to SAMPLED_IMAGE
                    DescriptorType::Texture => textures += count,
                }
            }
        }

        // These must be true accross all pipeline stages.
        assert!(samplers <= self.device.limits.max_descriptor_set_samplers);
        assert!(uniform_buffers <= self.device.limits.max_descriptor_set_uniform_buffers);
        assert!(storage_buffers <= self.device.limits.max_descriptor_set_storage_buffers);
        assert!(textures <= self.device.limits.max_descriptor_set_sampled_images);

        // These must only be true for each pipeline stage individually.
        // FIXME: Right now count all descriptors in all pipeline stages,
        // which is more restrictive that necessary.
        assert!(samplers <= self.device.limits.max_per_stage_descriptor_samplers);
        assert!(uniform_buffers <= self.device.limits.max_per_stage_descriptor_uniform_buffers);
        assert!(storage_buffers <= self.device.limits.max_per_stage_descriptor_storage_buffers);
        assert!(textures <= self.device.limits.max_per_stage_descriptor_sampled_images);
        assert!(
            samplers + uniform_buffers + storage_buffers + textures
                <= self.device.limits.max_per_stage_resources
        );

        let push_constant_ranges = descriptor
            .push_constant_ranges
            .iter()
            .map(|r| {
                assert!(r.range.end > r.range.start);

                let offset = r.range.start;
                let size = r.range.end - r.range.start;
                let stage_flags: vk::ShaderStageFlags = r.stages.into();

                assert!(offset < self.device.limits.max_push_constants_size);
                assert!(offset % 4 == 0);
                assert!(size > 0);
                assert!(size % 4 == 0);
                assert!(size <= self.device.limits.max_push_constants_size - offset);
                assert!(!stage_flags.is_empty());

                vk::PushConstantRange {
                    // - `offset` must be less than `VkPhysicalDeviceLimits::maxPushConstantsSize`.
                    // - `offset` must be a multiple of 4.
                    offset,
                    // - `size` must be greater than 0.
                    // - `size` must be a multiple of 4.
                    // - `size` must be less than or equal to `VkPhysicalDeviceLimits::maxPushConstantsSize` minus `offset`.
                    size,
                    // - `stageFlags` must not be 0.
                    stage_flags,
                }
            })
            .collect::<Vec<_>>();

        assert!(descriptors.len() as u32 <= self.device.limits.max_bound_descriptor_sets);

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            // - `setLayoutCount` must be less than or equal to `VkPhysicalDeviceLimits::maxBoundDescriptorSets`.
            .set_layouts(&descriptors)
            .push_constant_ranges(&push_constant_ranges);
        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_info, None)?
        };

        let mut stages = Vec::new();
        let mut color_attchment_formats: Vec<vk::Format> = Vec::new();

        // We need exactly one `VK_SHADER_STAGE_VERTEX_BIT` or `VK_SHADER_STAGE_MESH_BIT_EXT` stage.
        assert_eq!(
            descriptor
                .stages
                .iter()
                .filter(|stage| matches!(stage, PipelineStage::Vertex(_)))
                .count(),
            1,
            "Exactly one VERTEX or MESH shader stage is needed",
        );

        let shader_modules = ScratchBuffer::new(descriptor.stages.len());
        let stage_entry_pointers = ScratchBuffer::new(descriptor.stages.len());
        for stage in descriptor.stages {
            let vk_stage = match stage {
                PipelineStage::Vertex(stage) => {
                    let spirv = create_pipeline_shader_module(
                        &stage.shader.shader,
                        stage.entry,
                        ShaderStage::Vertex,
                        descriptor.descriptors,
                    );

                    let module = shader_modules.insert(unsafe { self.create_shader(&spirv) });
                    let name = stage_entry_pointers.insert(CString::new(stage.entry).unwrap());

                    validate_shader_bindings(stage.shader, descriptor.descriptors);

                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::VERTEX)
                        .module(module.shader)
                        .name(&*name)
                }
                PipelineStage::Fragment(stage) => {
                    color_attchment_formats
                        .extend(stage.targets.iter().copied().map(vk::Format::from));

                    let spirv = create_pipeline_shader_module(
                        &stage.shader.shader,
                        stage.entry,
                        ShaderStage::Fragment,
                        descriptor.descriptors,
                    );

                    let module = shader_modules.insert(unsafe { self.create_shader(&spirv) });
                    let name = stage_entry_pointers.insert(CString::new(stage.entry).unwrap());

                    validate_shader_bindings(stage.shader, descriptor.descriptors);

                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::FRAGMENT)
                        .module(module.shader)
                        .name(&*name)
                }
            };

            stages.push(vk_stage);
        }

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default();

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(descriptor.topology.into())
            .primitive_restart_enable(false);

        // We use dynamic viewport and scissors, so the actual viewport and scissors
        // pointers are ignored. We still have to enter the correct count of viewport/
        // scissors.
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            // - `viewportCount` must be less than or equal to `VkPhysicalDeviceLimits::maxViewports`.
            // - `viewportCount` must not be greater than 1. (If `multiViewport` feature is not enabled.)
            // - `viewportCount` must be greater than 0. (If `VK_DYNAMIC_STATE_VIEWPORT_WITH_COUNT` not set.)
            .viewport_count(1)
            // - `scissorCount` must be less than or eual to `VkPhysicalDeviceLimits::maxViewports`.
            // - `scissorCount` must not be greater than 1. (If `multiViewport` feature is not enabled.)
            // - `scissorCount` must be greater than 0. (If `VK_DYNAMIC_STATE_SCISSOR_WITH_COUNT` not set.)
            .scissor_count(1);

        let cull_mode = match descriptor.cull_mode {
            Some(Face::Front) => vk::CullModeFlags::FRONT,
            Some(Face::Back) => vk::CullModeFlags::BACK,
            None => vk::CullModeFlags::NONE,
        };

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_bias_enable(true)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(cull_mode)
            .front_face(descriptor.front_face.into());

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);

        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        let depth_stencil_state = descriptor.depth_stencil_state.as_ref().map(|state| {
            vk::PipelineDepthStencilStateCreateInfo::default()
                .flags(vk::PipelineDepthStencilStateCreateFlags::empty())
                .depth_test_enable(true)
                .depth_write_enable(state.depth_write_enabled)
                .depth_compare_op(state.depth_compare_op.into())
                .depth_bounds_test_enable(false)
                // TODO: Add API for this.
                .stencil_test_enable(false)
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0)
        });

        assert!(color_attchment_formats.len() <= self.device.limits.max_color_attachments as usize);
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            // - `colorAttachmentCount` must be less than `VkPhysicalDeviceLimits::maxColorAttachments`.
            .color_attachment_formats(&color_attchment_formats);

        if let Some(state) = &descriptor.depth_stencil_state {
            rendering_info = rendering_info.depth_attachment_format(state.format.into());
        }

        let mut info = vk::GraphicsPipelineCreateInfo::default()
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

        if let Some(state) = &depth_stencil_state {
            info = info.depth_stencil_state(&state);
        }

        let pipelines = match unsafe {
            self.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
        } {
            Ok(pipeline) => pipeline,
            Err((pipelines, err)) => {
                debug_assert!(pipelines.is_empty());

                unsafe {
                    self.device.destroy_pipeline_layout(pipeline_layout, None);
                }

                return Err(err.into());
            }
        };

        // Shaders can be destroyed after the pipeline was created.
        drop(shader_modules);

        Ok(Pipeline {
            device: self.device.clone(),
            pipeline: pipelines[0],
            pipeline_layout,
        })
    }

    /// Creates a new [`CommandPool`].
    ///
    /// The buffers allocated from that [`CommandPool`] may only be used in [`Queue`]s which match
    /// the provided [`QueueFamilyId`].
    pub fn create_command_pool(&self, queue_family: QueueFamilyId) -> Result<CommandPool, Error> {
        let Some(queue) = self
            .device
            .queues
            .iter()
            .find(|queue| queue.id == queue_family)
        else {
            panic!(
                "Cannot create command pool for queue family {:?} which does not exist",
                queue_family,
            );
        };

        // All command buffers must only be used on queues with the given `queue_family`.
        let info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::empty())
            .queue_family_index(queue_family.0);

        let pool = unsafe { self.device.create_command_pool(&info, None)? };

        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let buffers = match unsafe { self.device.allocate_command_buffers(&info) } {
            Ok(buffers) => buffers,
            Err(err) => {
                // Destroy the previously created pool. It is not destroyed
                // automatically in this function.
                unsafe {
                    self.device.destroy_command_pool(pool, None);
                }

                return Err(err.into());
            }
        };

        Ok(CommandPool {
            device: self.device.clone(),
            pool,
            buffers,
            next_buffer: 0,
            queue_family,
            queue_caps: queue.caps,
        })
    }

    /// Creates a new [`Semaphore`].
    pub fn create_semaphore(&self) -> Result<Semaphore, Error> {
        let info = vk::SemaphoreCreateInfo::default();

        let semaphore = unsafe { self.device.create_semaphore(&info, None)? };

        Ok(Semaphore {
            device: self.device.clone(),
            semaphore,
        })
    }

    /// Creates a new [`DescriptorPool`].
    pub fn create_descriptor_pool(
        &self,
        descriptor: &DescriptorPoolDescriptor,
    ) -> Result<DescriptorPool, Error> {
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
            (vk::DescriptorType::SAMPLER, descriptor.max_samplers),
            (
                vk::DescriptorType::SAMPLED_IMAGE,
                descriptor.max_sampled_images,
            ),
        ] {
            if count == 0 {
                continue;
            }

            // - `descriptorCount` must be greater than 0.
            let size = vk::DescriptorPoolSize::default()
                .ty(ty)
                .descriptor_count(count);

            sizes.push(size);
        }

        let info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&sizes)
            // - `maxSets` must be greater than 0.
            .max_sets(descriptor.max_sets.get());

        let pool = unsafe { self.device.create_descriptor_pool(&info, None)? };

        Ok(DescriptorPool {
            device: self.device.clone(),
            pool,
        })
    }

    /// Creates a new [`Fence`].
    pub fn create_fence(&self) -> Result<Fence, Error> {
        let info = vk::FenceCreateInfo::default();

        let fence = unsafe { self.device.create_fence(&info, None)? };
        Ok(Fence {
            device: self.device.clone(),
            fence,
            state: FenceState::Idle,
        })
    }

    /// Creates a new [`Sampler`].
    pub fn create_sampler(&self, descriptor: &SamplerDescriptor) -> Result<Sampler, Error> {
        let info = vk::SamplerCreateInfo::default()
            .min_filter(descriptor.min_filter.into())
            .mag_filter(descriptor.mag_filter.into())
            .address_mode_u(descriptor.address_mode_u.into())
            .address_mode_v(descriptor.address_mode_v.into())
            .address_mode_w(descriptor.address_mode_w.into())
            .mipmap_mode(descriptor.mipmap_filter.into())
            .min_lod(0.0)
            .max_lod(100.0)
            .mip_lod_bias(0.0)
            // TODO: Add API for this
            .anisotropy_enable(false)
            .max_anisotropy(1.0);

        let sampler = unsafe { self.device.create_sampler(&info, None)? };
        Ok(Sampler {
            device: self.device.clone(),
            sampler,
        })
    }
}

#[derive(Debug)]
pub struct Queue {
    device: Arc<DeviceShared>,
    queue: vk::Queue,
    queue_family: QueueFamilyId,
    queue_index: u32,
}

impl Queue {
    /// Returns the [`QueueFamily`] ID that was used to create this `Queue`.
    pub fn family(&self) -> QueueFamilyId {
        self.queue_family
    }

    /// Submits a list of [`CommandBuffer`]s to this `Queue`.
    pub fn submit<'a, T>(&mut self, buffers: T, cmd: QueueSubmit<'_>) -> Result<(), Error>
    where
        T: IntoIterator<Item = CommandBuffer<'a>>,
    {
        let buffers: Vec<_> = buffers
            .into_iter()
            .map(|buf| {
                assert_eq!(
                    buf.queue_family, self.queue_family,
                    "Queue with family {:?} cannot submit buffer allocated for queue family {:?}",
                    self.queue_family, buf.queue_family,
                );

                buf.buffer
            })
            .collect();

        // TODO: Give the caller control of this.
        // To relaxed this to COLOR_ATTACHMENT_OUTPUT stage we need
        // to insert a barrier from COLOR_ATTACHMENT_OUTPUT->COLOR_ATTACHMENT_OUTPUT
        // when doing the UNDEFINED->COLOR_ATTACHMENT_OPTIMAL transition.
        // https://github.com/KhronosGroup/Vulkan-ValidationLayers/issues/7193#issuecomment-1875960974
        let wait_stage = vk::PipelineStageFlags::TOP_OF_PIPE;

        let wait_semaphores: Vec<_> = cmd
            .wait
            .iter()
            .map(|semaphore| semaphore.semaphore)
            .collect();
        let wait_stages: Vec<_> = std::iter::repeat_n(wait_stage, cmd.wait.len()).collect();
        let signal_semaphores: Vec<_> = cmd
            .signal
            .iter()
            .map(|semaphore| semaphore.semaphore)
            .collect();

        let info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&buffers)
            .signal_semaphores(&signal_semaphores);

        // The fence should be unsignaled and not already in use by another
        // object.
        assert_eq!(cmd.signal_fence.state, FenceState::Idle);
        cmd.signal_fence.state = FenceState::Waiting;

        unsafe {
            self.device
                .device
                .queue_submit(self.queue, &[info], cmd.signal_fence.fence)?;
        }
        Ok(())
    }

    /// Waits for the `Queue` to become idle.
    ///
    /// When this function returns all previously submitted command buffers on this `Queue` have
    /// finished execution.
    pub fn wait_idle(&mut self) -> Result<(), Error> {
        unsafe {
            // - Access to `queue` must be externally synchronized.
            self.device.device.queue_wait_idle(self.queue)?;
            Ok(())
        }
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        for queue in self.device.queues.iter() {
            if queue.id != self.queue_family || queue.index != self.queue_index {
                continue;
            }

            // Release the queue by marking it as unused.
            let mut used = queue.used.lock();
            debug_assert!(*used);
            *used = false;
        }
    }
}

#[derive(Debug)]
struct SurfaceShared {
    instance: Arc<InstanceShared>,
    surface: vk::SurfaceKHR,
}

impl SurfaceShared {
    /// Creates a new [`SwapchainKHR`] and returns its images.
    ///
    /// If `old_swapchain` is not [`null`], it will be invalidated and cannot be used anymore. It
    /// must still be destroyed however.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if creating the new swapchain fails.
    ///
    /// If this function returns an [`Error`], the passed `old_swapchain` will still become
    /// invalidated and must be destroyed as mentioned above.
    ///
    /// # Safety
    ///
    /// `old_swapchain` must be either null or a non-retired swapchain created by this `Surface`.
    ///
    /// [`null`]: SwapchainKHR::null
    unsafe fn create_swapchain_inner(
        &self,
        device: &Device,
        config: &SwapchainConfig,
        caps: &SwapchainCapabilities,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<(vk::SwapchainKHR, Vec<vk::Image>), Error> {
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
            .contains(vk::CompositeAlphaFlagsKHR::OPAQUE));

        assert!(caps.present_modes.contains(&config.present_mode));

        assert!(caps.formats.contains(&config.format));

        let info = vk::SwapchainCreateInfoKHR::default()
            // - Surface must be supported. This is checked by the call to `get_capabilities` above.
            .surface(self.surface)
            // - `minImageCount` must be less than or equal to the `maxImageCount`. Checked above.
            // - `minImageCount` must be greater than or equal to `minImageCount`. Checked above.
            .min_image_count(config.image_count)
            // - `imageFormat` must match one of the formats returned by `vkGetPhysicalDeviceSurfaceFormatsKHR`.
            // Checked above.
            .image_format(config.format.format.into())
            // - `imageColorSpace` must match one of the formats returned by `vkGetPhysicalDeviceSurfaceFormatsKHR`.
            // Checked above.
            .image_color_space(config.format.color_space.into())
            // - `width` and `height` must both ne non-zero. Checked above.
            // - `width` and `height` must be between `minImageExtent` and `maxImageExtent`. Checked above.
            .image_extent(vk::Extent2D {
                width: config.extent.x,
                height: config.extent.y,
            })
            // - `imageArrayLayers` must be at least 1 and less than or equal to `maxImageArrayLayers`.
            // `vkGetPhysicalDeviceSurfaceCapabilitiesKHR` is required to always return at least 1.
            // This means the value `1` is always valid here.
            .image_array_layers(1)
            // - `imageUsage` must be a set of `supportedUsageFlags`.
            // `VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT` must always be included, so this value is always valid.
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            // - Must be a list of queues which are allowed to access the swapchain images when
            // the `imageSharingMode` is `CONCURRENT`.
            // We only use `EXCLUSIVE`, so this can be empty.
            .queue_family_indices(&[])
            // - `compositeAlpha` must be one bit from `supportedCompositeAlpha`. Checked above.
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
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
        let swapchain = unsafe { khr_device.create_swapchain(&info, None)? };

        let images = match unsafe { khr_device.get_swapchain_images(swapchain) } {
            Ok(images) => images,
            Err(err) => {
                // We will not return the new swapchain object from this function
                // on error. This means the newly created swapchain needs to be
                // destroyed manually, otherwise it will leak.
                unsafe {
                    khr_device.destroy_swapchain(swapchain, None);
                }

                return Err(err.into());
            }
        };

        Ok((swapchain, images))
    }
}

impl Drop for SurfaceShared {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        let instance =
            ash::khr::surface::Instance::new(&self.instance.entry, &self.instance.instance);

        unsafe {
            instance.destroy_surface(self.surface, None);
        }
    }
}

#[derive(Debug)]
pub struct Surface {
    shared: Arc<SurfaceShared>,
}

impl Surface {
    /// Returns the [`SwapchainCapabilities`] that can be used for this `Surface`.
    ///
    /// The passed [`Queue`] will be the queue used to present the swapchain.
    pub fn get_capabilities(
        &self,
        device: &Device,
        queue: &Queue,
    ) -> Result<SwapchainCapabilities, Error> {
        // - `physicalDevice` and `surface` must have been created from the same `VkInstance`.
        assert!(self.shared.instance.same(&device.device.instance));

        let instance = ash::khr::surface::Instance::new(
            &self.shared.instance.entry,
            &self.shared.instance.instance,
        );

        let is_supported = unsafe {
            instance.get_physical_device_surface_support(
                device.physical_device,
                queue.queue_family.0,
                self.shared.surface,
            )?
        };

        if !is_supported {
            return Err(Error::UnsupportedSurface);
        }

        let caps = unsafe {
            instance.get_physical_device_surface_capabilities(
                device.physical_device,
                self.shared.surface,
            )?
        };
        let formats = unsafe {
            instance
                .get_physical_device_surface_formats(device.physical_device, self.shared.surface)?
        };
        let present_modes = unsafe {
            instance.get_physical_device_surface_present_modes(
                device.physical_device,
                self.shared.surface,
            )?
        };

        // Vulkan spec requires that `maxImageArrayLayers` is at least one.
        debug_assert!(caps.max_image_array_layers >= 1);

        // Vulkan spec requires that `VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT` must be included.
        debug_assert!(caps
            .supported_usage_flags
            .contains(vk::ImageUsageFlags::COLOR_ATTACHMENT));

        // FIXME: This does not seem strictly required by the Vulkan spec?
        // See https://github.com/KhronosGroup/Vulkan-Docs/issues/2440
        assert!(caps.supported_transforms.contains(caps.current_transform));

        Ok(SwapchainCapabilities {
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
            formats: formats
                .into_iter()
                .filter_map(|v| {
                    let format = v.format.try_into().ok();
                    let color_space = v.color_space.try_into().ok();
                    format
                        .zip(color_space)
                        .map(|(format, color_space)| SurfaceFormat {
                            format,
                            color_space,
                        })
                })
                .collect(),
            present_modes: present_modes
                .into_iter()
                .filter_map(|v| v.try_into().ok())
                .collect(),
            current_transform: caps.current_transform,
            supported_composite_alpha: caps.supported_composite_alpha,
        })
    }

    pub fn create_swapchain(
        &self,
        device: &Device,
        config: SwapchainConfig,
        caps: &SwapchainCapabilities,
    ) -> Result<Swapchain, Error> {
        // SAFETY: `old_swapchain` is null.
        let (swapchain, images) = unsafe {
            self.shared
                .create_swapchain_inner(device, &config, &caps, vk::SwapchainKHR::null())?
        };

        Ok(Swapchain {
            surface: self.shared.clone(),
            device: device.clone(),
            swapchain,
            images,
            format: config.format,
            extent: config.extent,
        })
    }
}

#[derive(Debug)]
pub struct Swapchain {
    surface: Arc<SurfaceShared>,
    device: Device,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,

    format: SurfaceFormat,
    extent: UVec2,
}

impl Swapchain {
    /// Recreates the `Swapchain` using the new [`SwapchainConfig`].
    ///
    /// # Safety
    ///
    /// - All of the swapchain textures acquired from [`acquire_next_image`] must not be in use
    /// currently.
    pub unsafe fn recreate(
        &mut self,
        config: SwapchainConfig,
        caps: &SwapchainCapabilities,
    ) -> Result<(), Error> {
        // SAFETY: `self.swapchain` is a valid swapchain created by `self.surface`.
        // Since this function accepts a mutable reference this swapchain is not used.
        let (swapchain, images) = match unsafe {
            self.surface
                .create_swapchain_inner(&self.device, &config, caps, self.swapchain)
        } {
            Ok((swapchain, images)) => (swapchain, images),
            Err(err) => {
                // The `old_swapchain` still becomes invalidated if an error is returned.
                // Note that if this function returned an error prior, the swapchain was
                // already destroyed and is null.
                if !self.swapchain.is_null() {
                    unsafe {
                        let device = ash::khr::swapchain::Device::new(
                            &self.surface.instance.instance,
                            &self.device.device,
                        );
                        device.destroy_swapchain(self.swapchain, None);
                    }
                }

                self.swapchain = vk::SwapchainKHR::null();
                return Err(err.into());
            }
        };

        // The swapchain still needs to be destroyed after it has been invalidated.
        // Note that if this function returned an error prior, the swapchain was
        // already destroyed and is null.
        if !self.swapchain.is_null() {
            unsafe {
                let device = ash::khr::swapchain::Device::new(
                    &self.surface.instance.instance,
                    &self.device.device,
                );
                device.destroy_swapchain(self.swapchain, None);
            }
        }

        self.swapchain = swapchain;
        self.images = images;
        self.format = config.format;
        self.extent = config.extent;
        Ok(())
    }

    /// Acquires a new texture.
    ///
    /// # Errors
    ///
    /// Note that this function will always return an [`Error`] if [`recreate`] was and returned
    /// an error prior to calling thus function.
    ///
    /// [`recreate`]: Self::recreate
    pub fn acquire_next_image(
        &mut self,
        semaphore: &mut Semaphore,
    ) -> Result<SwapchainTexture<'_>, Error> {
        // If `recreate` was called an returned an error the swapchain
        // was invalidated and cannot be used.
        if self.swapchain.is_null() {
            return Err(Error::InvalidatedSwapchain);
        }

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

        Ok(SwapchainTexture {
            texture: Some(Texture {
                device: self.device.device.clone(),
                image: self.images[image_index as usize],
                format: self.format.format,
                size: self.extent,
                destroy_on_drop: false,
                mip_levels: 1,
            }),
            suboptimal,
            index: image_index,
            device: &self.device,
            swapchain: self,
        })
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        // The swapchain may already be dropped, in case `recreate`
        // failed.
        if self.swapchain.is_null() {
            return;
        }

        let device =
            ash::khr::swapchain::Device::new(&self.surface.instance.instance, &self.device.device);
        unsafe {
            device.destroy_swapchain(self.swapchain, None);
        }
    }
}

impl TryFrom<vk::PresentModeKHR> for PresentMode {
    type Error = UnknownEnumValue;

    fn try_from(value: vk::PresentModeKHR) -> Result<Self, Self::Error> {
        match value {
            vk::PresentModeKHR::FIFO => Ok(PresentMode::Fifo),
            vk::PresentModeKHR::IMMEDIATE => Ok(PresentMode::Immediate),
            vk::PresentModeKHR::FIFO_RELAXED => Ok(PresentMode::FifoRelaxed),
            vk::PresentModeKHR::MAILBOX => Ok(PresentMode::Mailbox),
            _ => Err(UnknownEnumValue),
        }
    }
}

impl From<PresentMode> for vk::PresentModeKHR {
    fn from(value: PresentMode) -> Self {
        match value {
            PresentMode::Fifo => vk::PresentModeKHR::FIFO,
            PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            PresentMode::FifoRelaxed => vk::PresentModeKHR::FIFO_RELAXED,
            PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
        }
    }
}

impl TryFrom<vk::Format> for TextureFormat {
    type Error = UnknownEnumValue;

    fn try_from(value: vk::Format) -> Result<Self, Self::Error> {
        match value {
            vk::Format::R8G8B8A8_UNORM => Ok(Self::Rgba8Unorm),
            vk::Format::R8G8B8A8_SRGB => Ok(Self::Rgba8UnormSrgb),
            vk::Format::B8G8R8A8_UNORM => Ok(Self::Bgra8Unorm),
            vk::Format::B8G8R8A8_SRGB => Ok(Self::Bgra8UnormSrgb),
            vk::Format::D32_SFLOAT => Ok(Self::Depth32Float),
            vk::Format::R16G16B16A16_SFLOAT => Ok(Self::Rgba16Float),
            _ => Err(UnknownEnumValue),
        }
    }
}

impl From<TextureFormat> for vk::Format {
    fn from(value: TextureFormat) -> Self {
        match value {
            TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
            TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
            TextureFormat::Bgra8Unorm => vk::Format::B8G8R8A8_UNORM,
            TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
            TextureFormat::Depth32Float => vk::Format::D32_SFLOAT,
            TextureFormat::Rgba16Float => vk::Format::R16G16B16A16_SFLOAT,
        }
    }
}

impl From<ColorSpace> for vk::ColorSpaceKHR {
    fn from(value: ColorSpace) -> Self {
        match value {
            ColorSpace::SrgbNonLinear => vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }
    }
}

impl TryFrom<vk::ColorSpaceKHR> for ColorSpace {
    type Error = UnknownEnumValue;

    fn try_from(value: vk::ColorSpaceKHR) -> Result<Self, Self::Error> {
        match value {
            vk::ColorSpaceKHR::SRGB_NONLINEAR => Ok(Self::SrgbNonLinear),
            _ => Err(UnknownEnumValue),
        }
    }
}

impl TryFrom<vk::PrimitiveTopology> for PrimitiveTopology {
    type Error = UnknownEnumValue;

    fn try_from(value: vk::PrimitiveTopology) -> Result<Self, Self::Error> {
        match value {
            vk::PrimitiveTopology::TRIANGLE_LIST => Ok(Self::TriangleList),
            _ => Err(UnknownEnumValue),
        }
    }
}

impl From<PrimitiveTopology> for vk::PrimitiveTopology {
    fn from(value: PrimitiveTopology) -> Self {
        match value {
            PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
            PrimitiveTopology::LineList => vk::PrimitiveTopology::LINE_LIST,
            PrimitiveTopology::PointList => vk::PrimitiveTopology::POINT_LIST,
            PrimitiveTopology::LineStrip => vk::PrimitiveTopology::LINE_STRIP,
            PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
        }
    }
}

impl From<FrontFace> for vk::FrontFace {
    fn from(value: FrontFace) -> Self {
        match value {
            FrontFace::Cw => vk::FrontFace::CLOCKWISE,
            FrontFace::Ccw => vk::FrontFace::COUNTER_CLOCKWISE,
        }
    }
}

impl From<DescriptorType> for vk::DescriptorType {
    fn from(value: DescriptorType) -> Self {
        match value {
            DescriptorType::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
            DescriptorType::Storage => vk::DescriptorType::STORAGE_BUFFER,
            DescriptorType::Sampler => vk::DescriptorType::SAMPLER,
            DescriptorType::Texture => vk::DescriptorType::SAMPLED_IMAGE,
        }
    }
}

impl From<ShaderStages> for vk::ShaderStageFlags {
    fn from(value: ShaderStages) -> Self {
        let mut flags = vk::ShaderStageFlags::empty();

        if value.contains(ShaderStages::VERTEX) {
            flags |= vk::ShaderStageFlags::VERTEX;
        }
        if value.contains(ShaderStages::FRAGMENT) {
            flags |= vk::ShaderStageFlags::FRAGMENT;
        }

        flags
    }
}

impl From<TextureUsage> for vk::ImageUsageFlags {
    fn from(value: TextureUsage) -> Self {
        let mut flags = vk::ImageUsageFlags::empty();
        if value.contains(TextureUsage::TRANSFER_SRC) {
            flags |= vk::ImageUsageFlags::TRANSFER_SRC;
        }
        if value.contains(TextureUsage::TRANSFER_DST) {
            flags |= vk::ImageUsageFlags::TRANSFER_DST;
        }
        if value.contains(TextureUsage::TEXTURE_BINDING) {
            flags |= vk::ImageUsageFlags::SAMPLED;
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

impl From<FilterMode> for vk::SamplerMipmapMode {
    fn from(value: FilterMode) -> Self {
        match value {
            FilterMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            FilterMode::Linear => vk::SamplerMipmapMode::LINEAR,
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

impl From<IndexFormat> for vk::IndexType {
    fn from(value: IndexFormat) -> Self {
        match value {
            IndexFormat::U16 => vk::IndexType::UINT16,
            IndexFormat::U32 => vk::IndexType::UINT32,
        }
    }
}

impl From<CompareOp> for vk::CompareOp {
    fn from(value: CompareOp) -> Self {
        match value {
            CompareOp::Never => Self::NEVER,
            CompareOp::Less => Self::LESS,
            CompareOp::LessEqual => Self::LESS_OR_EQUAL,
            CompareOp::Equal => Self::EQUAL,
            CompareOp::Greater => Self::GREATER,
            CompareOp::GreaterEqual => Self::GREATER_OR_EQUAL,
            CompareOp::Always => Self::ALWAYS,
            CompareOp::NotEqual => Self::NOT_EQUAL,
        }
    }
}

#[derive(Debug)]
pub struct ShaderModule {
    device: Arc<DeviceShared>,
    shader: vk::ShaderModule,
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.device.destroy_shader_module(self.shader, None);
        }
    }
}

#[derive(Debug)]
pub struct Pipeline {
    device: Arc<DeviceShared>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.device.destroy_pipeline(self.pipeline, None);
            self.device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

#[derive(Debug)]
pub struct CommandPool {
    device: Arc<DeviceShared>,
    pool: vk::CommandPool,
    buffers: Vec<vk::CommandBuffer>,
    /// Index of the next buffer.
    next_buffer: usize,
    /// Queue family allowed to submit buffers allocated from this pool.
    queue_family: QueueFamilyId,
    queue_caps: QueueCapabilities,
}

impl CommandPool {
    /// Acquires a new [`CommandEncoder`] from this `CommandPool`.
    pub fn create_encoder(&mut self) -> Result<CommandEncoder<'_>, Error> {
        let inheritance = vk::CommandBufferInheritanceInfo::default();

        let info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::empty())
            .inheritance_info(&inheritance);

        let Some(buffer) = self.buffers.get(self.next_buffer).copied() else {
            return Err(Error::CommandPoolExhausted);
        };

        // Move the buffer into the recording state.
        // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#vkBeginCommandBuffer
        // Safety:
        // - Buffer must not be in the initial state.
        // - Access to command buffer and command pool must be externally synchronized. (Asserted by
        // exclusive access.)
        unsafe {
            self.device.device.begin_command_buffer(buffer, &info)?;
        }

        // This should happen after `begin_command_buffer` was called.
        // In case it returns an error we should retry the same buffer
        // in the next call.
        self.next_buffer += 1;

        Ok(CommandEncoder {
            device: &self.device,
            buffer,
            queue_family: self.queue_family,
            queue_caps: self.queue_caps,
            _pool: PhantomData,
        })
    }

    /// Resets all command buffers in the pool.
    ///
    /// # Safety
    ///
    /// This operation invalidates all buffers created by [`create_encoder`]. All submissions using
    /// buffers from this `CommandPool` must have completed.
    pub unsafe fn reset(&mut self) -> Result<(), Error> {
        // Reset the pool and all buffers.
        // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#vkResetCommandPool
        // Safety:
        // - All buffers must NOT be in the pending state. (Guaranteed by caller.)
        // - Access to command pool must be externally synchronized. (Asserted by exclusive access.)
        unsafe {
            self.device
                .device
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())?
        }

        self.next_buffer = 0;
        Ok(())
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

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
    device: &'a DeviceShared,
    buffer: vk::CommandBuffer,
    /// Queue family which can be used to submit this buffer.
    queue_family: QueueFamilyId,
    queue_caps: QueueCapabilities,
    _pool: PhantomData<&'a CommandPool>,
}

impl<'a> CommandEncoder<'a> {
    /// Copy `count` bytes from `src` to `dst`.
    ///
    /// # Safety
    ///
    /// - `src` must have the [`TRANSFER_READ`] flag set at the time of operation.
    /// - `dst` must have the [`TRANSFER_WRITE`] flag set at the time of operation.
    /// - Only one operation must write to `dst` before the write is flushed.
    ///
    /// [`TRANSFER_READ`]: AccessFlags::TRANSFER_READ
    /// [`TRANSFER_WRITE`]: AccessFlags::TRANSFER_WRITE
    pub unsafe fn copy_buffer_to_buffer(
        &mut self,
        src: &Buffer,
        src_offset: u64,
        dst: &Buffer,
        dst_offset: u64,
        count: u64,
    ) {
        assert!(self.queue_caps.contains(QueueCapabilities::TRANSFER));

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

    /// Copies data from a [`Buffer`] into a [`Texture`].
    ///
    /// # Safety
    ///
    /// - The `src` [`Buffer`] must have the [`TRANSFER_READ`] flag set at the time of operation.
    /// - The target mip-level of the `dst` [`Texture`] must have the [`TRANSFER_WRITE`] flag set
    /// at the time of operation.
    /// - Only one operation must write to `dst` before the write is flushed.
    ///
    /// [`TRANSFER_READ`]: AccessFlags::TRANSFER_READ
    /// [`TRANSFER_WRITE`]: AccessFlags::TRANSFER_WRITE
    pub unsafe fn copy_buffer_to_texture(
        &mut self,
        src: CopyBuffer<'_>,
        dst: &Texture,
        mip_level: u32,
    ) {
        assert!(self.queue_caps.contains(QueueCapabilities::TRANSFER));

        assert_ne!(dst.size.x, 0);
        assert_ne!(dst.size.y, 0);

        let bytes_to_copy = src.layout.bytes_per_row as u64 * src.layout.rows_per_image as u64;
        assert!(src.buffer.size > src.offset);
        assert!(src.buffer.size - src.offset >= bytes_to_copy);

        assert!(mip_level < dst.mip_levels);

        let aspect_mask = if dst.format.is_depth() {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(aspect_mask)
            .mip_level(mip_level)
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

    /// Begins the recording of a [`RenderPass`].
    ///
    /// # Safety
    ///
    /// - All color attachments must have the [`COLOR_ATTACHMENT_READ`] flag set at the time of
    /// operation if they are being read from a shader.
    /// - All color attachments must have the [`COLOR_ATTACHMENT_WRITE`] flag set at the time of
    /// operation if they are being written to from a shader.
    /// - The depth attachment must have the [`DEPTH_ATTACHMENT_READ`] flag set at the time of
    /// operation if they are being read from.
    /// - The depth attachment must have the [`DEPTH_ATTACHMENT_WRITE`] flag set at the time of
    /// operation if they are being written to.
    /// - Every color/depth attachment that is being written to must not have any writes that
    /// are not yet flushed.
    ///
    /// [`COLOR_ATTACHMENT_READ`]: AccessFlags::COLOR_ATTACHMENT_READ
    /// [`COLOR_ATTACHMENT_WRITE`]: AccessFlags::COLOR_ATTACHMENT_WRITE
    /// [`DEPTH_ATTACHMENT_READ`]: AccessFlags::DEPTH_ATTACHMENT_READ
    /// [`DEPTH_ATTACHMENT_WRITE`]: AccessFlags::DEPTH_ATTACHMENT_WRITE
    pub unsafe fn begin_render_pass<'res>(
        &mut self,
        descriptor: &RenderPassDescriptor<'_, 'res>,
    ) -> RenderPass<'_, 'res> {
        assert!(self.queue_caps.contains(QueueCapabilities::GRAPHICS));

        let mut extent = UVec2::ZERO;

        let mut color_attachments = Vec::new();
        for attachment in descriptor.color_attachments {
            let load_op = match attachment.load_op {
                LoadOp::Load => vk::AttachmentLoadOp::LOAD,
                LoadOp::Clear(_) => vk::AttachmentLoadOp::CLEAR,
            };

            let store_op = match attachment.store_op {
                StoreOp::Discard => vk::AttachmentStoreOp::NONE,
                StoreOp::Store => vk::AttachmentStoreOp::STORE,
            };

            let clear_value = match attachment.load_op {
                LoadOp::Clear(color) => vk::ClearValue {
                    color: vk::ClearColorValue { float32: color.0 },
                },
                LoadOp::Load => vk::ClearValue::default(),
            };

            let layout = access_flags_to_image_layout(attachment.access);
            let info = vk::RenderingAttachmentInfo::default()
                .image_view(attachment.view.view)
                .image_layout(layout)
                .resolve_mode(vk::ResolveModeFlags::NONE)
                .load_op(load_op)
                .store_op(store_op)
                .clear_value(clear_value);

            color_attachments.push(info);
            extent = UVec2::max(extent, attachment.view.size);
        }

        let depth_attachment = descriptor.depth_stencil_attachment.map(|attachment| {
            let (load_op, clear_value) = match attachment.depth_load_op {
                LoadOp::Clear(value) => (
                    vk::AttachmentLoadOp::CLEAR,
                    vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: value,
                            stencil: 0,
                        },
                    },
                ),
                LoadOp::Load => (vk::AttachmentLoadOp::LOAD, vk::ClearValue::default()),
            };

            let store_op = match attachment.depth_store_op {
                StoreOp::Discard => vk::AttachmentStoreOp::NONE,
                StoreOp::Store => vk::AttachmentStoreOp::STORE,
            };

            let layout = access_flags_to_image_layout(attachment.access);
            extent = UVec2::max(extent, attachment.view.size);
            vk::RenderingAttachmentInfo::default()
                .image_view(attachment.view.view)
                .image_layout(layout)
                .resolve_mode(vk::ResolveModeFlags::NONE)
                .load_op(load_op)
                .store_op(store_op)
                .clear_value(clear_value)
        });

        assert_ne!(extent.x, 0);
        assert_ne!(extent.y, 0);

        let mut info = vk::RenderingInfo::default()
            .flags(vk::RenderingFlags::empty())
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: extent.x,
                    height: extent.y,
                },
            })
            .layer_count(1)
            .view_mask(0)
            .color_attachments(&color_attachments);

        if let Some(attachment) = &depth_attachment {
            info = info.depth_attachment(attachment);
        }

        unsafe {
            self.device.device.cmd_begin_rendering(self.buffer, &info);
        }

        // Since we have created the pipeline with `VK_DYNAMIC_STATE_VIEWPORT` and
        // `VK_DYNAMIC_STATE_SCISSOR` we must set the the viewport and scissors
        // before any draw opertaions.
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.x as f32,
            height: extent.y as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
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

    /// Inserts a batch of memory/execution barriers.
    pub fn insert_pipeline_barriers(&mut self, barriers: &PipelineBarriers<'_>) {
        // FIXME: This function should probably require the caller to
        // guarantee that the resource referenced by the barrier is
        // in the "correct" AccessFlags state (esp. for textures).

        let mut buffer_barriers = Vec::new();
        for barrier in barriers.buffer {
            assert!(barrier.src_access.is_allowed_for_queue(&self.queue_caps));
            assert!(barrier.dst_access.is_allowed_for_queue(&self.queue_caps));

            let src_access_flags = convert_access_flags(barrier.src_access);
            let dst_access_flags = convert_access_flags(barrier.dst_access);
            let src_stage_mask = access_flags_to_stage_mask(barrier.src_access);
            let dst_stage_mask = access_flags_to_stage_mask(barrier.dst_access);

            // - `offset` must be less than the size of `buffer`.
            // - `size` must not be 0.
            // - `size` must be less than or equal to the size of `buffer` minus `offset`.
            assert_ne!(barrier.size, 0);
            assert!(barrier.offset < barrier.buffer.size);
            assert!(barrier.size <= barrier.buffer.size - barrier.offset);

            let barrier = vk::BufferMemoryBarrier2::default()
                .buffer(barrier.buffer.buffer)
                .offset(barrier.offset)
                .size(barrier.size)
                .src_access_mask(src_access_flags)
                .dst_access_mask(dst_access_flags)
                .src_stage_mask(src_stage_mask)
                .dst_stage_mask(dst_stage_mask)
                // Do not transfer between queues.
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED);
            buffer_barriers.push(barrier);
        }

        let mut image_barriers = Vec::new();
        for barrier in barriers.texture {
            assert!(barrier.src_access.is_allowed_for_queue(&self.queue_caps));
            assert!(barrier.dst_access.is_allowed_for_queue(&self.queue_caps));

            let src_access_flags = convert_access_flags(barrier.src_access);
            let dst_access_flags = convert_access_flags(barrier.dst_access);
            let src_stage_mask = access_flags_to_stage_mask(barrier.src_access);
            let dst_stage_mask = access_flags_to_stage_mask(barrier.dst_access);
            let old_layout = access_flags_to_image_layout(barrier.src_access);
            let new_layout = access_flags_to_image_layout(barrier.dst_access);

            let aspect_mask = if barrier.texture.format.is_depth() {
                vk::ImageAspectFlags::DEPTH
            } else {
                vk::ImageAspectFlags::COLOR
            };

            // Images cannot be transitioned into `UNDEFINED`.
            assert_ne!(new_layout, vk::ImageLayout::UNDEFINED);

            assert!(barrier.base_mip_level < barrier.texture.mip_levels);
            assert!(barrier.base_mip_level + barrier.mip_levels <= barrier.texture.mip_levels);

            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(aspect_mask)
                .base_mip_level(barrier.base_mip_level)
                .level_count(barrier.mip_levels)
                .base_array_layer(0)
                .layer_count(1);

            let barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(src_stage_mask)
                .dst_stage_mask(dst_stage_mask)
                .src_access_mask(src_access_flags)
                .dst_access_mask(dst_access_flags)
                .old_layout(old_layout.into())
                .new_layout(new_layout.into())
                // Do not transfer between queues.
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(barrier.texture.image)
                .subresource_range(subresource_range);
            image_barriers.push(barrier);
        }

        let info = vk::DependencyInfo::default()
            .dependency_flags(vk::DependencyFlags::empty())
            .buffer_memory_barriers(&buffer_barriers)
            .image_memory_barriers(&image_barriers);

        unsafe {
            self.device.device.cmd_pipeline_barrier2(self.buffer, &info);
        }
    }

    pub fn finish(self) -> Result<CommandBuffer<'a>, Error> {
        unsafe {
            self.device.device.end_command_buffer(self.buffer)?;
        }

        Ok(CommandBuffer {
            buffer: self.buffer,
            queue_family: self.queue_family,
            _device: PhantomData,
        })
    }
}

pub struct RenderPass<'encoder, 'resources> {
    encoder: &'encoder CommandEncoder<'encoder>,
    // Marker to indicate that all resources that this render pass
    // may access must not be dropped while this render pass exists.
    _marker: PhantomData<fn() -> &'resources ()>,
    pipeline: Option<&'resources Pipeline>,
}

impl<'encoder, 'resources> RenderPass<'encoder, 'resources> {
    pub fn bind_pipeline(&mut self, pipeline: &'resources Pipeline) {
        // Bind the pipeline.
        // https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdBindPipeline.html
        // Safety:
        // - Since we are using `GRAPHICS`, the pipeline must be a graphics pipeline.
        // - Since we are using `GRAPHICS`, the command buffer must support graphics
        // operations.
        unsafe {
            self.encoder.device.device.cmd_bind_pipeline(
                self.encoder.buffer,
                vk::PipelineBindPoint::GRAPHICS,
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
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout,
                slot,
                &[descriptor_set.set],
                &[],
            );
        }
    }

    pub fn bind_index_buffer(&mut self, buffer: BufferView<'_>, format: IndexFormat) {
        assert!(buffer
            .buffer
            .usages
            .contains(vk::BufferUsageFlags::INDEX_BUFFER));

        assert_eq!(buffer.len() % u64::from(format.size()), 0);

        unsafe {
            self.encoder.device.device.cmd_bind_index_buffer(
                self.encoder.buffer,
                buffer.buffer.buffer,
                buffer.view.start,
                format.into(),
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

    pub fn draw_indexed(&mut self, indices: Range<u32>, vertex_offset: i32, instances: Range<u32>) {
        unsafe {
            self.encoder.device.device.cmd_draw_indexed(
                self.encoder.buffer,
                indices.len() as u32,
                instances.len() as u32,
                indices.start,
                vertex_offset,
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

#[derive(Debug)]
pub struct CommandBuffer<'a> {
    buffer: vk::CommandBuffer,
    /// Queue family which can be used to submit this buffer.
    queue_family: QueueFamilyId,
    _device: PhantomData<&'a DeviceShared>,
}

#[derive(Debug)]
pub struct Semaphore {
    device: Arc<DeviceShared>,
    semaphore: vk::Semaphore,
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

pub struct SwapchainTexture<'a> {
    pub texture: Option<Texture>,
    suboptimal: bool,
    index: u32,
    device: &'a Device,
    swapchain: &'a Swapchain,
}

impl<'a> SwapchainTexture<'a> {
    pub fn texture(&self) -> &Texture {
        self.texture.as_ref().unwrap()
    }

    pub unsafe fn take_texture(&mut self) -> Texture {
        assert!(!self.texture().destroy_on_drop);
        self.texture.take().unwrap()
    }

    /// Returns `true` if the [`Swapchain`] used to acquire this texture is suboptimal, i.e. could
    /// benefit from recreation even if no properties changed.
    pub fn is_suboptimal(&self) -> bool {
        self.suboptimal
    }

    /// Schedules this tecture for presentation on the swapchain.
    ///
    /// The operation will happen on the given [`Queue`] when the given [`Semaphore`] is signaled.
    ///
    /// # Safety
    ///
    /// When the operation occurs the texture must have the [`PRESENT`] set.
    ///
    /// [`PRESENT`]: AccessFlags::PRESENT
    pub unsafe fn present(
        &mut self,
        queue: &mut Queue,
        wait_semaphore: &mut Semaphore,
    ) -> Result<(), Error> {
        let device =
            ash::khr::swapchain::Device::new(&self.device.device.instance, &self.device.device);

        let wait_semaphores = &[wait_semaphore.semaphore];

        let swapchains = &[self.swapchain.swapchain];
        let image_indices = &[self.index];
        let info = vk::PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            // - Every element must be unique.
            .swapchains(swapchains)
            // - Every image must be in `VK_IMAGE_LAYOUT_PRESENT_SRC_KHR` once this is executed.
            .image_indices(image_indices);

        // Safety:
        // - `queue` must be externally synchronized.
        // - `semaphore` must be externally synchronized.
        // - `swapchain` must be externally synchronized.
        unsafe {
            device.queue_present(queue.queue, &info)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Texture {
    device: Arc<DeviceShared>,
    image: vk::Image,
    format: TextureFormat,
    size: UVec2,
    mip_levels: u32,
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

    pub fn mip_levels(&self) -> u32 {
        self.mip_levels
    }

    pub fn create_view<'a>(&'a self, descriptor: &TextureViewDescriptor) -> TextureView<'a> {
        assert!(descriptor.base_mip_level + descriptor.mip_levels <= self.mip_levels);

        let components = vk::ComponentMapping::default()
            .r(vk::ComponentSwizzle::IDENTITY)
            .g(vk::ComponentSwizzle::IDENTITY)
            .b(vk::ComponentSwizzle::IDENTITY)
            .a(vk::ComponentSwizzle::IDENTITY);

        let aspect_mask = if self.format.is_depth() {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(aspect_mask)
            .base_mip_level(descriptor.base_mip_level)
            .level_count(descriptor.mip_levels)
            .base_array_layer(0)
            .layer_count(1);

        let info = vk::ImageViewCreateInfo::default()
            .image(self.image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(self.format.into())
            .subresource_range(subresource_range)
            .components(components);

        let view = unsafe { self.device.device.create_image_view(&info, None).unwrap() };
        TextureView {
            device: self.device.clone(),
            view,
            size: mip_level_size_2d(self.size, descriptor.base_mip_level),
            parent: PhantomData,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        if self.destroy_on_drop {
            unsafe {
                self.device.device.destroy_image(self.image, None);
            }
        }
    }
}

#[derive(Debug)]
pub struct TextureView<'a> {
    device: Arc<DeviceShared>,
    view: vk::ImageView,
    size: UVec2,
    parent: PhantomData<&'a ()>,
}

impl<'a> TextureView<'a> {
    pub unsafe fn make_static(self) -> TextureView<'static> {
        // Since this only changes PhantomData<&'a ()> to PhantomData<&'static ()>,
        // this could be made safe if we could destructure self without calling drop
        // first. (https://github.com/rust-lang/rfcs/pull/3466)
        // SAFETY: We only transmute change the lifetime of a `PhantomData`, which
        // is safe.
        unsafe { core::mem::transmute(self) }
    }
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
    size: u64,
    usages: vk::BufferUsageFlags,
}

impl Buffer {
    pub fn slice<R>(&self, range: R) -> BufferView<'_>
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

        BufferView {
            buffer: self,
            view: start..end,
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

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
    flags: MemoryTypeFlags,
    memory_type: u32,
    is_mapped: bool,
}

impl DeviceMemory {
    pub fn slice<R>(&self, range: R) -> DeviceMemorySlice<'_>
    where
        R: RangeBounds<u64>,
    {
        let (offset, size) = range.into_offset_size(self.size.get());

        assert!(self.size.get() > offset);
        assert!(self.size.get() - offset >= size);

        DeviceMemorySlice {
            memory: self,
            offset,
            size,
        }
    }

    /// Maps the entire `DeviceMemory` into host memory.
    pub fn map(&mut self) -> Result<NonNull<u8>, Error> {
        assert!(!self.is_mapped);
        assert!(self.flags.contains(MemoryTypeFlags::HOST_VISIBLE));

        let ptr = unsafe {
            self.device
                .map_memory(self.memory, 0, self.size.get(), vk::MemoryMapFlags::empty())?
        };

        self.is_mapped = true;

        unsafe { Ok(NonNull::new_unchecked(ptr.cast::<u8>())) }
    }

    /// Unmaps the `DeviceMemory` that was previously mapped.
    ///
    /// Note that this invalidates the pointer returned by [`map`].
    ///
    /// [`map`]: Self::map
    pub fn ummap(&mut self) {
        assert!(self.is_mapped);
        self.is_mapped = false;

        // Safety:
        // - `memory` must currently be host mapped.
        unsafe {
            self.device.unmap_memory(self.memory);
        }
    }

    /// Flushes the `DeviceMemory`.
    ///
    /// Note that the pointer returned by [`map`] must not have any active reference when this
    /// function is called.
    ///
    /// [`map`]: Self::map
    pub fn flush(&self) -> Result<(), Error> {
        assert!(self.is_mapped);

        // Emit an SFENCE instruction.
        // Ensure that all prior stores are made visible.
        // This step is explicitly required by the Vulkan spec and not
        // handled by the driver.
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::x86_64::_mm_sfence();
        }

        let range = vk::MappedMemoryRange::default()
            .memory(self.memory)
            .offset(0)
            .size(vk::WHOLE_SIZE);

        unsafe {
            self.device.flush_mapped_memory_ranges(&[range])?;
        }
        Ok(())
    }
}

impl Drop for DeviceMemory {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.free_memory(self.memory, None);
        }

        self.device.num_allocations.fetch_sub(1, Ordering::Release);
    }
}

pub struct DeviceMemorySlice<'a> {
    memory: &'a DeviceMemory,
    offset: u64,
    size: u64,
}

impl<'a> DeviceMemorySlice<'a> {}

#[derive(Debug)]
pub struct DescriptorSetLayout {
    device: Arc<DeviceShared>,
    layout: vk::DescriptorSetLayout,
    bindings: Vec<super::DescriptorBinding>,
}

impl DescriptorSetLayout {
    pub(crate) fn bindings(&self) -> &[super::DescriptorBinding] {
        &self.bindings
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

#[derive(Debug)]
pub struct DescriptorPool {
    device: Arc<DeviceShared>,
    pool: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn create_descriptor_set(
        &mut self,
        layout: &DescriptorSetLayout,
    ) -> Result<DescriptorSet<'_>, Error> {
        let layouts = [layout.layout];

        let info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            // - `descriptorSetCount` must be greater than 0.
            .set_layouts(&layouts);

        let sets = unsafe { self.device.allocate_descriptor_sets(&info)? };
        Ok(DescriptorSet {
            pool: self,
            set: sets[0],
            bindings: layout.bindings.clone(),
        })
    }

    pub unsafe fn reset(&mut self) {
        unsafe {
            // - `flags` must be 0.
            self.device
                .reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::empty())
                .unwrap();
        }
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.pool, None);
        }
    }
}

#[derive(Debug)]
pub struct DescriptorSet<'a> {
    pool: &'a DescriptorPool,
    set: vk::DescriptorSet,
    bindings: Vec<super::DescriptorBinding>,
}

impl<'a> DescriptorSet<'a> {
    /// Updates the resources in this `DescriptorSet`.
    ///
    /// # Safety
    ///
    /// - This `DescriptorSet` must not currently be in use by any submitted command, i.e. it must
    /// not be in any command buffer that is in the pending state.
    pub unsafe fn update(&mut self, op: &WriteDescriptorResources<'_>) {
        #[derive(Copy, Clone, Debug)]
        struct Header {
            binding: u32,
            kind: DescriptorType,
            count: usize,
        }

        // This union must be `#[repr(C)]`, so we can pass a
        // pointer to it to Vulkan.
        #[derive(Copy, Clone)]
        #[repr(C)]
        union Info {
            header: Header,
            buffer: vk::DescriptorBufferInfo,
            image: vk::DescriptorImageInfo,
        }

        // We include all necessary data in one array.
        // Every descriptor begins with a `Header`, which describes
        // which `DescriptorType` follows in the next `count` elements.
        // Start with an initial capacity of `bindings * 2` which is the
        // minimum needed (1 for header, 1 for info).
        let mut infos = Vec::with_capacity(op.bindings.len() * 2);

        for (index, binding) in op.bindings.iter().enumerate() {
            let Some(layout_binding) = self.bindings.get(index) else {
                panic!(
                    "attempted to write to index {} of descriptor set with layout of {} elements",
                    index,
                    self.bindings.len()
                );
            };

            let (kind, count) = match binding.resource {
                WriteDescriptorResource::UniformBuffer(buffers) => {
                    (DescriptorType::Uniform, buffers.len())
                }
                WriteDescriptorResource::StorageBuffer(buffers) => {
                    (DescriptorType::Storage, buffers.len())
                }
                WriteDescriptorResource::Texture(textures) => {
                    (DescriptorType::Texture, textures.len())
                }
                WriteDescriptorResource::Sampler(samplers) => {
                    (DescriptorType::Sampler, samplers.len())
                }
            };

            assert_ne!(count, 0);
            assert!(count <= layout_binding.count.get() as usize);

            assert_eq!(
                layout_binding.kind, kind,
                "type missmatch at index {}: op = {:?}, layout = {:?}",
                index, kind, layout_binding.kind,
            );

            infos.push(Info {
                header: Header {
                    binding: binding.binding,
                    kind,
                    count,
                },
            });

            match binding.resource {
                WriteDescriptorResource::UniformBuffer(buffers)
                | WriteDescriptorResource::StorageBuffer(buffers) => {
                    for buffer in buffers {
                        let info = vk::DescriptorBufferInfo::default()
                            .buffer(buffer.buffer().buffer)
                            .offset(buffer.offset())
                            .range(buffer.len());

                        infos.push(Info { buffer: info });
                    }
                }
                WriteDescriptorResource::Texture(textures) => {
                    for texture in textures {
                        let info = vk::DescriptorImageInfo::default()
                            .image_view(texture.view)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                            .sampler(vk::Sampler::null());

                        infos.push(Info { image: info });
                    }
                }
                WriteDescriptorResource::Sampler(samplers) => {
                    for sampler in samplers {
                        let info = vk::DescriptorImageInfo::default()
                            .sampler(sampler.sampler)
                            .image_view(vk::ImageView::null());

                        infos.push(Info { image: info });
                    }
                }
            }
        }

        let mut writes = Vec::with_capacity(op.bindings.len());

        let mut index = 0;
        while index < infos.len() {
            let header = unsafe { infos[index].header };
            // Skip over header.
            index += 1;

            let descriptor_type = match header.kind {
                DescriptorType::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
                DescriptorType::Storage => vk::DescriptorType::STORAGE_BUFFER,
                DescriptorType::Texture => vk::DescriptorType::SAMPLED_IMAGE,
                DescriptorType::Sampler => vk::DescriptorType::SAMPLER,
            };

            let mut write = vk::WriteDescriptorSet::default()
                .dst_set(self.set)
                .dst_binding(header.binding)
                .dst_array_element(0)
                // - `descriptorCount` must be greater than 0.
                .descriptor_count(header.count as u32)
                .descriptor_type(descriptor_type);

            // Depending on the `descriptor_type` either `p_buffer_infos` or
            // `p_image_infos` is used, the other one is ignored.
            let ptr = infos[index..].as_ptr();
            write.p_buffer_info = ptr.cast::<vk::DescriptorBufferInfo>();
            write.p_image_info = ptr.cast::<vk::DescriptorImageInfo>();

            // Jump over all infos of this descriptor and the header.
            index += header.count;
            writes.push(write);
        }

        unsafe {
            self.pool.device.update_descriptor_sets(&writes, &[]);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum FenceState {
    /// Fence is not used.
    Idle,
    /// Fence has been registered, but it has not been signaled yet.
    Waiting,
}

#[derive(Debug)]
pub struct Fence {
    device: Arc<DeviceShared>,
    fence: vk::Fence,
    state: FenceState,
}

impl Fence {
    /// Waits for this fence to become signaled once.
    ///
    /// Returns `true` if the fence is signaled within the given `timeout`. Returns `false` if it
    /// is not signaled within the `timeout`.
    ///
    /// `None` represents an infinite timeout. In this case this function will never return `false`.
    ///
    /// Note that the timeout may be longer as requested.
    pub fn wait(&mut self, timeout: Option<Duration>) -> Result<bool, Error> {
        assert_eq!(self.state, FenceState::Waiting);

        let mut timeout = timeout.map(|timeout| timeout.as_nanos());

        loop {
            let step = match timeout {
                Some(ns) => u64::try_from(ns).unwrap_or(u64::MAX),
                None => u64::MAX,
            };

            // SAFETY:
            // - Fence count must be greater than 0.
            let res = unsafe { self.device.wait_for_fences(&[self.fence], true, step) };
            match res {
                Ok(()) => break,
                Err(err) if err != vk::Result::TIMEOUT => return Err(err.into()),
                Err(_) => (),
            }

            debug_assert_eq!(res.unwrap_err(), vk::Result::TIMEOUT);

            if let Some(timeout) = &mut timeout {
                *timeout -= u128::from(step);

                if *timeout == 0 {
                    return Ok(false);
                }
            }
        }

        self.reset();
        Ok(true)
    }

    fn reset(&mut self) {
        self.state = FenceState::Idle;
        unsafe {
            self.device.reset_fences(&[self.fence]).unwrap();
        }
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

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
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.destroy_sampler(self.sampler, None);
        }
    }
}

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    typ: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = match unsafe { data.message_as_c_str() } {
        Some(msg) => msg.to_string_lossy(),
        None => Cow::Borrowed("(no message)"),
    };

    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            let backtrace = Backtrace::force_capture();
            tracing::error!("[{:?}]: {}\n{}", typ, message, backtrace);
            panic!("abort due to prior validation error");
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            tracing::warn!("[{:?}]: {}", typ, message);
        }
        _ => (),
    }

    // The application should always return `VK_FALSE`.
    vk::FALSE
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnknownEnumValue;

#[derive(Clone)]
struct InstanceShared {
    config: Config,
    entry: Entry,
    instance: ash::Instance,
    messenger: Option<vk::DebugUtilsMessengerEXT>,
}

impl InstanceShared {
    fn same(self: &Arc<Self>, other: &Arc<Self>) -> bool {
        Arc::ptr_eq(&self, other)
    }
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
        if thread::panicking() {
            return;
        }

        if let Some(messenger) = self.messenger.take() {
            unsafe {
                let instance = debug_utils::Instance::new(&self.entry, &self.instance);
                instance.destroy_debug_utils_messenger(messenger, None);
            }
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
    limits: DeviceLimits,
    memory_properties: AdapterMemoryProperties,
    /// Number of currently active allocations.
    num_allocations: Arc<AtomicU32>,
    queues: Arc<[QueueSlot]>,
}

impl DeviceShared {
    /// Returns `true` if both `DeviceShared` instances refer to the same Vulkan device.
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.instance, &other.instance)
            && self.device.handle() == other.device.handle()
    }
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
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.destroy_device(None);
        }
    }
}

#[derive(Debug)]
struct QueueSlot {
    id: QueueFamilyId,
    index: u32,
    caps: QueueCapabilities,
    used: Mutex<bool>,
}

#[derive(Copy, Clone, Debug)]
struct DeviceLimits {
    max_push_constants_size: u32,
    max_bound_descriptor_sets: u32,
    max_memory_allocation_count: u32,
    /// Is always a power of two.
    buffer_image_granularity: u64,
    max_per_stage_descriptor_samplers: u32,
    max_per_stage_descriptor_uniform_buffers: u32,
    max_per_stage_descriptor_storage_buffers: u32,
    max_per_stage_descriptor_sampled_images: u32,
    max_per_stage_resources: u32,
    max_descriptor_set_samplers: u32,
    max_descriptor_set_uniform_buffers: u32,
    max_descriptor_set_storage_buffers: u32,
    max_descriptor_set_sampled_images: u32,
    max_color_attachments: u32,
}

trait RangeBoundsExt {
    fn into_offset_size(self, upper_bound: u64) -> (u64, u64);
}

impl<T> RangeBoundsExt for T
where
    T: RangeBounds<u64>,
{
    fn into_offset_size(self, upper_bound: u64) -> (u64, u64) {
        let start = match self.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start + 1,
            Bound::Unbounded => 0,
        };

        let end = match self.end_bound() {
            Bound::Included(end) => *end + 1,
            Bound::Excluded(end) => *end,
            Bound::Unbounded => upper_bound,
        };

        (start, end - start)
    }
}

fn convert_access_flags(flags: AccessFlags) -> vk::AccessFlags2 {
    let mut access = vk::AccessFlags2::empty();

    for flag in flags.iter() {
        let vk_flag = match flag {
            AccessFlags::TRANSFER_READ => vk::AccessFlags2::TRANSFER_READ,
            AccessFlags::TRANSFER_WRITE => vk::AccessFlags2::TRANSFER_WRITE,
            AccessFlags::COLOR_ATTACHMENT_READ => vk::AccessFlags2::COLOR_ATTACHMENT_READ,
            AccessFlags::COLOR_ATTACHMENT_WRITE => vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            AccessFlags::INDEX => vk::AccessFlags2::INDEX_READ,
            AccessFlags::INDIRECT => vk::AccessFlags2::INDIRECT_COMMAND_READ,
            AccessFlags::DEPTH_ATTACHMENT_READ => vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ,
            AccessFlags::DEPTH_ATTACHMENT_WRITE => vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
            AccessFlags::VERTEX_SHADER_READ => vk::AccessFlags2::SHADER_READ,
            AccessFlags::VERTEX_SHADER_WRITE => vk::AccessFlags2::SHADER_WRITE,
            AccessFlags::FRAGMENT_SHADER_READ => vk::AccessFlags2::SHADER_READ,
            AccessFlags::FRAGMENT_SHADER_WRITE => vk::AccessFlags2::SHADER_WRITE,
            AccessFlags::PRESENT => continue,
            _ => unreachable!(),
        };

        access |= vk_flag;
    }

    access
}

fn access_flags_to_image_layout(flags: AccessFlags) -> vk::ImageLayout {
    let mut transfer_read = false;
    let mut transfer_write = false;
    let mut color_attachment_read = false;
    let mut color_attachment_write = false;
    let mut depth_attachment_read = false;
    let mut depth_attachment_write = false;
    let mut shader_read = false;
    let mut shader_write = false;
    let mut present = false;

    for flag in flags.iter() {
        match flag {
            AccessFlags::TRANSFER_READ => transfer_read = true,
            AccessFlags::TRANSFER_WRITE => transfer_write = true,
            AccessFlags::COLOR_ATTACHMENT_READ => color_attachment_read = true,
            AccessFlags::COLOR_ATTACHMENT_WRITE => color_attachment_write = true,
            AccessFlags::DEPTH_ATTACHMENT_READ => depth_attachment_read = true,
            AccessFlags::DEPTH_ATTACHMENT_WRITE => depth_attachment_write = true,
            AccessFlags::VERTEX_SHADER_READ => shader_read = true,
            AccessFlags::VERTEX_SHADER_WRITE => shader_write = true,
            AccessFlags::FRAGMENT_SHADER_READ => shader_read = true,
            AccessFlags::FRAGMENT_SHADER_WRITE => shader_write = true,
            AccessFlags::PRESENT => present = true,
            AccessFlags::INDEX => {
                unreachable!("{:?} has no image layout", AccessFlags::INDEX)
            }
            AccessFlags::INDIRECT => {
                unreachable!("{:?} has no image layout", AccessFlags::INDIRECT)
            }
            _ => unreachable!("unhandled access flag: {:?}", flag),
        }
    }

    match (
        transfer_read,
        transfer_write,
        color_attachment_read,
        color_attachment_write,
        depth_attachment_read,
        depth_attachment_write,
        shader_read,
        shader_write,
        present,
    ) {
        (false, false, false, false, false, false, false, false, false) => {
            vk::ImageLayout::UNDEFINED
        }
        (true, false, false, false, false, false, false, false, false) => {
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL
        }
        (false, true, false, false, false, false, false, false, false) => {
            vk::ImageLayout::TRANSFER_DST_OPTIMAL
        }
        (false, false, true, _, false, false, false, false, false)
        | (false, false, _, true, false, false, false, false, false) => {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        }
        (false, false, false, false, true, false, false, false, false) => {
            vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL
        }
        (false, false, false, _, true, true, false, false, false) => {
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL
        }
        (false, false, false, false, false, false, true, false, false) => {
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        }
        (false, false, false, false, false, false, false, false, true) => {
            vk::ImageLayout::PRESENT_SRC_KHR
        }
        (_, _, _, _, _, _, _, _, true) => {
            panic!(
                "{:?} is mutually exclusive with all other flags",
                AccessFlags::PRESENT
            );
        }
        _ => vk::ImageLayout::GENERAL,
    }
}

fn access_flags_to_stage_mask(flags: AccessFlags) -> vk::PipelineStageFlags2 {
    // See https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#synchronization-pipeline-stages-order
    // for ordered list of pipeline stages.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum GraphicsStage {
        DrawIndirect,
        VertexInput,
        VertexShader,
        EarlyFragmentTests,
        FragmentShader,
        //LateFragmentTests,
        ColorAttachmentOutput,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum TransferStage {
        Transfer,
    }

    let mut transfer = None;
    if flags.contains(AccessFlags::TRANSFER_READ) | flags.contains(AccessFlags::TRANSFER_WRITE) {
        transfer = Some(TransferStage::Transfer);
    }

    // See https://registry.khronos.org/vulkan/specs/latest/man/html/VkAccessFlagBits2.html
    // for which accesses map to which pipeline stages.
    let mut graphics = None;
    for (flag, stage) in [
        (AccessFlags::INDIRECT, GraphicsStage::DrawIndirect),
        (AccessFlags::INDEX, GraphicsStage::VertexInput),
        (AccessFlags::VERTEX_SHADER_READ, GraphicsStage::VertexShader),
        (
            AccessFlags::VERTEX_SHADER_WRITE,
            GraphicsStage::VertexShader,
        ),
        (
            AccessFlags::FRAGMENT_SHADER_READ,
            GraphicsStage::FragmentShader,
        ),
        (
            AccessFlags::FRAGMENT_SHADER_WRITE,
            GraphicsStage::FragmentShader,
        ),
        (
            AccessFlags::DEPTH_ATTACHMENT_READ,
            GraphicsStage::EarlyFragmentTests,
        ),
        (
            AccessFlags::DEPTH_ATTACHMENT_WRITE,
            GraphicsStage::EarlyFragmentTests,
        ),
        (
            AccessFlags::COLOR_ATTACHMENT_READ,
            GraphicsStage::FragmentShader,
        ),
        (
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            GraphicsStage::ColorAttachmentOutput,
        ),
    ] {
        if !flags.contains(flag) {
            continue;
        }

        match &mut graphics {
            Some(earliest_stage) => {
                *earliest_stage = core::cmp::min(*earliest_stage, stage);
            }
            None => graphics = Some(stage),
        }
    }

    let transfer = match transfer {
        Some(TransferStage::Transfer) => vk::PipelineStageFlags2::TRANSFER,
        None => vk::PipelineStageFlags2::empty(),
    };

    let graphics = match graphics {
        Some(GraphicsStage::DrawIndirect) => vk::PipelineStageFlags2::DRAW_INDIRECT,
        Some(GraphicsStage::VertexInput) => vk::PipelineStageFlags2::VERTEX_INPUT,
        Some(GraphicsStage::VertexShader) => vk::PipelineStageFlags2::VERTEX_SHADER,
        Some(GraphicsStage::EarlyFragmentTests) => vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
        Some(GraphicsStage::FragmentShader) => vk::PipelineStageFlags2::FRAGMENT_SHADER,
        //Some(GraphicsStage::LateFragmentTests) => vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
        Some(GraphicsStage::ColorAttachmentOutput) => {
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT
        }
        None => vk::PipelineStageFlags2::empty(),
    };

    transfer | graphics
}

fn validate_shader_bindings(shader: &super::ShaderModule, descriptors: &[&DescriptorSetLayout]) {
    for shader_binding in &shader.shader.bindings() {
        if shader_binding.group >= descriptors.len() as u32 {
            panic!(
                "shader requires descriptor set bound to group {} (only {} descriptor sets were bound)",
                shader_binding.group,
                descriptors.len(),
            );
        }

        let Some(binding) = descriptors[shader_binding.group as usize]
            .bindings
            .iter()
            .find(|descriptor_binding| descriptor_binding.binding == shader_binding.binding)
        else {
            panic!(
                "shader requires descriptor set with binding {} in group {}",
                shader_binding.group, shader_binding.binding,
            );
        };

        assert!(shader_binding.kind == binding.kind);
    }
}

fn create_pipeline_shader_module(
    shader: &Shader,
    entry_point: &str,
    stage: ShaderStage,
    layouts: &[&DescriptorSetLayout],
) -> Vec<u32> {
    // FIXME: Doubles with validate_shader_bindings.
    let mut binding_map = HashMap::new();
    for binding in shader.bindings() {
        let Some(layout) = layouts.get(binding.group as usize) else {
            panic!("shader binding {:?} is not bound", binding);
        };

        let Some(binding_layout) = layout
            .bindings
            .iter()
            .find(|b| b.binding == binding.binding)
        else {
            panic!("shader binding {:?} is not bound", binding);
        };

        if let Some(count) = binding.count {
            assert_eq!(
                binding_layout.count, count,
                "shader expects {} descriptors, layout provides {}",
                count, binding_layout.count,
            );
        } else {
            binding_map.insert(
                binding.location(),
                BindingInfo {
                    count: binding_layout.count,
                },
            );
        }
    }

    let instance = shader.instantiate(&shader::Options {
        entry_point,
        stage,
        bindings: binding_map,
    });

    instance.to_spirv()
}
