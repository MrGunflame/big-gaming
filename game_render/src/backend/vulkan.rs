mod alloc;

use std::alloc::Layout;
use std::backtrace::Backtrace;
use std::borrow::Cow;
use std::ffi::{c_void, CStr, CString};
use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::{Bound, Deref, Range, RangeBounds};
use std::ptr::{null_mut, NonNull};
use std::sync::atomic::{fence, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use allocator_api2::vec::Vec as VecWithAlloc;
use ash::vk::Handle;
use ash::{vk, Entry};
use game_common::collections::scratch_buffer::ScratchBuffer;
use game_common::utils::vec_ext::VecExt;
use glam::UVec2;
use hashbrown::HashMap;
use parking_lot::Mutex;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use thiserror::Error;

use crate::backend::vulkan::alloc::BumpAllocator;
use crate::backend::{
    mip_level_size_2d, DedicatedAllocation, DescriptorType, SurfaceFormat, TextureLayout,
};
use crate::shader::ShaderInstance;

use super::{
    AccessFlags, AdapterKind, AdapterMemoryProperties, AdapterProperties, AddressMode, BlendFactor,
    BlendOp, BufferUsage, BufferView, ColorSpace, CompareOp, CopyBuffer, DedicatedResource,
    DescriptorPoolDescriptor, DescriptorSetDescriptor, Face, FilterMode, FrontFace, IndexFormat,
    LoadOp, MemoryHeap, MemoryHeapFlags, MemoryRequirements, MemoryType, MemoryTypeFlags,
    PipelineBarriers, PipelineDescriptor, PipelineStage, PresentMode, PrimitiveTopology, QueryKind,
    QueryPoolDescriptor, QueueCapabilities, QueueFamily, QueueFamilyId, QueuePresent, QueueSubmit,
    RenderPassDescriptor, SamplerDescriptor, ShaderStage, ShaderStages, StoreOp,
    SwapchainCapabilities, SwapchainConfig, TextureDescriptor, TextureFormat, TextureUsage,
    TextureViewDescriptor, TimestampPipelineStage, WriteDescriptorResource,
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
    /// `VK_EXT_surface_maintenance1`
    ///
    /// Provides the ability to attach a fence to a `vkQueuePresent`, which is needed to legally
    /// destroy a swapchain.
    /// See <https://github.com/KhronosGroup/Vulkan-Docs/issues/1678>
    surface_maintenance1: bool,
    /// `VK_KHR_get_surface_capabilities2`
    ///
    /// Dependency for `surface_maintenance1`.
    get_surface_capabilities2: bool,
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
            (self.surface_maintenance1, vk::EXT_SURFACE_MAINTENANCE1_NAME),
            (
                self.get_surface_capabilities2,
                vk::KHR_GET_SURFACE_CAPABILITIES2_NAME,
            ),
        ] {
            if enabled {
                names.push(name);
            }
        }

        names
    }
}

impl<'a> FromIterator<&'a CStr> for InstanceExtensions {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = &'a CStr>,
    {
        let mut extensions = Self::default();
        for name in iter {
            match name {
                name if name == vk::KHR_SURFACE_NAME => extensions.surface = true,
                name if name == vk::KHR_WAYLAND_SURFACE_NAME => extensions.surface_wayland = true,
                name if name == vk::KHR_XCB_SURFACE_NAME => extensions.surface_xcb = true,
                name if name == vk::KHR_XLIB_SURFACE_NAME => extensions.surface_xlib = true,
                name if name == vk::KHR_WIN32_SURFACE_NAME => extensions.surface_win32 = true,
                name if name == vk::EXT_DEBUG_UTILS_NAME => extensions.debug_utils = true,
                name if name == vk::EXT_SURFACE_MAINTENANCE1_NAME => {
                    extensions.surface_maintenance1 = true
                }
                name if name == vk::KHR_GET_SURFACE_CAPABILITIES2_NAME => {
                    extensions.get_surface_capabilities2 = true
                }
                _ => (),
            }
        }
        extensions
    }
}

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
    #[error("missing features: {0}")]
    MissingFeatures(DeviceFeatures),
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
    /// GPU Assisted validation. Expensive
    pub gpuav: bool,
    /// Enable support for GPU assisted performance counters.
    pub performance_counters: bool,
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
        tracing::info!(
            "Instance::create(): VALIDATION_LAYERS={} GPUAV={}",
            config.validation,
            config.gpuav
        );

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
        const FALSE: &[u8] = &vk::FALSE.to_ne_bytes();

        let gpuav_enabled = if config.gpuav { TRUE } else { FALSE };

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
            (c"gpuav_enable", gpuav_enabled),
            (c"gpuav_safe_mode", gpuav_enabled),
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

        let instance = unsafe { entry.create_instance(&info, ALLOC)? };

        let extensions = InstanceExtensionFns::new(&entry, &instance, &supported_extensions);

        let messenger = if config.validation {
            if let Some(instance_d) = &extensions.debug_utils {
                match unsafe { instance_d.create_debug_utils_messenger(&debug_info, ALLOC) } {
                    Ok(messenger) => Some(messenger),
                    Err(err) => {
                        // We must manually destroy the instance if an error occurs,
                        // otherwise the vkInstance would leak.
                        unsafe {
                            instance.destroy_instance(ALLOC);
                        }

                        return Err(err.into());
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            instance: Arc::new(InstanceShared {
                extensions: InstanceExtensionFns::new(&entry, &instance, &supported_extensions),
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
    /// - If an extension to create the surface is missing, [`MissingExtension`] is returned.
    /// - If the handle is not recognised, [`UnsupportedSurface`] is returned.
    ///
    /// # Safety
    ///
    /// - The passed `display` and `window` handles must be valid until the [`Surface`] is dropped.
    ///
    /// [`MissingExtension`]: Error::MissingExtension
    /// [`UnsupportedSurface`]: Error::UnsupportedSurface
    pub unsafe fn create_surface(
        &self,
        display: RawDisplayHandle,
        window: RawWindowHandle,
    ) -> Result<Surface, Error> {
        if !self.extensions.surface {
            return Err(Error::MissingExtension(vk::KHR_SURFACE_NAME));
        }
        if !self.extensions.surface_maintenance1 {
            return Err(Error::MissingExtension(vk::EXT_SURFACE_MAINTENANCE1_NAME));
        }

        let surface = match (display, window) {
            #[cfg(all(unix, feature = "wayland"))]
            (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
                let Some(instance) = &self.instance.extensions.surface_wayland else {
                    return Err(Error::MissingExtension(vk::KHR_WAYLAND_SURFACE_NAME));
                };

                let info = vk::WaylandSurfaceCreateInfoKHR::default()
                    // - `display` must be a valid Wayland `wl_display`.
                    .display(display.display.as_ptr())
                    // - `surface` must be a valid Wayland `wl_surface`.
                    .surface(window.surface.as_ptr())
                    // - `flags` must be `0`.
                    .flags(vk::WaylandSurfaceCreateFlagsKHR::empty());

                unsafe { instance.create_wayland_surface(&info, ALLOC)? }
            }
            #[cfg(all(unix, feature = "x11"))]
            (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
                let Some(instance) = &self.instance.extensions.surface_xcb else {
                    return Err(Error::MissingExtension(vk::KHR_XCB_SURFACE_NAME));
                };

                let info = vk::XcbSurfaceCreateInfoKHR::default()
                    // - `connection` must point to a valid X11 `xcb_connection_t`.
                    .connection(display.connection.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    // - `window` must be a valid X11 `xcb_window_t`.
                    .window(window.window.get())
                    // - `flags` must be `0`.
                    .flags(vk::XcbSurfaceCreateFlagsKHR::empty());

                unsafe { instance.create_xcb_surface(&info, ALLOC)? }
            }
            #[cfg(all(unix, feature = "x11"))]
            (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
                let Some(instance) = &self.instance.extensions.surface_xlib else {
                    return Err(Error::MissingExtension(vk::KHR_XLIB_SURFACE_NAME));
                };

                let info = vk::XlibSurfaceCreateInfoKHR::default()
                    // - `dpy` must point to a valid Xlib `Display`.
                    .dpy(display.display.map(|v| v.as_ptr()).unwrap_or(null_mut()))
                    // - `window` must point to a valid Xlib `Window`.
                    .window(window.window)
                    // - `flags` must be `0`.
                    .flags(vk::XlibSurfaceCreateFlagsKHR::empty());

                unsafe { instance.create_xlib_surface(&info, ALLOC)? }
            }
            #[cfg(windows)]
            (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
                let Some(instance) = &self.instance.extensions.surface_win32 else {
                    return Err(Error::MissingExtension(vk::KHR_WIN32_SURFACE_NAME));
                };

                let info = vk::Win32SurfaceCreateInfoKHR::default()
                    // - `hinstance` must be a valid Win32 `HINSTANCE`.
                    .hinstance(window.hinstance.map(|v| v.get()).unwrap_or_default())
                    // - `hwnd` must be a valid Win32 `HWND`.
                    .hwnd(window.hwnd.get())
                    // - `flags` must be `0`.
                    .flags(vk::Win32SurfaceCreateFlagsKHR::empty());

                unsafe { instance.create_win32_surface(&info, ALLOC)? }
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
        let ext_props = unsafe { entry.enumerate_instance_extension_properties(None).unwrap() };
        ext_props
            .iter()
            .map(|props| {
                CStr::from_bytes_until_nul(bytemuck::bytes_of(&props.extension_name)).unwrap()
            })
            .collect()
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

        let formats = self.get_supported_format_usages();

        AdapterProperties {
            name,
            kind,
            formats,
        }
    }

    fn get_supported_format_usages(&self) -> HashMap<TextureFormat, TextureUsage> {
        let mut formats = HashMap::new();
        for format in TextureFormat::all() {
            let mut props = vk::FormatProperties2::default();

            unsafe {
                self.instance.get_physical_device_format_properties2(
                    self.physical_device,
                    vk::Format::from(*format),
                    &mut props,
                )
            }

            let features = props.format_properties.optimal_tiling_features;
            let mut usages = TextureUsage::empty();

            if features.contains(vk::FormatFeatureFlags::TRANSFER_SRC) {
                usages |= TextureUsage::TRANSFER_SRC;
            }
            if features.contains(vk::FormatFeatureFlags::TRANSFER_DST) {
                usages |= TextureUsage::TRANSFER_DST;
            }
            if features.contains(vk::FormatFeatureFlags::SAMPLED_IMAGE) {
                usages |= TextureUsage::TEXTURE_BINDING;
            }
            if features.contains(vk::FormatFeatureFlags::STORAGE_IMAGE) {
                usages |= TextureUsage::STORAGE;
            }
            if features.contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT)
                || features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            {
                usages |= TextureUsage::RENDER_ATTACHMENT;
            }

            formats.insert(*format, usages);
        }

        formats
    }

    fn get_supported_extensions(&self) -> DeviceExtensions {
        let ext_props = unsafe {
            self.instance
                .instance
                .enumerate_device_extension_properties(self.physical_device)
                .unwrap()
        };
        ext_props
            .iter()
            .map(|props| {
                CStr::from_bytes_until_nul(bytemuck::bytes_of(&props.extension_name)).unwrap()
            })
            .collect()
    }

    fn get_supported_features(&self) -> DeviceFeatures {
        let extensions = self.get_supported_extensions();

        let mut features11 = vk::PhysicalDeviceVulkan11Features::default();
        let mut features12 = vk::PhysicalDeviceVulkan12Features::default();
        let mut features13 = vk::PhysicalDeviceVulkan13Features::default();

        let mut swapchain_maintenance1 =
            vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default();
        let mut mesh_shader = vk::PhysicalDeviceMeshShaderFeaturesEXT::default();

        let mut features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut features11)
            .push_next(&mut features12)
            .push_next(&mut features13);

        if extensions.swapchain_maintenance1 {
            features = features.push_next(&mut swapchain_maintenance1);
        }

        if extensions.mesh_shader {
            features = features.push_next(&mut mesh_shader);
        }

        unsafe {
            self.instance
                .instance
                .get_physical_device_features2(self.physical_device, &mut features);
        }

        let features = features.features;

        let cast = |v: vk::Bool32| -> bool {
            // All bool values MUST be 0 or 1.
            // https://registry.khronos.org/vulkan/specs/latest/man/html/VkBool32.html
            debug_assert!(v == 0 || v == 1);
            v != 0
        };

        DeviceFeatures {
            mutli_draw_indirect: cast(features.multi_draw_indirect),
            storage_buffer_16bit_access: cast(features11.storage_buffer16_bit_access),
            shader_draw_parameters: cast(features11.shader_draw_parameters),
            shader_input_attachment_array_dynamic_indexing: cast(
                features12.shader_input_attachment_array_dynamic_indexing,
            ),
            shader_uniform_texel_buffer_array_dynamic_indexing: cast(
                features12.shader_uniform_texel_buffer_array_dynamic_indexing,
            ),
            shader_storage_texel_buffer_array_dynamic_indexing: cast(
                features12.shader_storage_texel_buffer_array_dynamic_indexing,
            ),
            shader_uniform_buffer_array_non_uniform_indexing: cast(
                features12.shader_uniform_buffer_array_non_uniform_indexing,
            ),
            shader_sampled_image_array_non_uniform_indexing: cast(
                features12.shader_sampled_image_array_non_uniform_indexing,
            ),
            shader_storage_buffer_array_non_uniform_indexing: cast(
                features12.shader_storage_buffer_array_non_uniform_indexing,
            ),
            shader_storage_image_array_non_uniform_indexing: cast(
                features12.shader_storage_image_array_non_uniform_indexing,
            ),
            shader_input_attachment_array_non_uniform_indexing: cast(
                features12.shader_input_attachment_array_non_uniform_indexing,
            ),
            shader_uniform_texel_buffer_array_non_uniform_indexing: cast(
                features12.shader_uniform_texel_buffer_array_non_uniform_indexing,
            ),
            shader_storage_texel_buffer_array_non_uniform_indexing: cast(
                features12.shader_storage_texel_buffer_array_non_uniform_indexing,
            ),
            descriptor_binding_uniform_buffer_update_after_bind: cast(
                features12.descriptor_binding_uniform_buffer_update_after_bind,
            ),
            descriptor_binding_sampled_image_update_after_bind: cast(
                features12.descriptor_binding_sampled_image_update_after_bind,
            ),
            descriptor_binding_storage_image_update_after_bind: cast(
                features12.descriptor_binding_storage_image_update_after_bind,
            ),
            descriptor_binding_storage_buffer_update_after_bind: cast(
                features12.descriptor_binding_storage_buffer_update_after_bind,
            ),
            descriptor_binding_uniform_texel_buffer_update_after_bind: cast(
                features12.descriptor_binding_uniform_texel_buffer_update_after_bind,
            ),
            descriptor_binding_storage_texel_buffer_update_after_bind: cast(
                features12.descriptor_binding_storage_texel_buffer_update_after_bind,
            ),
            descriptor_binding_update_unused_while_pending: cast(
                features12.descriptor_binding_update_unused_while_pending,
            ),
            descriptor_binding_partially_bound: cast(features12.descriptor_binding_partially_bound),
            descriptor_binding_variable_descriptor_count: cast(
                features12.descriptor_binding_variable_descriptor_count,
            ),
            runtime_descriptor_array: cast(features12.runtime_descriptor_array),
            storage_buffer_8bit_access: cast(features12.storage_buffer8_bit_access),
            uniform_and_storage_buffer_8bit_access: cast(
                features12.uniform_and_storage_buffer8_bit_access,
            ),
            storage_push_constant8: cast(features12.storage_push_constant8),
            shader_float16: cast(features12.shader_float16),
            shader_int8: cast(features12.shader_int8),
            host_query_reset: cast(features12.host_query_reset),
            dynamic_rendering: cast(features13.dynamic_rendering),
            synchronization2: cast(features13.synchronization2),
            swapchain_maintenace1: cast(swapchain_maintenance1.swapchain_maintenance1),
            task_shader: cast(mesh_shader.task_shader),
            mesh_shader: cast(mesh_shader.mesh_shader),
        }
    }

    /// Queries and returns information about this `Adapter`'s memories.
    pub fn memory_properties(&self) -> AdapterMemoryProperties {
        let props = unsafe {
            self.instance
                .instance
                .get_physical_device_memory_properties(self.physical_device)
        };

        let limits = self.device_limits();

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

        AdapterMemoryProperties {
            heaps,
            types,
            max_allocation_size: unsafe {
                NonZeroU64::new_unchecked(limits.max_memory_allocation_size)
            },
        }
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
                    timestamp_bits: queue.timestamp_valid_bits,
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

        let supported_features = self.get_supported_features();
        let supported_extensions = self.get_supported_extensions();
        if !supported_extensions.swapchain {
            return Err(Error::MissingExtension(DeviceExtensions::SWAPCHAIN));
        }

        let mut extensions = Vec::new();
        extensions.extend(supported_extensions.names().iter().map(|v| v.as_ptr()));

        let required_features = DeviceFeatures {
            storage_buffer_16bit_access: true,
            mutli_draw_indirect: true,
            shader_draw_parameters: true,
            shader_input_attachment_array_dynamic_indexing: true,
            shader_uniform_texel_buffer_array_dynamic_indexing: true,
            shader_storage_texel_buffer_array_dynamic_indexing: true,
            shader_uniform_buffer_array_non_uniform_indexing: true,
            shader_sampled_image_array_non_uniform_indexing: true,
            shader_storage_buffer_array_non_uniform_indexing: true,
            shader_storage_image_array_non_uniform_indexing: true,
            shader_input_attachment_array_non_uniform_indexing: true,
            shader_uniform_texel_buffer_array_non_uniform_indexing: true,
            shader_storage_texel_buffer_array_non_uniform_indexing: true,
            descriptor_binding_uniform_buffer_update_after_bind: true,
            descriptor_binding_sampled_image_update_after_bind: true,
            descriptor_binding_storage_image_update_after_bind: true,
            descriptor_binding_storage_buffer_update_after_bind: true,
            descriptor_binding_uniform_texel_buffer_update_after_bind: true,
            descriptor_binding_storage_texel_buffer_update_after_bind: true,
            descriptor_binding_update_unused_while_pending: true,
            descriptor_binding_partially_bound: true,
            descriptor_binding_variable_descriptor_count: true,
            runtime_descriptor_array: true,
            shader_float16: true,
            shader_int8: true,
            storage_buffer_8bit_access: true,
            uniform_and_storage_buffer_8bit_access: true,
            storage_push_constant8: false,
            dynamic_rendering: true,
            synchronization2: true,
            // Optional features
            task_shader: false,
            mesh_shader: false,
            swapchain_maintenace1: false,
            host_query_reset: self.instance.config.performance_counters,
        };

        supported_features.validate_requirements(required_features)?;

        let features = vk::PhysicalDeviceFeatures::default()
            // Allows passing a draw count greater than 1 to indirect
            // draw calls.
            .multi_draw_indirect(true)
            .shader_int16(true)
            .shader_int64(true)
            .fragment_stores_and_atomics(true);

        let mut features11 = vk::PhysicalDeviceVulkan11Features::default()
            // Enables `SPV_KHR_shader_draw_parameters`, which in turns provides
            // `BaseInstance`, `BaseVertex` and `DrawIndex` needed for indirect
            // draws.
            .shader_draw_parameters(true)
            .storage_buffer16_bit_access(true);

        let mut features12 = vk::PhysicalDeviceVulkan12Features::default()
            // Runtime Descriptor indexing
            .shader_input_attachment_array_dynamic_indexing(true)
            .shader_uniform_texel_buffer_array_dynamic_indexing(true)
            .shader_storage_texel_buffer_array_dynamic_indexing(true)
            .shader_uniform_buffer_array_non_uniform_indexing(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .shader_storage_buffer_array_non_uniform_indexing(true)
            .shader_storage_image_array_non_uniform_indexing(true)
            .shader_uniform_texel_buffer_array_non_uniform_indexing(true)
            .shader_storage_texel_buffer_array_non_uniform_indexing(true)
            .shader_input_attachment_array_non_uniform_indexing(true)
            .descriptor_binding_uniform_buffer_update_after_bind(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_uniform_texel_buffer_update_after_bind(true)
            .descriptor_binding_storage_texel_buffer_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_variable_descriptor_count(true)
            .runtime_descriptor_array(true)
            // Shader float16 and int8
            .shader_float16(true)
            .shader_int8(true)
            // 8 Bit Storage
            .storage_buffer8_bit_access(true)
            .uniform_and_storage_buffer8_bit_access(true)
            .storage_push_constant8(false)
            // Needed for performance measuring.
            .host_query_reset(self.instance.config.performance_counters);

        let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true)
            .shader_demote_to_helper_invocation(true);

        let mut swapchain_maintenance1 =
            vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default()
                .swapchain_maintenance1(true);

        let mut mesh_shader = vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
            .task_shader(true)
            .mesh_shader(true);

        // Allow passing deprecated `enabled_layer_names`.
        #[allow(deprecated)]
        let mut create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            // Device layers are deprecated, but the Vulkan spec still recommends
            // applications to pass layers.
            // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#extendingvulkan-layers-devicelayerdeprecation
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features)
            .push_next(&mut features11)
            .push_next(&mut features12)
            .push_next(&mut features13);

        if supported_extensions.swapchain_maintenance1 {
            create_info = create_info.push_next(&mut swapchain_maintenance1);
        }

        if supported_extensions.mesh_shader {
            create_info = create_info.push_next(&mut mesh_shader);
        }

        let device = unsafe {
            self.instance
                .instance
                .create_device(self.physical_device, &create_info, ALLOC)?
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
                extensions: DeviceExtensionFns::new(&self.instance, &device, &supported_extensions),
                instance: self.instance.clone(),
                device,
                limits: self.device_limits(),
                memory_properties: self.memory_properties(),
                num_allocations: Arc::new(AtomicU32::new(0)),
                queues,
                queue_families: queue_families.to_vec(),
            }),
        })
    }

    fn device_limits(&self) -> DeviceLimits {
        let mut maintenance3 = vk::PhysicalDeviceMaintenance3Properties::default();
        let mut mesh_shader = vk::PhysicalDeviceMeshShaderPropertiesEXT::default();
        let mut props = vk::PhysicalDeviceProperties2::default()
            .push_next(&mut maintenance3)
            .push_next(&mut mesh_shader);

        unsafe {
            self.instance
                .get_physical_device_properties2(self.physical_device, &mut props);
        }

        DeviceLimits {
            max_push_constants_size: props.properties.limits.max_push_constants_size,
            max_bound_descriptor_sets: props.properties.limits.max_bound_descriptor_sets,
            max_memory_allocation_count: props.properties.limits.max_memory_allocation_count,
            buffer_image_granularity: props.properties.limits.buffer_image_granularity,
            max_per_stage_descriptor_samplers: props
                .properties
                .limits
                .max_per_stage_descriptor_samplers,
            max_per_stage_descriptor_uniform_buffers: props
                .properties
                .limits
                .max_per_stage_descriptor_uniform_buffers,
            max_per_stage_descriptor_storage_buffers: props
                .properties
                .limits
                .max_per_stage_descriptor_storage_buffers,
            max_per_stage_descriptor_sampled_images: props
                .properties
                .limits
                .max_per_stage_descriptor_sampled_images,
            max_per_stage_resources: props.properties.limits.max_per_stage_resources,
            max_descriptor_set_sampled_images: props
                .properties
                .limits
                .max_descriptor_set_sampled_images,
            max_descriptor_set_storage_images: props
                .properties
                .limits
                .max_descriptor_set_storage_images,
            max_descriptor_set_samplers: props.properties.limits.max_descriptor_set_samplers,
            max_descriptor_set_storage_buffers: props
                .properties
                .limits
                .max_descriptor_set_storage_buffers,
            max_descriptor_set_uniform_buffers: props
                .properties
                .limits
                .max_descriptor_set_uniform_buffers,
            max_color_attachments: props.properties.limits.max_color_attachments,
            max_compute_work_group_count: props.properties.limits.max_compute_work_group_count,
            max_compute_work_group_invocations: props
                .properties
                .limits
                .max_compute_work_group_invocations,
            max_compute_work_group_size: props.properties.limits.max_compute_work_group_size,
            timestamp_period_nanos: props.properties.limits.timestamp_period,
            max_per_set_descriptors: maintenance3.max_per_set_descriptors,
            max_memory_allocation_size: maintenance3.max_memory_allocation_size,
            max_task_work_group_total_count: mesh_shader.max_task_work_group_total_count,
            max_task_work_group_count: mesh_shader.max_task_work_group_count,
            max_task_work_group_invocations: mesh_shader.max_task_work_group_invocations,
            max_task_work_group_size: mesh_shader.max_task_work_group_size,
            max_task_payload_size: mesh_shader.max_task_payload_size,
            max_mesh_work_group_total_count: mesh_shader.max_mesh_work_group_total_count,
            max_mesh_work_group_count: mesh_shader.max_mesh_work_group_count,
            max_mesh_work_group_invocations: mesh_shader.max_mesh_work_group_invocations,
            max_mesh_work_group_size: mesh_shader.max_mesh_work_group_size,
            max_mesh_output_vertices: mesh_shader.max_mesh_output_vertices,
            max_mesh_output_primitives: mesh_shader.max_mesh_output_primitives,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    physical_device: vk::PhysicalDevice,
    device: Arc<DeviceShared>,
}

impl Device {
    fn get_format_features(&self, format: TextureFormat) -> vk::FormatFeatureFlags {
        let mut props = vk::FormatProperties2::default();

        unsafe {
            self.device.instance.get_physical_device_format_properties2(
                self.physical_device,
                format.into(),
                &mut props,
            );
        }

        // buffer and linear features are uninteresting as we never
        // use them.
        props.format_properties.optimal_tiling_features
    }

    /// Returns the enabled device extensions.
    pub fn extensions(&self) -> DeviceExtensions {
        DeviceExtensions {
            swapchain: self.device.extensions.swapchain.is_some(),
            swapchain_maintenance1: self.device.extensions.swapchain_maintenance1.is_some(),
            mesh_shader: self.device.extensions.mesh_shader.is_some(),
        }
    }

    pub fn limits(&self) -> DeviceLimits {
        self.device.limits
    }

    ///
    /// # Safety
    ///
    /// All queues of this `Device` must be externally synchronized, i.e. no call to any function
    /// of any [`Queue`] must be currently active on another thread.
    pub unsafe fn wait_idle(&self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
    }

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

        let family_data = self
            .device
            .queue_families
            .iter()
            .find(|f| f.id == family)
            .unwrap();

        Ok(Queue {
            device: self.device.clone(),
            queue,
            queue_family: family,
            queue_index,
            family: *family_data,
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

        let buffer = unsafe { self.device.create_buffer(&info, ALLOC)? };
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
        dedicated_for: Option<DedicatedResource<'_>>,
    ) -> Result<DeviceMemory, Error> {
        let heap = self.device.memory_properties.types[memory_type_index as usize].heap;

        assert!(
            !self.device.memory_properties.types[memory_type_index as usize]
                .flags
                .contains(MemoryTypeFlags::_VK_PROTECTED),
        );

        let mut dedicated_info = match dedicated_for {
            Some(DedicatedResource::Buffer(buffer)) => {
                vk::MemoryDedicatedAllocateInfo::default().buffer(buffer.buffer)
            }
            Some(DedicatedResource::Texture(texture)) => {
                vk::MemoryDedicatedAllocateInfo::default().image(texture.image)
            }
            None => vk::MemoryDedicatedAllocateInfo::default(),
        };

        let mut info = vk::MemoryAllocateInfo::default()
            // - `allocationSize` must be greater than 0.
            .allocation_size(size.get())
            // - memoryTypeIndex must not indicate a memory type that reports `VK_MEMORY_PROPERTY_PROTECTED_BIT`.
            .memory_type_index(memory_type_index);

        if dedicated_for.is_some() {
            info = info.push_next(&mut dedicated_info);
        }

        assert!(
            size.get() <= u64::from(self.device.memory_properties.heaps[heap as usize].size),
            "attempted to allocate more than heap size: heap size = {}, allocation = {}",
            self.device.memory_properties.heaps[heap as usize].size,
            size,
        );

        assert!(
            size.get() <= self.device.limits.max_memory_allocation_size,
            "attempted to allocate ({}) more than max_memory_allocation_size ({})",
            size,
            self.device.limits.max_memory_allocation_size,
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
            self.device.allocate_memory(&info, ALLOC)
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

        let mut dedicated_req = vk::MemoryDedicatedRequirements::default();
        let mut req = vk::MemoryRequirements2::default().push_next(&mut dedicated_req);
        let info = vk::BufferMemoryRequirementsInfo2::default().buffer(buffer.buffer);

        unsafe {
            self.device.get_buffer_memory_requirements2(&info, &mut req);
        }

        // Since buffer with size 0 are forbidden, the size/align
        // of any buffer is not 0.
        debug_assert!(req.memory_requirements.size > 0);
        debug_assert!(req.memory_requirements.alignment > 0);

        // See https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#resources-association
        // - The `alignment` member is a power of two.
        debug_assert!(req.memory_requirements.alignment.is_power_of_two());

        let size = unsafe { NonZeroU64::new_unchecked(req.memory_requirements.size) };
        let align = unsafe { NonZeroU64::new_unchecked(req.memory_requirements.alignment) };

        // Bit `i` is set iff the memory type at index `i` is
        // supported for this buffer.
        let mut memory_types = Vec::new();
        let mut bits = req.memory_requirements.memory_type_bits;
        while bits != 0 {
            let index = bits.trailing_zeros();
            memory_types.push(index);
            bits &= !(1 << index);
        }

        let dedicated = match (
            dedicated_req.requires_dedicated_allocation,
            dedicated_req.prefers_dedicated_allocation,
        ) {
            (vk::TRUE, _) => DedicatedAllocation::Required,
            (_, vk::TRUE) => DedicatedAllocation::Preferred,
            _ => DedicatedAllocation::None,
        };

        MemoryRequirements {
            size,
            align,
            memory_types,
            dedicated,
        }
    }

    /// Returns the [`MemoryRequirements`] for a [`Texture`].
    pub fn image_memory_requirements(&self, texture: &Texture) -> MemoryRequirements {
        assert!(self.device.same(&texture.device));

        let mut dedicated_req = vk::MemoryDedicatedRequirements::default();
        let mut req = vk::MemoryRequirements2::default().push_next(&mut dedicated_req);
        let info = vk::ImageMemoryRequirementsInfo2::default().image(texture.image);

        unsafe {
            self.device.get_image_memory_requirements2(&info, &mut req);
        }

        // Bit `i` is set iff the memory type at index `i` is
        // supported for this buffer.
        let mut memory_types = Vec::new();
        let mut bits = req.memory_requirements.memory_type_bits;
        while bits != 0 {
            let index = bits.trailing_zeros();
            memory_types.push(index);
            bits &= !(1 << index);
        }

        debug_assert!(req.memory_requirements.size > 0);
        debug_assert!(req.memory_requirements.alignment > 0);

        // See https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#resources-association
        // - The `alignment` member is a power of two.
        debug_assert!(req.memory_requirements.alignment.is_power_of_two());

        // To handle `bufferImageGranularity` we just overalign all images
        // to `bufferImageGranularity`. This means the image will always
        // start on a fresh "page".
        // To ensure that the next resource is a new "page" we grow the size
        // to the next multiple of `bufferImageGranularity`.
        // This is usually not a problem, since images already have a big
        // alignment and size and `bufferImageGranularity` is usually relatively small.
        let buffer_image_granularity = self.device.limits.buffer_image_granularity;
        let align = u64::max(req.memory_requirements.alignment, buffer_image_granularity);
        // size + (size % align) = (size + align - 1) & !(align - 1)
        let size = (req.memory_requirements.size + buffer_image_granularity - 1)
            & !(buffer_image_granularity - 1);

        debug_assert_eq!(align % self.device.limits.buffer_image_granularity, 0);
        debug_assert_eq!(size % self.device.limits.buffer_image_granularity, 0);

        let dedicated = match (
            dedicated_req.requires_dedicated_allocation,
            dedicated_req.prefers_dedicated_allocation,
        ) {
            (vk::TRUE, _) => DedicatedAllocation::Required,
            (_, vk::TRUE) => DedicatedAllocation::Preferred,
            _ => DedicatedAllocation::None,
        };

        MemoryRequirements {
            size: unsafe { NonZeroU64::new_unchecked(size) },
            align: unsafe { NonZeroU64::new_unchecked(align) },
            memory_types,
            dedicated,
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

        let image = unsafe { self.device.create_image(&info, ALLOC)? };
        Ok(Texture {
            device: self.device.clone(),
            image,
            format: descriptor.format,
            size: descriptor.size,
            destroy_on_drop: true,
            mip_levels: descriptor.mip_levels,
        })
    }

    /// Creates a [`ShaderModule`] from SPIR-V bytecode.
    ///
    /// # Safety
    ///
    /// The SPIR-V bytecode must be well formed according to both the SPIR-V and Vulkan
    /// specifications.
    unsafe fn create_shader_module_spirv(&self, code: &[u32]) -> Result<ShaderModule, Error> {
        // Code size must be greater than 0.
        debug_assert!(code.len() != 0);

        let info = vk::ShaderModuleCreateInfo::default().code(code);

        let shader = unsafe { self.device.create_shader_module(&info, ALLOC)? };
        Ok(ShaderModule {
            device: self.device.clone(),
            shader,
        })
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
        let layout = unsafe { self.device.create_descriptor_set_layout(&info, ALLOC)? };

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
        let mut sampled_images = 0;
        let mut storage_images = 0;

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
                    DescriptorType::Texture => sampled_images += count,
                    DescriptorType::StorageTexture => storage_images += count,
                }
            }
        }

        // These must be true accross all pipeline stages.
        assert!(samplers <= self.device.limits.max_descriptor_set_samplers);
        assert!(uniform_buffers <= self.device.limits.max_descriptor_set_uniform_buffers);
        assert!(storage_buffers <= self.device.limits.max_descriptor_set_storage_buffers);
        assert!(sampled_images <= self.device.limits.max_descriptor_set_sampled_images);
        assert!(storage_images <= self.device.limits.max_descriptor_set_storage_images);

        // These must only be true for each pipeline stage individually.
        // FIXME: Right now count all descriptors in all pipeline stages,
        // which is more restrictive that necessary.
        assert!(samplers <= self.device.limits.max_per_stage_descriptor_samplers);
        assert!(uniform_buffers <= self.device.limits.max_per_stage_descriptor_uniform_buffers);
        assert!(storage_buffers <= self.device.limits.max_per_stage_descriptor_storage_buffers);
        assert!(sampled_images <= self.device.limits.max_per_stage_descriptor_sampled_images);
        assert!(
            samplers + uniform_buffers + storage_buffers + sampled_images
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
        let layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_info, ALLOC)?
        };

        let mut stages = Vec::new();

        let shader_modules = ScratchBuffer::new(descriptor.stages.len());
        let stage_entry_pointers = ScratchBuffer::new(descriptor.stages.len());

        {
            const VALID_STAGE_CONFIGS: &[&[ShaderStage]] = &[
                &[ShaderStage::Vertex, ShaderStage::Fragment],
                &[ShaderStage::Task, ShaderStage::Mesh, ShaderStage::Fragment],
                &[ShaderStage::Mesh, ShaderStage::Fragment],
                &[ShaderStage::Compute],
            ];

            let requested_stages: Vec<_> = descriptor
                .stages
                .iter()
                .map(|stage| stage.shader_stage())
                .collect();

            if !VALID_STAGE_CONFIGS.contains(&requested_stages.as_slice()) {
                panic!("invalid pipeline stage composition: {:?}", requested_stages);
            }
        }

        for stage in descriptor.stages {
            let vk_stage = match stage {
                PipelineStage::Vertex(stage) => {
                    let module = shader_modules.insert(create_pipeline_shader_module(
                        self,
                        &stage.shader,
                        descriptor.descriptors,
                    )?);
                    let name = stage_entry_pointers.insert(CString::new(stage.entry).unwrap());

                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::VERTEX)
                        .module(module.shader)
                        .name(&*name)
                }
                PipelineStage::Fragment(stage) => {
                    for target in stage.targets {
                        if !self
                            .get_format_features(target.format)
                            .contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT)
                        {
                            panic!(
                                "format {:?} does not support being used as an color attachment",
                                target
                            );
                        }
                    }

                    let module = shader_modules.insert(create_pipeline_shader_module(
                        self,
                        &stage.shader,
                        descriptor.descriptors,
                    )?);
                    let name = stage_entry_pointers.insert(CString::new(stage.entry).unwrap());

                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::FRAGMENT)
                        .module(module.shader)
                        .name(&*name)
                }
                PipelineStage::Task(stage) => {
                    assert!(
                        self.device.extensions.mesh_shader.is_some(),
                        "Cannot use Task shader when EXT_MESH_SHADER is not enabled"
                    );

                    let module = shader_modules.insert(create_pipeline_shader_module(
                        self,
                        &stage.shader,
                        descriptor.descriptors,
                    )?);
                    let name = stage_entry_pointers.insert(CString::new(stage.entry).unwrap());

                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::TASK_EXT)
                        .module(module.shader)
                        .name(&*name)
                }
                PipelineStage::Mesh(stage) => {
                    assert!(
                        self.device.extensions.mesh_shader.is_some(),
                        "Cannot use Mesh shader when EXT_MESH_SHADER is not enabled"
                    );

                    let module = shader_modules.insert(create_pipeline_shader_module(
                        self,
                        &stage.shader,
                        descriptor.descriptors,
                    )?);
                    let name = stage_entry_pointers.insert(CString::new(stage.entry).unwrap());

                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::MESH_EXT)
                        .module(module.shader)
                        .name(&*name)
                }
                PipelineStage::Compute(stage) => {
                    let module = shader_modules.insert(create_pipeline_shader_module(
                        self,
                        &stage.shader,
                        descriptor.descriptors,
                    )?);
                    let name = stage_entry_pointers.insert(CString::new(stage.entry).unwrap());

                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::COMPUTE)
                        .module(module.shader)
                        .name(&*name)
                }
            };

            stages.push(vk_stage);
        }

        let res = match descriptor.stages[0].shader_stage() {
            ShaderStage::Compute => self.create_compute_pipeline(layout, stages[0]),
            _ => self.create_graphics_pipeline(descriptor, layout, &stages),
        };

        let pipeline = match res {
            Ok(pipeline) => pipeline,
            Err(err) => {
                unsafe {
                    self.device.destroy_pipeline_layout(layout, ALLOC);
                }

                return Err(err);
            }
        };

        // Shaders can be destroyed after the pipeline was created.
        drop(shader_modules);

        Ok(Pipeline {
            device: self.device.clone(),
            pipeline,
            pipeline_layout: layout,
            stages: descriptor.stages.iter().map(|v| v.shader_stage()).collect(),
        })
    }

    fn create_graphics_pipeline(
        &self,
        descriptor: &PipelineDescriptor<'_>,
        layout: vk::PipelineLayout,
        stages: &[vk::PipelineShaderStageCreateInfo<'_>],
    ) -> Result<vk::Pipeline, Error> {
        let mut color_attachment_formats = Vec::<vk::Format>::new();
        let mut color_blend_attachments = Vec::new();

        // Fill with some standard values. If the pipeline does not
        // include the stages with this data their values are ignored
        // by the driver.
        let mut primitive_topology = PrimitiveTopology::TriangleList;
        let mut raster_front_face = FrontFace::Ccw;
        let mut raster_cull_mode = None;
        let mut raster_depth_stencil_state = None;

        for stage in descriptor.stages {
            match stage {
                PipelineStage::Vertex(stage) => {
                    primitive_topology = stage.topology;
                }
                PipelineStage::Fragment(stage) => {
                    raster_front_face = stage.front_face;
                    raster_cull_mode = stage.cull_mode;
                    raster_depth_stencil_state = stage.depth_stencil_state;

                    for target in stage.targets {
                        let mut color_blend_state =
                            vk::PipelineColorBlendAttachmentState::default()
                                .color_write_mask(vk::ColorComponentFlags::RGBA)
                                .blend_enable(false);

                        if let Some(state) = target.blend {
                            color_blend_state = color_blend_state
                                .blend_enable(true)
                                .src_color_blend_factor(state.color_src_factor.into())
                                .dst_color_blend_factor(state.color_dst_factor.into())
                                .color_blend_op(state.color_op.into())
                                .src_alpha_blend_factor(state.alpha_src_factor.into())
                                .dst_alpha_blend_factor(state.alpha_dst_factor.into())
                                .alpha_blend_op(state.alpha_op.into());
                        }

                        color_attachment_formats.push(target.format.into());
                        color_blend_attachments.push(color_blend_state);
                    }
                }
                _ => {}
            }
        }

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default();

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(primitive_topology.into())
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

        let cull_mode = match raster_cull_mode {
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
            .front_face(raster_front_face.into());

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        let depth_stencil_state = raster_depth_stencil_state.as_ref().map(|state| {
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

        assert!(
            color_attachment_formats.len() <= self.device.limits.max_color_attachments as usize
        );
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats);

        if let Some(state) = &raster_depth_stencil_state {
            rendering_info = rendering_info.depth_attachment_format(state.format.into());
        }

        let mut info = vk::GraphicsPipelineCreateInfo::default()
            .flags(vk::PipelineCreateFlags::empty())
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            // RenderPass parameters, we use `DYNAMIC_RENDERING` instead.
            .render_pass(vk::RenderPass::null())
            .subpass(0)
            // Pipeline derivation parameters, we don't use those.
            .base_pipeline_handle(vk::Pipeline::null())
            .base_pipeline_index(0)
            // Dynamic rendering
            .push_next(&mut rendering_info);

        if let Some(depth_stencil_state) = &depth_stencil_state {
            info = info.depth_stencil_state(depth_stencil_state);
        }

        match unsafe {
            self.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], ALLOC)
        } {
            Ok(pipelines) => Ok(pipelines[0]),
            Err((pipelines, err)) => {
                debug_assert!(pipelines.is_empty());
                Err(err.into())
            }
        }
    }

    fn create_compute_pipeline(
        &self,
        layout: vk::PipelineLayout,
        stage: vk::PipelineShaderStageCreateInfo<'_>,
    ) -> Result<vk::Pipeline, Error> {
        let info = vk::ComputePipelineCreateInfo::default()
            .flags(vk::PipelineCreateFlags::empty())
            .stage(stage)
            .layout(layout)
            .base_pipeline_handle(vk::Pipeline::null())
            .base_pipeline_index(0);

        match unsafe {
            self.device
                .create_compute_pipelines(vk::PipelineCache::null(), &[info], ALLOC)
        } {
            Ok(pipelines) => Ok(pipelines[0]),
            Err((pipelines, err)) => {
                debug_assert!(pipelines.is_empty());
                Err(err.into())
            }
        }
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

        let pool = unsafe { self.device.create_command_pool(&info, ALLOC)? };

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
                    self.device.destroy_command_pool(pool, ALLOC);
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

        let semaphore = unsafe { self.device.create_semaphore(&info, ALLOC)? };

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
            (
                vk::DescriptorType::STORAGE_IMAGE,
                descriptor.max_storage_images,
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

        let pool = unsafe { self.device.create_descriptor_pool(&info, ALLOC)? };

        Ok(DescriptorPool {
            device: self.device.clone(),
            pool,
        })
    }

    /// Creates a new [`Fence`].
    pub fn create_fence(&self) -> Result<Fence, Error> {
        let info = vk::FenceCreateInfo::default();

        let fence = unsafe { self.device.create_fence(&info, ALLOC)? };
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

        let sampler = unsafe { self.device.create_sampler(&info, ALLOC)? };
        Ok(Sampler {
            device: self.device.clone(),
            sampler,
        })
    }

    pub fn create_query_pool(&self, descriptor: &QueryPoolDescriptor) -> Result<QueryPool, Error> {
        assert!(
            self.device.instance.config.performance_counters,
            "Device::create_query_pool requires Config::performance_counts to be enabled",
        );

        let info = vk::QueryPoolCreateInfo::default()
            .flags(vk::QueryPoolCreateFlags::empty())
            .query_type(descriptor.kind.into())
            .query_count(descriptor.count.get())
            // Not used for timestamp queries.
            .pipeline_statistics(vk::QueryPipelineStatisticFlags::empty());

        let pool = unsafe { self.device.create_query_pool(&info, ALLOC)? };

        // Seems like we need to reset before first use.
        unsafe {
            self.device
                .reset_query_pool(pool, 0, descriptor.count.get());
        }

        Ok(QueryPool {
            device: self.device.clone(),
            pool,
            query_count: descriptor.count,
        })
    }
}

#[derive(Debug)]
pub struct Queue {
    device: Arc<DeviceShared>,
    queue: vk::Queue,
    queue_family: QueueFamilyId,
    queue_index: u32,
    family: QueueFamily,
}

impl Queue {
    /// Returns the [`QueueFamily`] ID that was used to create this `Queue`.
    pub fn family(&self) -> QueueFamily {
        self.family
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

        let present_modes = &[config.present_mode.into()];
        let mut present_modes_info =
            vk::SwapchainPresentModesCreateInfoEXT::default().present_modes(present_modes);

        let mut info = vk::SwapchainCreateInfoKHR::default()
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

        if device.extensions().swapchain_maintenance1 {
            info = info.push_next(&mut present_modes_info);
        }

        let device = device.device.extensions.swapchain.as_ref().unwrap();
        let swapchain = unsafe { device.create_swapchain(&info, ALLOC)? };

        let images = match unsafe { device.get_swapchain_images(swapchain) } {
            Ok(images) => images,
            Err(err) => {
                // We will not return the new swapchain object from this function
                // on error. This means the newly created swapchain needs to be
                // destroyed manually, otherwise it will leak.
                unsafe {
                    device.destroy_swapchain(swapchain, ALLOC);
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

        // SAFETY: The surface could only have been created if the surface extension
        // exists.
        debug_assert!(self.instance.extensions.surface.is_some());
        let instance = unsafe { self.instance.extensions.surface.as_ref().unwrap_unchecked() };

        unsafe {
            instance.destroy_surface(self.surface, ALLOC);
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

        // SAFETY: The `create_surface` constructor that creates this `Surface`
        // already checks that the surface extension exists.
        // If the value was `Some(..)` at creation of this `Surface` it stays
        // `Some(..)` forever.
        debug_assert!(self.shared.instance.extensions.surface.is_some());
        let instance = unsafe {
            self.shared
                .instance
                .extensions
                .surface
                .as_ref()
                .unwrap_unchecked()
        };

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
        // This is the first time we create a swapchain.
        // Once we return a `Swapchain` we must ensure that the swapchain
        // extension is enabled.
        assert!(device.device.extensions.swapchain.is_some());

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
        // SAFETY: The `Swapchain` constructor guarantees that the swapchain extension
        // is enabled on the device. Enabled extensions cannot be disabled.
        debug_assert!(self.device.device.extensions.swapchain.is_some());
        let device = unsafe {
            self.device
                .device
                .extensions
                .swapchain
                .as_ref()
                .unwrap_unchecked()
        };

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
                        device.destroy_swapchain(self.swapchain, ALLOC);
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
                device.destroy_swapchain(self.swapchain, ALLOC);
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
    /// Note that there are no guarantees in which order the images are aquired.
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

        // SAFETY: The `Swapchain` constructor guarantees that the swapchain extension
        // is enabled on the device. Enabled extensions cannot be disabled.
        debug_assert!(self.device.device.extensions.swapchain.is_some());
        let device = unsafe {
            self.device
                .device
                .extensions
                .swapchain
                .as_ref()
                .unwrap_unchecked()
        };

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

        // `pImageIndex` is a valid index within the swapchain images,
        // as required by the spec.
        debug_assert!(image_index < self.images.len() as u32);

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

        // SAFETY: The `Swapchain` constructor guarantees that the swapchain extension
        // is enabled on the device. Enabled extensions cannot be disabled.
        debug_assert!(self.device.device.extensions.swapchain.is_some());
        let device = unsafe {
            self.device
                .device
                .extensions
                .swapchain
                .as_ref()
                .unwrap_unchecked()
        };

        unsafe {
            device.destroy_swapchain(self.swapchain, ALLOC);
        }
    }
}

macro_rules! vk_enum {
    ($t:ty => $vk_ty:ty, $($lhs:path => $rhs:path),*,) => {
        impl From<$t> for $vk_ty {
            fn from(value: $t) -> Self {
                match value {
                    $(
                        $lhs => $rhs,
                    )*
                }
            }
        }

        impl TryFrom<$vk_ty> for $t {
            type Error = UnknownEnumValue;

            fn try_from(value: $vk_ty) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $rhs => Ok($lhs),
                    )*
                    _ => Err(UnknownEnumValue)
                }
            }
        }
    };
}

vk_enum! {
    PresentMode => vk::PresentModeKHR,
    PresentMode::Fifo => vk::PresentModeKHR::FIFO,
    PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
    PresentMode::FifoRelaxed => vk::PresentModeKHR::FIFO_RELAXED,
    PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
}

vk_enum! {
    TextureFormat => vk::Format,
    TextureFormat::Rgb8Unorm => vk::Format::R8G8B8_UNORM,
    TextureFormat::Rgb8UnormSrgb => vk::Format::R8G8B8_SRGB,
    TextureFormat::Rgba8Unorm => vk::Format::R8G8B8A8_UNORM,
    TextureFormat::Rgba8UnormSrgb => vk::Format::R8G8B8A8_SRGB,
    TextureFormat::Bgra8Unorm => vk::Format::B8G8R8A8_UNORM,
    TextureFormat::Bgra8UnormSrgb => vk::Format::B8G8R8A8_SRGB,
    TextureFormat::Depth32Float => vk::Format::D32_SFLOAT,
    TextureFormat::Rgba16Float => vk::Format::R16G16B16A16_SFLOAT,
    TextureFormat::Bc1RgbaUnorm => vk::Format::BC1_RGBA_UNORM_BLOCK,
    TextureFormat::Bc1RgbaUnormSrgb => vk::Format::BC1_RGBA_SRGB_BLOCK,
    TextureFormat::Bc2RgbaUnorm => vk::Format::BC2_UNORM_BLOCK,
    TextureFormat::Bc2RgbaUnormSrgb => vk::Format::BC2_SRGB_BLOCK,
    TextureFormat::Bc3RgbaUnorm => vk::Format::BC3_UNORM_BLOCK,
    TextureFormat::Bc3RgbaUnormSrgb => vk::Format::BC3_SRGB_BLOCK,
    TextureFormat::Bc4RUnorm => vk::Format::BC4_UNORM_BLOCK,
    TextureFormat::Bc4RSnorm => vk::Format::BC4_SNORM_BLOCK,
    TextureFormat::Bc5RgUnorm => vk::Format::BC5_UNORM_BLOCK,
    TextureFormat::Bc5RgSnorm => vk::Format::BC5_SNORM_BLOCK,
    TextureFormat::Bc6HRgbUFloat => vk::Format::BC6H_UFLOAT_BLOCK,
    TextureFormat::Bc6HRgbSFloat => vk::Format::BC6H_SFLOAT_BLOCK,
    TextureFormat::Bc7RgbaUnorm => vk::Format::BC7_UNORM_BLOCK,
    TextureFormat::Bc7RgbaUnormSrgb => vk::Format::BC7_SRGB_BLOCK,
    TextureFormat::Rgb9E5Ufloat => vk::Format::E5B9G9R9_UFLOAT_PACK32,
    TextureFormat::R32Uint => vk::Format::R32_UINT,
    TextureFormat::R32Sint => vk::Format::R32_SINT,
    TextureFormat::R32SFloat => vk::Format::R32_SFLOAT,
    TextureFormat::Rg32Uint => vk::Format::R32G32_UINT,
    TextureFormat::Rg32Sint => vk::Format::R32G32_SINT,
    TextureFormat::Rg32SFloat => vk::Format::R32G32_SFLOAT,
}

vk_enum! {
    ColorSpace => vk::ColorSpaceKHR,
    ColorSpace::SrgbNonLinear => vk::ColorSpaceKHR::SRGB_NONLINEAR,
}

vk_enum! {
    PrimitiveTopology => vk::PrimitiveTopology,
    PrimitiveTopology::TriangleList => vk::PrimitiveTopology::TRIANGLE_LIST,
    PrimitiveTopology::LineList => vk::PrimitiveTopology::LINE_LIST,
    PrimitiveTopology::PointList => vk::PrimitiveTopology::POINT_LIST,
    PrimitiveTopology::LineStrip => vk::PrimitiveTopology::LINE_STRIP,
    PrimitiveTopology::TriangleStrip => vk::PrimitiveTopology::TRIANGLE_STRIP,
}

vk_enum! {
    FrontFace => vk::FrontFace,
    FrontFace::Cw => vk::FrontFace::CLOCKWISE,
    FrontFace::Ccw => vk::FrontFace::COUNTER_CLOCKWISE,
}

vk_enum! {
    DescriptorType => vk::DescriptorType,
    DescriptorType::Uniform => vk::DescriptorType::UNIFORM_BUFFER,
    DescriptorType::Storage => vk::DescriptorType::STORAGE_BUFFER,
    DescriptorType::Sampler => vk::DescriptorType::SAMPLER,
    DescriptorType::Texture => vk::DescriptorType::SAMPLED_IMAGE,
    DescriptorType::StorageTexture => vk::DescriptorType::STORAGE_IMAGE,
}

vk_enum! {
    TextureLayout => vk::ImageLayout,
    TextureLayout::Undefined => vk::ImageLayout::UNDEFINED,
    TextureLayout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    TextureLayout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
    TextureLayout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    TextureLayout::ShaderRead => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
}

vk_enum! {
    FilterMode => vk::Filter,
    FilterMode::Nearest => vk::Filter::NEAREST,
    FilterMode::Linear => vk::Filter::LINEAR,
}

impl From<FilterMode> for vk::SamplerMipmapMode {
    fn from(value: FilterMode) -> Self {
        match value {
            FilterMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            FilterMode::Linear => vk::SamplerMipmapMode::LINEAR,
        }
    }
}

vk_enum! {
    AddressMode => vk::SamplerAddressMode,
    AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
    AddressMode::MirrorRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
    AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
    AddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
}

vk_enum! {
    IndexFormat => vk::IndexType,
    IndexFormat::U16 => vk::IndexType::UINT16,
    IndexFormat::U32 => vk::IndexType::UINT32,
}

vk_enum! {
    CompareOp => vk::CompareOp,
    CompareOp::Never => vk::CompareOp::NEVER,
    CompareOp::Less => vk::CompareOp::LESS,
    CompareOp::LessEqual => vk::CompareOp::LESS_OR_EQUAL,
    CompareOp::Equal => vk::CompareOp::EQUAL,
    CompareOp::Greater => vk::CompareOp::GREATER,
    CompareOp::GreaterEqual => vk::CompareOp::GREATER_OR_EQUAL,
    CompareOp::Always => vk::CompareOp::ALWAYS,
    CompareOp::NotEqual => vk::CompareOp::NOT_EQUAL,
}

vk_enum! {
    BlendFactor => vk::BlendFactor,
    BlendFactor::Zero => vk::BlendFactor::ZERO,
    BlendFactor::One => vk::BlendFactor::ONE,
    BlendFactor::Src => vk::BlendFactor::SRC_COLOR,
    BlendFactor::OneMinusSrc => vk::BlendFactor::ONE_MINUS_SRC_COLOR,
    BlendFactor::SrcAlpha => vk::BlendFactor::SRC_ALPHA,
    BlendFactor::OneMinusSrcAlpha => vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
    BlendFactor::Dst => vk::BlendFactor::DST_COLOR,
    BlendFactor::OneMinusDst => vk::BlendFactor::ONE_MINUS_DST_COLOR,
    BlendFactor::DstAlpha => vk::BlendFactor::DST_ALPHA,
    BlendFactor::OneMinusDstAlpha => vk::BlendFactor::ONE_MINUS_DST_ALPHA,
}

vk_enum! {
    BlendOp => vk::BlendOp,
    BlendOp::Add => vk::BlendOp::ADD,
}

vk_enum! {
    QueryKind => vk::QueryType,
    QueryKind::Timestamp => vk::QueryType::TIMESTAMP,
}

impl From<ShaderStages> for vk::ShaderStageFlags {
    fn from(value: ShaderStages) -> Self {
        let mut flags = vk::ShaderStageFlags::empty();

        for flag in value.iter() {
            let vk_flag = match flag {
                ShaderStages::VERTEX => vk::ShaderStageFlags::VERTEX,
                ShaderStages::FRAGMENT => vk::ShaderStageFlags::FRAGMENT,
                ShaderStages::TASK => vk::ShaderStageFlags::TASK_EXT,
                ShaderStages::MESH => vk::ShaderStageFlags::MESH_EXT,
                ShaderStages::COMPUTE => vk::ShaderStageFlags::COMPUTE,
                _ => unreachable!(),
            };

            flags |= vk_flag;
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
        if value.contains(TextureUsage::STORAGE) {
            flags |= vk::ImageUsageFlags::STORAGE;
        }
        flags
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
            self.device.device.destroy_shader_module(self.shader, ALLOC);
        }
    }
}

#[derive(Debug)]
pub struct Pipeline {
    device: Arc<DeviceShared>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    stages: Vec<ShaderStage>,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.device.destroy_pipeline(self.pipeline, ALLOC);
            self.device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, ALLOC);
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
            // `ONE_TIME_SUBMIT` means we can only submit the final buffer
            // once. This is always given since `Queue::submit` takes the
            // `CommandBuffer` by value, guaranteeing that it will never be
            // used again.
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
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
            allocator: BumpAllocator::new(),
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
            self.device.device.destroy_command_pool(self.pool, ALLOC);
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
    allocator: BumpAllocator,
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
        tracing::trace!(
            "vkCopyBufferToBuffer(src={:p} dst={:p})",
            src.buffer,
            dst.buffer,
        );

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

        assert!(mip_level < dst.mip_levels);

        // Use the size of the selected mip level.
        // https://docs.vulkan.org/spec/latest/chapters/resources.html#resources-image-mip-level-sizing
        let dst_size = UVec2::max(UVec2::ONE, dst.size >> mip_level);

        let bytes_to_copy = src.layout.format.storage_size(dst_size) as u64;
        assert!(src.buffer.size > src.offset);
        assert!(src.buffer.size - src.offset >= bytes_to_copy);

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
            // We use a value of 0, which indicates that the source buffer is tightly packed.
            .buffer_row_length(0)
            //.buffer_row_length(0)
            // - `bufferImageHeight` must be 0, or greater than or equal to `height` of `imageExtent`.
            // We use a value of 0, which indicates that the source buffer is tightly packed.
            .buffer_image_height(0)
            //.buffer_image_height(0)
            .image_subresource(subresource)
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                // - `imageExtent.width` must not be 0.
                width: dst_size.x,
                // - `imageExtent.height` must not be 0.
                height: dst_size.y,
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

    /// Copies texels from a source [`Texture`] to a destination [`Texture`].
    ///
    /// # Safety
    ///
    /// - The `src` [`Texture`] must have only the [`TRANSFER_READ`] flag set at the time of operation.
    /// - The `dst` [`Texture`] must have only the [`TRANSFER_WRITE`] flag set at the time of operation.
    /// - Only one operation must write to `dst` before the write is flushed.
    ///
    /// [`TRANSFER_READ`]: AccessFlags::TRANSFER_READ
    /// [`TRANSFER_WRITE`]: AccessFlags::TRANSFER_WRITE
    pub unsafe fn copy_texture_to_texture(
        &mut self,
        src: &Texture,
        src_mip_level: u32,
        dst: &Texture,
        dst_mip_level: u32,
    ) {
        assert!(self.queue_caps.contains(QueueCapabilities::TRANSFER));

        assert_ne!(src.size.x, 0);
        assert_ne!(src.size.y, 0);
        assert!(src_mip_level < src.mip_levels);

        assert_ne!(dst.size.x, 0);
        assert_ne!(dst.size.y, 0);
        assert!(dst_mip_level < dst.mip_levels);

        // TODO: We only support copying the entire image for now
        // and as such both must be the same size.
        // Note that we must account for the mip levels.
        // https://docs.vulkan.org/spec/latest/chapters/resources.html#resources-image-mip-level-sizing
        let src_size = src.size >> src_mip_level;
        let dst_size = dst.size >> dst_mip_level;
        assert_eq!(src_size, dst_size);

        let src_aspect_mask = if src.format.is_depth() {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let src_subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(src_aspect_mask)
            .mip_level(src_mip_level)
            .base_array_layer(0)
            .layer_count(1);

        let dst_aspect_mask = if dst.format.is_depth() {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let dst_subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(dst_aspect_mask)
            .mip_level(dst_mip_level)
            .base_array_layer(0)
            .layer_count(1);

        let region = vk::ImageCopy2::default()
            .src_subresource(src_subresource)
            .src_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .dst_subresource(dst_subresource)
            .dst_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .extent(vk::Extent3D {
                // - `extent.width` must not be 0.
                width: dst.size.x,
                // - `extent.height` must not be 0.
                height: dst.size.y,
                // - `extent.depth` must not be 0.
                depth: 1,
            });

        let regions = &[region];
        let info = vk::CopyImageInfo2::default()
            .src_image(src.image)
            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .dst_image(dst.image)
            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .regions(regions);

        unsafe {
            self.device.device.cmd_copy_image2(self.buffer, &info);
        }
    }

    /// Clears all pixels in a texture to a value.
    ///
    /// # Safety
    ///
    /// - The passed [`Texture`] must have only the [`TRANSFER_WRITE`]  flag set at the time of
    ///   operation.
    ///
    /// [`TRANSFER_WRITE`]: AccessFlags::TRANSFER_WRITE
    pub unsafe fn clear_texture(&self, texture: &Texture, mip_level: u32, value: [u32; 4]) {
        assert!(self.queue_caps.contains(QueueCapabilities::TRANSFER));

        assert!(mip_level < texture.mip_levels);

        let value = vk::ClearColorValue { uint32: value };
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: mip_level,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        unsafe {
            self.device.cmd_clear_color_image(
                self.buffer,
                texture.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &value,
                &[subresource_range],
            );
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

    pub fn begin_compute_pass<'res>(&mut self) -> ComputePass<'_, 'res> {
        assert!(self.queue_caps.contains(QueueCapabilities::COMPUTE));
        ComputePass {
            encoder: self,
            pipeline: None,
        }
    }

    /// Inserts a batch of memory/execution barriers.
    ///
    /// # Safety
    ///
    /// For each [`BufferBarrier`] passed into this function all must be true:
    /// - The buffer must currently be in the `src_access` state.
    /// - The buffer range `offset+size` must be in bounds of the buffer.
    /// - No buffer ranges are passed in more than once.
    /// - The buffer range must not be empty.
    ///
    /// For each [`TextureBarrier`] passed into this function all must be true:
    /// - The texture must currently be in the `src_access` state.
    /// - The `base_mip_level+mip_levels` must be in bounds.
    /// - No mip level chain was passed in more than once.
    pub unsafe fn insert_pipeline_barriers(&mut self, barriers: &PipelineBarriers<'_>) {
        let alloc = self.allocator.span();

        let mut buffer_barriers = VecWithAlloc::with_capacity_in(barriers.buffer.len(), alloc);
        for barrier in barriers.buffer {
            debug_assert!(barrier.src_access.is_allowed_for_queue(&self.queue_caps));
            debug_assert!(barrier.dst_access.is_allowed_for_queue(&self.queue_caps));

            let src_access_flags = convert_access_flags(barrier.src_access);
            let dst_access_flags = convert_access_flags(barrier.dst_access);
            let src_stage_mask =
                access_flags_to_stage_mask(barrier.src_access, BarrierAccessScope::Source);
            let dst_stage_mask =
                access_flags_to_stage_mask(barrier.dst_access, BarrierAccessScope::Destination);

            // Guaranteed by caller.
            // - `offset` must be less than the size of `buffer`.
            // - `size` must not be 0.
            // - `size` must be less than or equal to the size of `buffer` minus `offset`.
            debug_assert_ne!(barrier.size, 0);
            debug_assert!(barrier.offset < barrier.buffer.size);
            debug_assert!(barrier.size <= barrier.buffer.size - barrier.offset);

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

            // SAFETY: We have preallocated the exact number of entries.
            unsafe {
                buffer_barriers.push_unchecked(barrier);
            }
        }

        let mut image_barriers = VecWithAlloc::with_capacity_in(barriers.texture.len(), alloc);
        for barrier in barriers.texture {
            // Guaranteed by caller.
            debug_assert!(barrier.src_access.is_allowed_for_queue(&self.queue_caps));
            debug_assert!(barrier.dst_access.is_allowed_for_queue(&self.queue_caps));

            let src_access_flags = convert_access_flags(barrier.src_access);
            let dst_access_flags = convert_access_flags(barrier.dst_access);
            let src_stage_mask =
                access_flags_to_stage_mask(barrier.src_access, BarrierAccessScope::Source);
            let dst_stage_mask =
                access_flags_to_stage_mask(barrier.dst_access, BarrierAccessScope::Destination);
            let old_layout = access_flags_to_image_layout(barrier.src_access);
            let new_layout = access_flags_to_image_layout(barrier.dst_access);

            let aspect_mask = if barrier.texture.format.is_depth() {
                vk::ImageAspectFlags::DEPTH
            } else {
                vk::ImageAspectFlags::COLOR
            };

            // Images cannot be transitioned into `UNDEFINED`.
            if new_layout == vk::ImageLayout::UNDEFINED {
                panic!(
                    "invalid image transition {:?}->{:?}: dst flags result in transition into UNDEFINED layout",
                    src_access_flags, dst_access_flags
                );
            }

            // Guaranteed by caller.
            debug_assert!(barrier.base_mip_level < barrier.texture.mip_levels);
            debug_assert!(
                barrier.base_mip_level + barrier.mip_levels <= barrier.texture.mip_levels
            );

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

            // SAFETY: We have preallocated the exact number of entries.
            unsafe {
                image_barriers.push_unchecked(barrier);
            }
        }

        let info = vk::DependencyInfo::default()
            .dependency_flags(vk::DependencyFlags::empty())
            .buffer_memory_barriers(&buffer_barriers)
            .image_memory_barriers(&image_barriers);

        unsafe {
            self.device.device.cmd_pipeline_barrier2(self.buffer, &info);
        }
    }

    pub unsafe fn write_timestamp_query(
        &self,
        pool: &QueryPool,
        index: u32,
        stage: TimestampPipelineStage,
    ) {
        assert!(index < pool.query_count.get());

        let stage = match stage {
            TimestampPipelineStage::None => vk::PipelineStageFlags2::NONE,
            TimestampPipelineStage::Graphics => vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            TimestampPipelineStage::Compute => vk::PipelineStageFlags2::COMPUTE_SHADER,
            TimestampPipelineStage::All => vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
        };

        unsafe {
            self.device
                .cmd_write_timestamp2(self.buffer, stage, pool.pool, index);
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

    /// Dispatches indirect draw commands from the given `buffer`.
    ///
    /// <https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdDrawIndirect.html>
    pub fn draw_indirect(&mut self, buffer: &Buffer, offset: u64, draw_count: u32, stride: u32) {
        assert!(buffer
            .usages
            .contains(vk::BufferUsageFlags::INDIRECT_BUFFER));

        assert!(offset <= buffer.size);
        assert!(offset + u64::from(draw_count) * u64::from(stride) <= buffer.size);

        unsafe {
            self.encoder.device.cmd_draw_indirect(
                self.encoder.buffer,
                buffer.buffer,
                offset,
                draw_count,
                stride,
            );
        }
    }

    /// <https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdDrawIndexedIndirect.html>
    pub fn draw_indexed_indirect(
        &mut self,
        buffer: &Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        assert!(buffer
            .usages
            .contains(vk::BufferUsageFlags::INDIRECT_BUFFER));

        assert!(offset <= buffer.size);
        assert!(offset + u64::from(draw_count) * u64::from(stride) <= buffer.size);

        unsafe {
            self.encoder.device.cmd_draw_indexed_indirect(
                self.encoder.buffer,
                buffer.buffer,
                offset,
                draw_count,
                stride,
            );
        }
    }

    /// Dispatches Mesh/Task shader workgroups.
    ///
    /// Requires `VK_EXT_mesh_shader`.
    ///
    /// See <https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdDrawMeshTasksEXT.html>
    pub fn draw_mesh_tasks(&mut self, x: u32, y: u32, z: u32) {
        let Some(device) = &self.encoder.device.extensions.mesh_shader else {
            panic!("VK_EXT_mesh_shader extension not enabled");
        };

        let pipeline = self.pipeline.as_ref().expect("Pipeline is not set");
        let total = x.checked_mul(y).unwrap().checked_mul(z).unwrap();
        match pipeline.stages[0] {
            ShaderStage::Task => {
                assert!(total <= self.encoder.device.limits.max_task_work_group_total_count);
                assert!(x <= self.encoder.device.limits.max_task_work_group_count[0]);
                assert!(y <= self.encoder.device.limits.max_task_work_group_count[0]);
                assert!(z <= self.encoder.device.limits.max_task_work_group_count[0]);
            }
            ShaderStage::Mesh => {
                assert!(total <= self.encoder.device.limits.max_mesh_work_group_total_count);
                assert!(x <= self.encoder.device.limits.max_mesh_work_group_count[0]);
                assert!(y <= self.encoder.device.limits.max_mesh_work_group_count[1]);
                assert!(z <= self.encoder.device.limits.max_mesh_work_group_count[2]);
            }
            stage => {
                panic!("Cannot use `RenderPass::draw_mesh_tasks` on {:?}", stage);
            }
        }

        unsafe {
            device.cmd_draw_mesh_tasks(self.encoder.buffer, x, y, z);
        }
    }

    /// See <https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdDrawMeshTasksIndirectEXT.html>
    pub fn draw_mesh_tasks_indirect(
        &mut self,
        buffer: &Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        let Some(device) = &self.encoder.device.extensions.mesh_shader else {
            panic!("VK_EXT_mesh_shader extension not enabled");
        };

        unsafe {
            device.cmd_draw_mesh_tasks_indirect(
                self.encoder.buffer,
                buffer.buffer,
                offset,
                draw_count,
                stride,
            );
        }
    }

    /// <https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdDrawMeshTasksIndirectCountEXT.html>
    pub fn draw_mesh_tasks_indirect_count(
        &mut self,
        buffer: &Buffer,
        offset: u64,
        count_buffer: &Buffer,
        count_buffer_offset: u64,
        max_draw_count: u32,
        stride: u32,
    ) {
        let Some(device) = &self.encoder.device.extensions.mesh_shader else {
            panic!("VK_EXT_mesh_shader extension not enabled");
        };

        unsafe {
            device.cmd_draw_mesh_tasks_indirect_count(
                self.encoder.buffer,
                buffer.buffer,
                offset,
                count_buffer.buffer,
                count_buffer_offset,
                max_draw_count,
                stride,
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

pub struct ComputePass<'encoder, 'resources> {
    encoder: &'encoder CommandEncoder<'encoder>,
    pipeline: Option<&'resources Pipeline>,
}

impl<'encoder, 'resources> ComputePass<'encoder, 'resources> {
    pub fn bind_pipeline(&mut self, pipeline: &'resources Pipeline) {
        unsafe {
            self.encoder.device.device.cmd_bind_pipeline(
                self.encoder.buffer,
                vk::PipelineBindPoint::COMPUTE,
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
                vk::PipelineBindPoint::COMPUTE,
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

        // We are in a compute pass, so `COMPUTE` is the only shader stage.
        assert_eq!(stages, ShaderStages::COMPUTE);

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

    /// Dispatches compute shader workgroups.
    ///
    /// <https://registry.khronos.org/vulkan/specs/latest/man/html/vkCmdDispatch.html>
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        let pipeline = self.pipeline.as_ref().expect("Pipeline is not set");
        match pipeline.stages[0] {
            ShaderStage::Compute => {
                assert!(x <= self.encoder.device.limits.max_compute_work_group_count[0]);
                assert!(y <= self.encoder.device.limits.max_compute_work_group_count[1]);
                assert!(z <= self.encoder.device.limits.max_compute_work_group_count[2])
            }
            stage => {
                panic!("Cannot use `RenderPass::dispatch` on {:?}", stage);
            }
        }

        unsafe {
            self.encoder
                .device
                .cmd_dispatch(self.encoder.buffer, x, y, z);
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
            self.device.device.destroy_semaphore(self.semaphore, ALLOC);
        }
    }
}

#[derive(Debug)]
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

    /// Returns the index of the texture in the swapchain.
    ///
    /// The index returned is guaranteed to be valid, i.e. be less than the number of swapchain
    /// images.
    pub fn index(&self) -> u32 {
        self.index
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
    /// Note that [`QueuePresent::signal`] is ignored if the `VK_EXT_swapchain_maintenance1`
    /// extension is not enabled.
    ///
    /// # Safety
    ///
    /// When the operation occurs the texture must have the [`PRESENT`] set.
    ///
    /// [`PRESENT`]: AccessFlags::PRESENT
    pub unsafe fn present(
        &mut self,
        queue: &mut Queue,
        cmd: QueuePresent<'_>,
    ) -> Result<(), Error> {
        // SAFETY: The `SwapchainTexture` constructor guarantees that the swapchain
        // extension is enabled on the device.
        debug_assert!(self.device.device.extensions.swapchain.is_some());
        let device = unsafe {
            self.device
                .device
                .extensions
                .swapchain
                .as_ref()
                .unwrap_unchecked()
        };

        let signal_fences = cmd.signal.map(|fence| {
            if self.device.extensions().swapchain_maintenance1 {
                fence.register();
            }

            [fence.fence]
        });
        let mut present_fence_info = signal_fences
            .as_ref()
            .map(|fences| vk::SwapchainPresentFenceInfoEXT::default().fences(fences));

        let wait_semaphores = &[cmd.wait.semaphore];

        let swapchains = &[self.swapchain.swapchain];
        let image_indices = &[self.index];
        let mut info = vk::PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            // - Every element must be unique.
            .swapchains(swapchains)
            // - Every image must be in `VK_IMAGE_LAYOUT_PRESENT_SRC_KHR` once this is executed.
            .image_indices(image_indices);

        // Present fence must only be present if `VK_EXT_swapchain_maintenance1` is enabled.
        // We ignore the parameter if the extnesion is not present.
        if self
            .device
            .device
            .extensions
            .swapchain_maintenance1
            .is_some()
        {
            if let Some(present_fence_info) = &mut present_fence_info {
                info = info.push_next(present_fence_info);
            }
        } else if cfg!(debug_assertions) && present_fence_info.is_some() {
            tracing::warn!(
                "QueuePresent::signal exists, but VK_EXT_swapchain_maintenance1 is not enabled"
            );
        }

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

        let view = unsafe { self.device.device.create_image_view(&info, ALLOC).unwrap() };
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
                self.device.device.destroy_image(self.image, ALLOC);
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
            self.device.device.destroy_image_view(self.view, ALLOC);
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
            self.device.destroy_buffer(self.buffer, ALLOC);
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

        // We must ensure that all stores become globally visible before we call
        // vkFlushMappedMemoryRanges, including for non-temporal stores, for which
        // a Release fence is to weak.
        // A SeqCst fence generates MFENCE with includes the operations of SFENCE
        // of making all stores, including non-temporal stores visible.
        // This step is explicitly required by the Vulkan spec.
        fence(Ordering::SeqCst);

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
            self.device.free_memory(self.memory, ALLOC);
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
            self.device
                .destroy_descriptor_set_layout(self.layout, ALLOC);
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
            self.device.destroy_descriptor_pool(self.pool, ALLOC);
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

        for binding in op.bindings {
            let Some(layout_binding) = self.bindings.get(binding.binding as usize) else {
                panic!(
                    "attempted to write to index {} of descriptor set with layout of {} elements",
                    binding.binding,
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
                WriteDescriptorResource::StorageTexture(textures) => {
                    (DescriptorType::StorageTexture, textures.len())
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
                binding.binding, kind, layout_binding.kind,
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
                WriteDescriptorResource::StorageTexture(textures) => {
                    for texture in textures {
                        let info = vk::DescriptorImageInfo::default()
                            .image_view(texture.view)
                            // TODO: Use a more suitable format than GENERAL
                            // depending on usage.
                            .image_layout(vk::ImageLayout::GENERAL)
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
                DescriptorType::StorageTexture => vk::DescriptorType::STORAGE_IMAGE,
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

        // SAFETY:
        // Since we only allow one submission to register this fence, we
        // can be sure that after the fence was signaled it will not become
        // signaled again.
        unsafe {
            self.reset();
        }

        Ok(true)
    }

    /// Returns the current status of this `Fence`.
    ///
    /// If the fence was signaled, this function will return `true`, it will return `false`.
    pub fn status(&self) -> Result<bool, Error> {
        unsafe {
            self.device
                .get_fence_status(self.fence)
                .map_err(|e| e.into())
        }
    }

    /// Resets this `Fence`.
    ///
    /// # Safety
    ///
    /// All queue operations that referenced this `Fence` must be complete.
    pub unsafe fn reset(&mut self) {
        self.state = FenceState::Idle;
        unsafe {
            self.device.reset_fences(&[self.fence]).unwrap();
        }
    }

    #[track_caller]
    fn register(&mut self) {
        assert_eq!(self.state, FenceState::Idle);
        self.state = FenceState::Waiting;
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.destroy_fence(self.fence, ALLOC);
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
            self.device.destroy_sampler(self.sampler, ALLOC);
        }
    }
}

#[derive(Debug)]
pub struct QueryPool {
    device: Arc<DeviceShared>,
    pool: vk::QueryPool,
    query_count: NonZeroU32,
}

impl QueryPool {
    /// Resets all queries in this pool.
    ///
    /// # Safety
    ///
    /// All submissions that write to this `QueryPool` must have completed.
    pub unsafe fn reset(&self) {
        unsafe {
            self.device
                .reset_query_pool(self.pool, 0, self.query_count.get());
        }
    }

    pub unsafe fn get(&self, offset: u32, count: u32) -> Result<Vec<u64>, Error> {
        assert!(offset + count <= self.query_count.get());

        let mut data = vec![0; count as usize];

        unsafe {
            self.device.get_query_pool_results::<u64>(
                self.pool,
                offset,
                &mut data,
                vk::QueryResultFlags::TYPE_64,
            )?;
        }

        Ok(data)
    }
}

impl Drop for QueryPool {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }

        unsafe {
            self.device.destroy_query_pool(self.pool, ALLOC);
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
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            tracing::info!("[{:?}]: {}", typ, message);
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
    extensions: InstanceExtensionFns,
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
            let instance = self.extensions.debug_utils.as_ref().unwrap();

            unsafe {
                instance.destroy_debug_utils_messenger(messenger, ALLOC);
            }
        }

        unsafe {
            self.instance.destroy_instance(ALLOC);
        }
    }
}

/// Instance extension function pointers.
#[derive(Clone)]
struct InstanceExtensionFns {
    surface: Option<ash::khr::surface::Instance>,
    #[cfg(all(unix, feature = "wayland"))]
    surface_wayland: Option<ash::khr::wayland_surface::Instance>,
    #[cfg(all(unix, feature = "x11"))]
    surface_xcb: Option<ash::khr::xcb_surface::Instance>,
    #[cfg(all(unix, feature = "x11"))]
    surface_xlib: Option<ash::khr::xlib_surface::Instance>,
    #[cfg(windows)]
    surface_win32: Option<ash::khr::win32_surface::Instance>,
    surface_maintenance1: Option<()>,
    get_surface_capabilities2: Option<()>,
    debug_utils: Option<ash::ext::debug_utils::Instance>,
}

impl InstanceExtensionFns {
    fn new(
        entry: &Entry,
        instance: &ash::Instance,
        supported_extensions: &InstanceExtensions,
    ) -> Self {
        Self {
            surface: supported_extensions
                .surface
                .then(|| ash::khr::surface::Instance::new(entry, instance)),
            #[cfg(all(unix, feature = "wayland"))]
            surface_wayland: supported_extensions
                .surface_wayland
                .then(|| ash::khr::wayland_surface::Instance::new(entry, instance)),
            #[cfg(all(unix, feature = "x11"))]
            surface_xcb: supported_extensions
                .surface_xcb
                .then(|| ash::khr::xcb_surface::Instance::new(entry, instance)),
            #[cfg(all(unix, feature = "x11"))]
            surface_xlib: supported_extensions
                .surface_xlib
                .then(|| ash::khr::xlib_surface::Instance::new(entry, instance)),
            #[cfg(windows)]
            surface_win32: supported_extensions
                .surface_win32
                .then(|| ash::khr::win32_surface::Instance::new(entry, instance)),
            surface_maintenance1: supported_extensions.surface_maintenance1.then(|| ()),
            get_surface_capabilities2: supported_extensions.get_surface_capabilities2.then(|| ()),
            debug_utils: supported_extensions
                .debug_utils
                .then(|| ash::ext::debug_utils::Instance::new(entry, instance)),
        }
    }
}

#[derive(Clone)]
struct DeviceShared {
    instance: Arc<InstanceShared>,
    device: ash::Device,
    limits: DeviceLimits,
    extensions: DeviceExtensionFns,
    memory_properties: AdapterMemoryProperties,
    /// Number of currently active allocations.
    num_allocations: Arc<AtomicU32>,
    queues: Arc<[QueueSlot]>,
    queue_families: Vec<QueueFamily>,
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
            self.device.destroy_device(ALLOC);
        }
    }
}

/// Device extension function pointers.
#[derive(Clone)]
struct DeviceExtensionFns {
    swapchain: Option<ash::khr::swapchain::Device>,
    swapchain_maintenance1: Option<()>,
    mesh_shader: Option<ash::ext::mesh_shader::Device>,
}

impl DeviceExtensionFns {
    fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        supported_extensions: &DeviceExtensions,
    ) -> Self {
        Self {
            swapchain: supported_extensions
                .swapchain
                .then(|| ash::khr::swapchain::Device::new(instance, device)),
            swapchain_maintenance1: supported_extensions.swapchain_maintenance1.then(|| ()),
            mesh_shader: supported_extensions
                .mesh_shader
                .then(|| ash::ext::mesh_shader::Device::new(instance, device)),
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
pub struct DeviceLimits {
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
    max_descriptor_set_storage_images: u32,
    max_color_attachments: u32,
    // VkPhysicalDeviceMaintenance3Properties
    max_per_set_descriptors: u32,
    max_memory_allocation_size: u64,
    max_task_work_group_total_count: u32,
    max_task_work_group_count: [u32; 3],
    max_task_work_group_invocations: u32,
    max_task_work_group_size: [u32; 3],
    max_task_payload_size: u32,
    max_mesh_work_group_total_count: u32,
    max_mesh_work_group_count: [u32; 3],
    max_mesh_work_group_invocations: u32,
    max_mesh_work_group_size: [u32; 3],
    max_mesh_output_vertices: u32,
    max_mesh_output_primitives: u32,
    max_compute_work_group_count: [u32; 3],
    max_compute_work_group_invocations: u32,
    max_compute_work_group_size: [u32; 3],
    pub timestamp_period_nanos: f32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct DeviceExtensions {
    /// `VK_KHR_swapchain`
    pub swapchain: bool,
    /// `VK_EXT_swapchain_maintenance1`
    pub swapchain_maintenance1: bool,
    /// `VK_EXT_mesh_shader`
    pub mesh_shader: bool,
}

impl DeviceExtensions {
    const SWAPCHAIN_MAINTENANCE1: &CStr = ash::ext::swapchain_maintenance1::NAME;
    const SWAPCHAIN: &CStr = ash::khr::swapchain::NAME;
    const MESH_SAHDER: &CStr = ash::ext::mesh_shader::NAME;

    fn names(&self) -> Vec<&'static CStr> {
        let mut names = Vec::new();

        for (enabled, name) in [
            (self.swapchain_maintenance1, Self::SWAPCHAIN_MAINTENANCE1),
            (self.swapchain, Self::SWAPCHAIN),
            (self.mesh_shader, Self::MESH_SAHDER),
        ] {
            if enabled {
                names.push(name);
            }
        }

        names
    }
}

impl<'a> FromIterator<&'a CStr> for DeviceExtensions {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = &'a CStr>,
    {
        let mut extensions = Self::default();
        for name in iter {
            match name {
                name if name == Self::SWAPCHAIN_MAINTENANCE1 => {
                    extensions.swapchain_maintenance1 = true
                }
                name if name == Self::SWAPCHAIN => extensions.swapchain = true,
                name if name == Self::MESH_SAHDER => extensions.mesh_shader = true,
                _ => (),
            }
        }
        extensions
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeviceFeatures {
    /// Vulkan 1.0
    mutli_draw_indirect: bool,
    /// Vulkan 1.1 or `VK_KHR_16bit_storage`
    storage_buffer_16bit_access: bool,
    /// Vulkan 1.1 or `VK_KHR_shader_draw_parameters`
    shader_draw_parameters: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_input_attachment_array_dynamic_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_uniform_texel_buffer_array_dynamic_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_storage_texel_buffer_array_dynamic_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_uniform_buffer_array_non_uniform_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_sampled_image_array_non_uniform_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_storage_buffer_array_non_uniform_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_storage_image_array_non_uniform_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_input_attachment_array_non_uniform_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_uniform_texel_buffer_array_non_uniform_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    shader_storage_texel_buffer_array_non_uniform_indexing: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_uniform_buffer_update_after_bind: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_sampled_image_update_after_bind: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_storage_image_update_after_bind: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_storage_buffer_update_after_bind: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_uniform_texel_buffer_update_after_bind: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_storage_texel_buffer_update_after_bind: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_update_unused_while_pending: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_partially_bound: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    descriptor_binding_variable_descriptor_count: bool,
    /// Vulkan 1.2 or `VK_EXT_descriptor_indexing`
    runtime_descriptor_array: bool,
    /// Vulkan 1.2 or `VK_KHR_8bit_storage`
    storage_buffer_8bit_access: bool,
    /// Vulkan 1.2 or `VK_KHR_8bit_storage`
    uniform_and_storage_buffer_8bit_access: bool,
    /// Vulkan 1.2 or `VK_KHR_8bit_storage`
    storage_push_constant8: bool,
    /// Vulkan 1.2 or `VK_KHR_shader_float16_int8`
    shader_float16: bool,
    /// Vulkan 1.2 or `VK_KHR_shader_float16_int8`
    shader_int8: bool,
    /// Vulkan 1.2 or `VK_EXT_host_query_reset`
    host_query_reset: bool,
    /// Vulkan 1.3 or `VK_KHR_dynamic_rendering`
    dynamic_rendering: bool,
    /// Vulkan 1.3 or `VK_KHR_synchronization2`
    synchronization2: bool,
    /// `VK_EXT_swapchain_maintenance1`
    swapchain_maintenace1: bool,
    /// `VK_EXT_mesh_shader`
    task_shader: bool,
    /// `VK_EXT_mesh_shader`
    mesh_shader: bool,
}

impl DeviceFeatures {
    /// Returns `Ok()` when `self` contains all flags that are set in `features`.
    fn validate_requirements(&self, required: Self) -> Result<(), Error> {
        let mut missing_features = Self::default();

        let Self {
            mutli_draw_indirect,
            storage_buffer_16bit_access,
            shader_draw_parameters,
            shader_input_attachment_array_dynamic_indexing,
            shader_uniform_texel_buffer_array_dynamic_indexing,
            shader_storage_texel_buffer_array_dynamic_indexing,
            shader_uniform_buffer_array_non_uniform_indexing,
            shader_sampled_image_array_non_uniform_indexing,
            shader_storage_buffer_array_non_uniform_indexing,
            shader_storage_image_array_non_uniform_indexing,
            shader_input_attachment_array_non_uniform_indexing,
            shader_uniform_texel_buffer_array_non_uniform_indexing,
            shader_storage_texel_buffer_array_non_uniform_indexing,
            descriptor_binding_uniform_buffer_update_after_bind,
            descriptor_binding_sampled_image_update_after_bind,
            descriptor_binding_storage_image_update_after_bind,
            descriptor_binding_storage_buffer_update_after_bind,
            descriptor_binding_uniform_texel_buffer_update_after_bind,
            descriptor_binding_storage_texel_buffer_update_after_bind,
            descriptor_binding_update_unused_while_pending,
            descriptor_binding_partially_bound,
            descriptor_binding_variable_descriptor_count,
            runtime_descriptor_array,
            storage_buffer_8bit_access,
            uniform_and_storage_buffer_8bit_access,
            storage_push_constant8,
            shader_float16,
            shader_int8,
            host_query_reset,
            dynamic_rendering,
            synchronization2,
            swapchain_maintenace1,
            task_shader,
            mesh_shader,
        } = self;

        macro_rules! set_missing_feature_flags {
            ($($name:ident),*,) => {
                $(
                    missing_features.$name = required.$name && !$name;
                )*
            };
        }

        set_missing_feature_flags! {
            mutli_draw_indirect,
            storage_buffer_16bit_access,
            shader_draw_parameters,
            shader_input_attachment_array_dynamic_indexing,
            shader_uniform_texel_buffer_array_dynamic_indexing,
            shader_storage_texel_buffer_array_dynamic_indexing,
            shader_uniform_buffer_array_non_uniform_indexing,
            shader_sampled_image_array_non_uniform_indexing,
            shader_storage_buffer_array_non_uniform_indexing,
            shader_storage_image_array_non_uniform_indexing,
            shader_input_attachment_array_non_uniform_indexing,
            shader_uniform_texel_buffer_array_non_uniform_indexing,
            shader_storage_texel_buffer_array_non_uniform_indexing,
            descriptor_binding_uniform_buffer_update_after_bind,
            descriptor_binding_sampled_image_update_after_bind,
            descriptor_binding_storage_image_update_after_bind,
            descriptor_binding_storage_buffer_update_after_bind,
            descriptor_binding_uniform_texel_buffer_update_after_bind,
            descriptor_binding_storage_texel_buffer_update_after_bind,
            descriptor_binding_update_unused_while_pending,
            descriptor_binding_partially_bound,
            descriptor_binding_variable_descriptor_count,
            runtime_descriptor_array,
            storage_buffer_8bit_access,
            uniform_and_storage_buffer_8bit_access,
            storage_push_constant8,
            shader_float16,
            shader_int8,
            host_query_reset,
            dynamic_rendering,
            synchronization2,
            swapchain_maintenace1,
            task_shader,
            mesh_shader,
        }

        if missing_features.is_empty() {
            Ok(())
        } else {
            Err(Error::MissingFeatures(missing_features))
        }
    }

    fn is_empty(&self) -> bool {
        let Self {
            mutli_draw_indirect,
            storage_buffer_16bit_access,
            shader_draw_parameters,
            shader_input_attachment_array_dynamic_indexing,
            shader_uniform_texel_buffer_array_dynamic_indexing,
            shader_storage_texel_buffer_array_dynamic_indexing,
            shader_uniform_buffer_array_non_uniform_indexing,
            shader_sampled_image_array_non_uniform_indexing,
            shader_storage_buffer_array_non_uniform_indexing,
            shader_storage_image_array_non_uniform_indexing,
            shader_input_attachment_array_non_uniform_indexing,
            shader_uniform_texel_buffer_array_non_uniform_indexing,
            shader_storage_texel_buffer_array_non_uniform_indexing,
            descriptor_binding_uniform_buffer_update_after_bind,
            descriptor_binding_sampled_image_update_after_bind,
            descriptor_binding_storage_image_update_after_bind,
            descriptor_binding_storage_buffer_update_after_bind,
            descriptor_binding_uniform_texel_buffer_update_after_bind,
            descriptor_binding_storage_texel_buffer_update_after_bind,
            descriptor_binding_update_unused_while_pending,
            descriptor_binding_partially_bound,
            descriptor_binding_variable_descriptor_count,
            runtime_descriptor_array,
            storage_buffer_8bit_access,
            uniform_and_storage_buffer_8bit_access,
            storage_push_constant8,
            shader_float16,
            shader_int8,
            host_query_reset,
            dynamic_rendering,
            synchronization2,
            swapchain_maintenace1,
            task_shader,
            mesh_shader,
        } = *self;

        let is_not_empty = mutli_draw_indirect
            || storage_buffer_16bit_access
            || shader_draw_parameters
            || shader_input_attachment_array_dynamic_indexing
            || shader_uniform_texel_buffer_array_dynamic_indexing
            || shader_storage_texel_buffer_array_dynamic_indexing
            || shader_uniform_buffer_array_non_uniform_indexing
            || shader_sampled_image_array_non_uniform_indexing
            || shader_storage_buffer_array_non_uniform_indexing
            || shader_storage_image_array_non_uniform_indexing
            || shader_input_attachment_array_non_uniform_indexing
            || shader_uniform_texel_buffer_array_non_uniform_indexing
            || shader_storage_texel_buffer_array_non_uniform_indexing
            || descriptor_binding_uniform_buffer_update_after_bind
            || descriptor_binding_sampled_image_update_after_bind
            || descriptor_binding_storage_image_update_after_bind
            || descriptor_binding_storage_buffer_update_after_bind
            || descriptor_binding_uniform_texel_buffer_update_after_bind
            || descriptor_binding_storage_texel_buffer_update_after_bind
            || descriptor_binding_update_unused_while_pending
            || descriptor_binding_partially_bound
            || descriptor_binding_variable_descriptor_count
            || runtime_descriptor_array
            || storage_buffer_8bit_access
            || uniform_and_storage_buffer_8bit_access
            || storage_push_constant8
            || shader_float16
            || shader_int8
            || host_query_reset
            || dynamic_rendering
            || synchronization2
            || swapchain_maintenace1
            || task_shader
            || mesh_shader;

        !is_not_empty
    }
}

impl Display for DeviceFeatures {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Self {
            storage_buffer_16bit_access,
            mutli_draw_indirect,
            shader_draw_parameters,
            shader_input_attachment_array_dynamic_indexing,
            shader_uniform_texel_buffer_array_dynamic_indexing,
            shader_storage_texel_buffer_array_dynamic_indexing,
            shader_uniform_buffer_array_non_uniform_indexing,
            shader_sampled_image_array_non_uniform_indexing,
            shader_storage_buffer_array_non_uniform_indexing,
            shader_storage_image_array_non_uniform_indexing,
            shader_input_attachment_array_non_uniform_indexing,
            shader_uniform_texel_buffer_array_non_uniform_indexing,
            shader_storage_texel_buffer_array_non_uniform_indexing,
            descriptor_binding_uniform_buffer_update_after_bind,
            descriptor_binding_sampled_image_update_after_bind,
            descriptor_binding_storage_image_update_after_bind,
            descriptor_binding_storage_buffer_update_after_bind,
            descriptor_binding_uniform_texel_buffer_update_after_bind,
            descriptor_binding_storage_texel_buffer_update_after_bind,
            descriptor_binding_update_unused_while_pending,
            descriptor_binding_partially_bound,
            descriptor_binding_variable_descriptor_count,
            runtime_descriptor_array,
            storage_buffer_8bit_access,
            uniform_and_storage_buffer_8bit_access,
            storage_push_constant8,
            shader_float16,
            shader_int8,
            host_query_reset,
            dynamic_rendering,
            synchronization2,
            swapchain_maintenace1,
            task_shader,
            mesh_shader,
        } = *self;

        macro_rules! create_strings {
            ($($name:ident),*,) => {{
                let mut strings = Vec::new();

                $(
                    if $name {
                        strings.push(stringify!($name));
                    }
                )*

                strings
            }};
        }

        let strings = create_strings! {
            storage_buffer_16bit_access,
            mutli_draw_indirect,
            shader_draw_parameters,
            shader_input_attachment_array_dynamic_indexing,
            shader_uniform_texel_buffer_array_dynamic_indexing,
            shader_storage_texel_buffer_array_dynamic_indexing,
            shader_uniform_buffer_array_non_uniform_indexing,
            shader_sampled_image_array_non_uniform_indexing,
            shader_storage_buffer_array_non_uniform_indexing,
            shader_storage_image_array_non_uniform_indexing,
            shader_input_attachment_array_non_uniform_indexing,
            shader_uniform_texel_buffer_array_non_uniform_indexing,
            shader_storage_texel_buffer_array_non_uniform_indexing,
            descriptor_binding_uniform_buffer_update_after_bind,
            descriptor_binding_sampled_image_update_after_bind,
            descriptor_binding_storage_image_update_after_bind,
            descriptor_binding_storage_buffer_update_after_bind,
            descriptor_binding_uniform_texel_buffer_update_after_bind,
            descriptor_binding_storage_texel_buffer_update_after_bind,
            descriptor_binding_update_unused_while_pending,
            descriptor_binding_partially_bound,
            descriptor_binding_variable_descriptor_count,
            runtime_descriptor_array,
            storage_buffer_8bit_access,
            uniform_and_storage_buffer_8bit_access,
            storage_push_constant8,
            shader_float16,
            shader_int8,
            host_query_reset,
            dynamic_rendering,
            synchronization2,
            swapchain_maintenace1,
            task_shader,
            mesh_shader,
        };

        f.write_str(&strings.join(", "))
    }
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

fn convert_access_flags(mut flags: AccessFlags) -> vk::AccessFlags2 {
    let mut access = vk::AccessFlags2::empty();

    let mut map_flags = |src, dst| {
        if flags.intersects(src) {
            access |= dst;
            // Every invocation should contain unique flags so
            // that by removing those flags we only remain with
            // the flags that have not been mapped at the end.
            // This is then used as an extra assertion to make sure
            // we forgot no flags.
            // This operation is optimized out in release.
            flags &= !src;
        }
    };

    map_flags(AccessFlags::TRANSFER_READ, vk::AccessFlags2::TRANSFER_READ);
    map_flags(
        AccessFlags::TRANSFER_WRITE,
        vk::AccessFlags2::TRANSFER_WRITE,
    );
    map_flags(
        AccessFlags::VERTEX_SHADER_READ
            | AccessFlags::FRAGMENT_SHADER_READ
            | AccessFlags::TASK_SHADER_READ
            | AccessFlags::MESH_SHADER_READ
            | AccessFlags::COMPUTE_SHADER_READ,
        vk::AccessFlags2::SHADER_READ,
    );
    map_flags(
        AccessFlags::VERTEX_SHADER_WRITE
            | AccessFlags::FRAGMENT_SHADER_WRITE
            | AccessFlags::TASK_SHADER_WRITE
            | AccessFlags::MESH_SHADER_WRITE
            | AccessFlags::COMPUTE_SHADER_WRITE,
        vk::AccessFlags2::SHADER_WRITE,
    );
    map_flags(
        AccessFlags::COLOR_ATTACHMENT_READ,
        vk::AccessFlags2::COLOR_ATTACHMENT_READ,
    );
    map_flags(
        AccessFlags::COLOR_ATTACHMENT_WRITE,
        vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
    );
    map_flags(
        AccessFlags::DEPTH_ATTACHMENT_READ,
        vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ,
    );
    map_flags(
        AccessFlags::DEPTH_ATTACHMENT_WRITE,
        vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
    );
    map_flags(AccessFlags::INDEX, vk::AccessFlags2::INDEX_READ);
    map_flags(
        AccessFlags::INDIRECT,
        vk::AccessFlags2::INDIRECT_COMMAND_READ,
    );

    map_flags(AccessFlags::PRESENT, vk::AccessFlags2::empty());

    debug_assert!(flags.is_empty(), "unhandled AccessFlags: {:?}", flags);

    access
}

fn access_flags_to_image_layout(flags: AccessFlags) -> vk::ImageLayout {
    // If the bitset is not empty the difference between contains
    // the bits that are set in the source bitset, but not set in
    // the target bitset.
    // We check if the bitset is empty beforehand, so if the difference
    // at any point is zero we know that all source bits were a subset
    // of the checked flags.
    let matches_any = |src| flags.difference(src).is_empty();

    if flags.is_empty() {
        vk::ImageLayout::UNDEFINED
    } else if matches_any(AccessFlags::TRANSFER_READ) {
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL
    } else if matches_any(AccessFlags::TRANSFER_WRITE) {
        vk::ImageLayout::TRANSFER_DST_OPTIMAL
    } else if matches_any(AccessFlags::DEPTH_ATTACHMENT_READ) {
        vk::ImageLayout::DEPTH_READ_ONLY_OPTIMAL
    } else if matches_any(AccessFlags::PRESENT) {
        debug_assert!(
            flags == AccessFlags::PRESENT,
            "PRESENT is mutually exclusive with all other flags"
        );

        vk::ImageLayout::PRESENT_SRC_KHR
    } else if matches_any(
        AccessFlags::VERTEX_SHADER_READ
            | AccessFlags::FRAGMENT_SHADER_READ
            | AccessFlags::TASK_SHADER_READ
            | AccessFlags::MESH_SHADER_READ
            | AccessFlags::COMPUTE_SHADER_READ,
    ) {
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
    } else if matches_any(AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE)
    {
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
    } else if matches_any(AccessFlags::DEPTH_ATTACHMENT_READ | AccessFlags::DEPTH_ATTACHMENT_WRITE)
    {
        vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL
    } else {
        debug_assert!(
            !flags.contains(AccessFlags::INDEX),
            "images cannot have the INDEX flag"
        );
        debug_assert!(
            !flags.contains(AccessFlags::INDIRECT),
            "images cannot have the INDIRECT flag",
        );

        vk::ImageLayout::GENERAL
    }
}

#[derive(Copy, Clone, Debug)]
enum BarrierAccessScope {
    Source,
    Destination,
}

fn access_flags_to_stage_mask(
    flags: AccessFlags,
    scope: BarrierAccessScope,
) -> vk::PipelineStageFlags2 {
    // See https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#synchronization-pipeline-stages-order
    // for ordered list of pipeline stages.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum GraphicsPrimitiveStage {
        DrawIndirect,
        // PRIMITIVE PIPELINE
        VertexInput,
        // PRIMITIVE PIPELINE
        VertexShader,
        // MESH PIPELINE
        TaskShader,
        // MESH PIPELINE
        MeshShader,
        EarlyFragmentTests,
        FragmentShader,
        LateFragmentTests,
        ColorAttachmentOutput,
        None,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum ComputeStage {
        Compute,
        None,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum TransferStage {
        Transfer,
        None,
    }

    let mut transfer = TransferStage::None;
    if flags.intersects(AccessFlags::TRANSFER_READ | AccessFlags::TRANSFER_WRITE) {
        transfer = TransferStage::Transfer;
    }

    let mut compute = ComputeStage::None;
    if flags.intersects(AccessFlags::COMPUTE_SHADER_READ | AccessFlags::COMPUTE_SHADER_WRITE) {
        compute = ComputeStage::Compute;
    }

    // See https://registry.khronos.org/vulkan/specs/latest/man/html/VkAccessFlagBits2.html
    // for which accesses map to which pipeline stages.
    let mut graphics = GraphicsPrimitiveStage::None;

    for (flag, stage) in [
        (AccessFlags::INDIRECT, GraphicsPrimitiveStage::DrawIndirect),
        (AccessFlags::INDEX, GraphicsPrimitiveStage::VertexInput),
        (
            AccessFlags::VERTEX_SHADER_READ | AccessFlags::VERTEX_SHADER_WRITE,
            GraphicsPrimitiveStage::VertexShader,
        ),
        (
            AccessFlags::FRAGMENT_SHADER_READ | AccessFlags::FRAGMENT_SHADER_WRITE,
            GraphicsPrimitiveStage::FragmentShader,
        ),
        (
            // Even LOAD ops for color attachments happen at the COLOR_ATTACHMENT_WRITE stage.
            // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#renderpass-load-operations
            AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE,
            GraphicsPrimitiveStage::ColorAttachmentOutput,
        ),
        (
            AccessFlags::TASK_SHADER_READ | AccessFlags::TASK_SHADER_WRITE,
            GraphicsPrimitiveStage::TaskShader,
        ),
        (
            AccessFlags::MESH_SHADER_READ | AccessFlags::MESH_SHADER_WRITE,
            GraphicsPrimitiveStage::MeshShader,
        ),
    ] {
        if flags.intersects(flag) {
            graphics = core::cmp::min(graphics, stage);
        }
    }

    if flags.intersects(AccessFlags::DEPTH_ATTACHMENT_READ | AccessFlags::DEPTH_ATTACHMENT_WRITE) {
        // Accessing depth buffers may happen in both the EARLY_FRAGMENT_TESTS and
        // LATE_FRAGMENT_TESTS stage.
        // This means flushing must happen in the LATE_FRAGMENT_TESTS stage and
        // invalidation must happen in the EARLY_FRAGMENT_TESTS stage so that no
        // accesses overlap.
        let stage = match scope {
            BarrierAccessScope::Source => GraphicsPrimitiveStage::LateFragmentTests,
            BarrierAccessScope::Destination => GraphicsPrimitiveStage::EarlyFragmentTests,
        };

        graphics = core::cmp::min(graphics, stage);
    }

    let transfer = match transfer {
        TransferStage::Transfer => vk::PipelineStageFlags2::TRANSFER,
        TransferStage::None => vk::PipelineStageFlags2::empty(),
    };

    let compute = match compute {
        ComputeStage::Compute => vk::PipelineStageFlags2::COMPUTE_SHADER,
        ComputeStage::None => vk::PipelineStageFlags2::empty(),
    };

    let graphics = match graphics {
        GraphicsPrimitiveStage::DrawIndirect => vk::PipelineStageFlags2::DRAW_INDIRECT,
        GraphicsPrimitiveStage::VertexInput => vk::PipelineStageFlags2::VERTEX_INPUT,
        GraphicsPrimitiveStage::VertexShader => vk::PipelineStageFlags2::VERTEX_SHADER,
        GraphicsPrimitiveStage::TaskShader => vk::PipelineStageFlags2::TASK_SHADER_EXT,
        GraphicsPrimitiveStage::MeshShader => vk::PipelineStageFlags2::MESH_SHADER_EXT,
        GraphicsPrimitiveStage::EarlyFragmentTests => vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS,
        GraphicsPrimitiveStage::FragmentShader => vk::PipelineStageFlags2::FRAGMENT_SHADER,
        GraphicsPrimitiveStage::LateFragmentTests => vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
        GraphicsPrimitiveStage::ColorAttachmentOutput => {
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT
        }
        GraphicsPrimitiveStage::None => vk::PipelineStageFlags2::empty(),
    };

    transfer | compute | graphics
}

fn validate_shader_bindings(shader: &ShaderInstance<'_>, descriptors: &[&DescriptorSetLayout]) {
    for shader_binding in shader.bindings() {
        let location = shader_binding.location;

        if location.group >= descriptors.len() as u32 {
            panic!(
                "shader requires descriptor set bound to group {} (only {} descriptor sets were bound)",
                location.group,
                descriptors.len(),
            );
        }

        let Some(binding) = descriptors[location.group as usize]
            .bindings
            .iter()
            .find(|descriptor_binding| descriptor_binding.binding == location.binding)
        else {
            panic!(
                "shader requires descriptor set with binding {} in group {}",
                location.binding, location.group,
            );
        };

        assert_eq!(shader_binding.count, binding.count);

        let is_compatible = if shader_binding.kind == DescriptorType::Texture
            && binding.kind == DescriptorType::StorageTexture
        {
            true
        } else {
            shader_binding.kind == binding.kind
        };

        assert!(
            is_compatible,
            "cannot bind {:?} to shader slot {:?}",
            binding, shader_binding,
        );
    }
}

fn create_pipeline_shader_module(
    device: &Device,
    shader: &ShaderInstance<'_>,
    layouts: &[&DescriptorSetLayout],
) -> Result<ShaderModule, Error> {
    if cfg!(debug_assertions) {
        validate_shader_bindings(shader, layouts);
    }

    unsafe { device.create_shader_module_spirv(&shader.to_spirv()) }
}

const ALLOC: Option<&vk::AllocationCallbacks<'static>> = Some(&vk::AllocationCallbacks {
    p_user_data: null_mut(),
    pfn_allocation: Some(vk_alloc),
    pfn_reallocation: Some(vk_realloc),
    pfn_free: Some(vk_dealloc),
    pfn_internal_allocation: None,
    pfn_internal_free: None,
    _marker: PhantomData,
});

/// Header providing information about a allocation.
///
/// We need this because `PFN_vkReallocationFunction` and `PFN_vkFreeFunction` do not provide the
/// size of the allocation so we must store it somewhere.
///
/// This header is prepended to every allocation, but it is important that we place the header
/// exactly before the pointer that we return and as such might need extra padding before the
/// header.
///
/// The entire allocation then looks as follows:
///
/// ```text
/// ---------------------------------
/// | Padding | Header | Allocation |
/// ---------------------------------
/// ```
#[derive(Copy, Clone, Debug)]
struct VkAllocHeader {
    ptr: *mut u8,
    layout: Layout,
}

unsafe extern "system" fn vk_alloc(
    _: *mut c_void,
    size: usize,
    align: usize,
    _scope: vk::SystemAllocationScope,
) -> *mut c_void {
    debug_assert!(align.is_power_of_two());

    if size == 0 {
        return null_mut();
    }

    // Allocations with small alignment need to aligned up
    // so that the header is always aligned.
    let align_up = usize::max(align, align_of::<VkAllocHeader>());

    // Round up the size of the header so that is a multiple of the align.
    // All bytes except for the sizeof header bytes are padding.
    let header_padding_size = (size_of::<VkAllocHeader>() + align_up - 1) & !(align_up - 1);
    let padding = header_padding_size - size_of::<VkAllocHeader>();

    let layout = unsafe { Layout::from_size_align_unchecked(header_padding_size + size, align_up) };

    unsafe {
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            return null_mut();
        }

        let header = ptr.byte_add(padding);
        let data = ptr.byte_add(padding + size_of::<VkAllocHeader>());

        debug_assert!(ptr.addr() + layout.size() - data.addr() >= size);
        debug_assert_eq!(data.addr() % align, 0);

        header
            .cast::<VkAllocHeader>()
            .write(VkAllocHeader { ptr, layout });

        data.cast::<c_void>()
    }
}

unsafe extern "system" fn vk_realloc(
    _: *mut c_void,
    ptr: *mut c_void,
    size: usize,
    align: usize,
    scope: vk::SystemAllocationScope,
) -> *mut c_void {
    if ptr.is_null() {
        return unsafe { vk_alloc(null_mut(), size, align, scope) };
    }

    if size == 0 {
        unsafe {
            vk_dealloc(null_mut(), ptr);
        }

        return null_mut();
    }

    unsafe {
        let header = ptr.cast::<VkAllocHeader>().sub(1).read();
        let header_padding_size =
            (size_of::<VkAllocHeader>() + header.layout.align() - 1) & !(header.layout.align() - 1);
        let old_size = header.layout.size() - header_padding_size;

        let new_ptr = vk_alloc(null_mut(), size, align, scope);

        // If the new allocation fails we MUST NOT deallocate
        // the old allocation.
        if new_ptr.is_null() {
            return null_mut();
        }

        let count = usize::min(old_size, size);
        core::ptr::copy_nonoverlapping(ptr.cast::<u8>(), new_ptr.cast::<u8>(), count);

        vk_dealloc(null_mut(), ptr);

        new_ptr
    }
}

unsafe extern "system" fn vk_dealloc(_: *mut c_void, ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let header = ptr.cast::<VkAllocHeader>().sub(1).read();
        std::alloc::dealloc(header.ptr, header.layout);
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::null_mut;

    use ash::vk::SystemAllocationScope;

    use super::{vk_alloc, vk_dealloc, vk_realloc};

    #[test]
    fn alloc_and_dealloc() {
        unsafe {
            let a = vk_alloc(null_mut(), 1, 1, SystemAllocationScope::INSTANCE);
            assert!(!a.is_null());
            assert_eq!(a.addr() % 1, 0);
            let b = vk_alloc(null_mut(), 123, 2, SystemAllocationScope::INSTANCE);
            assert!(!b.is_null());
            assert_eq!(b.addr() % 2, 0);

            vk_dealloc(null_mut(), a);
            vk_dealloc(null_mut(), b);
        }
    }

    #[test]
    fn realloc_nullptr() {
        unsafe {
            let a = vk_realloc(
                null_mut(),
                null_mut(),
                1,
                1,
                SystemAllocationScope::INSTANCE,
            );
            vk_dealloc(null_mut(), a);
        }
    }

    #[test]
    fn realloc_grow_align() {
        unsafe {
            let a = vk_alloc(null_mut(), 1, 1, SystemAllocationScope::INSTANCE);
            assert!(!a.is_null());
            assert_eq!(a.addr() % 1, 0);

            let b = vk_realloc(null_mut(), a, 23, 4, SystemAllocationScope::INSTANCE);
            assert!(!b.is_null());
            assert_eq!(a.addr() % 4, 0);

            vk_dealloc(null_mut(), b);
        }
    }

    #[test]
    fn realloc_shrink_align() {
        unsafe {
            let a = vk_alloc(null_mut(), 42, 2048, SystemAllocationScope::INSTANCE);
            assert!(!a.is_null());
            assert_eq!(a.addr() % 2048, 0);

            let b = vk_realloc(null_mut(), a, 64, 4, SystemAllocationScope::INSTANCE);
            assert!(!b.is_null());
            assert_eq!(b.addr() % 4, 0);

            vk_dealloc(null_mut(), b);
        }
    }
}
