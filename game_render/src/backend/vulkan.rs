use std::borrow::Cow;
use std::collections::HashSet;
use std::ffi::{c_void, CStr};
use std::num::NonZeroU32;
use std::ptr::null_mut;

use ash::ext::debug_utils;
use ash::vk::{
    self, ApplicationInfo, Bool32, ColorSpaceKHR, CompositeAlphaFlagsKHR,
    DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
    DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT,
    DeviceCreateInfo, DeviceQueueCreateInfo, DeviceQueueInfo2, Extent2D, Format, ImageUsageFlags,
    InstanceCreateInfo, PhysicalDevice, PhysicalDeviceFeatures, PhysicalDeviceType, PresentModeKHR,
    QueueFlags, SharingMode, SurfaceKHR, SurfaceTransformFlagsKHR, SwapchainCreateInfoKHR,
    SwapchainKHR, FALSE,
};
use ash::Entry;
use glam::UVec2;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::{
    AdapterKind, AdapterProperties, PresentMode, QueueCapabilities, QueueFamily,
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

const DEVICE_EXTENSIONS: &[&CStr] = &[ash::khr::swapchain::NAME];

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

        let create_info = DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            // Device layers are deprecated, but the Vulkan spec still recommends
            // applications to pass layers.
            // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#extendingvulkan-layers-devicelayerdeprecation
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features);

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
    queue: ash::vk::Queue,
}

impl<'a> Queue<'a> {}

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

        Swapchain {
            surface: self,
            swapchain,
            device,
            images,
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
    images: Vec<vk::Image>,

    format: TextureFormat,
    extent: UVec2,
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
