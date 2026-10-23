//! one Vulkan device for the renderer and the presenter, when they would
//! pick the same GPU, so the screens the renderer draws are shown straight
//! from its images rather than copied out to the host and up again.

use ash::vk;

/// a device, the instance it came from and its one queue, gone once the
/// last of the renderer and the presenter lets go of it. both record and
/// submit on the thread the window runs on, so the queue needs no lock.
pub struct SharedDevice {
    pub(crate) _entry: ash::Entry,
    pub(crate) instance: ash::Instance,
    pub(crate) physical: vk::PhysicalDevice,
    pub(crate) family: u32,
    pub(crate) device: ash::Device,
    pub(crate) queue: vk::Queue,
    /// whether the device does logic ops, and counts pipeline statistics.
    pub(crate) logic_ops: bool,
    pub(crate) statistics: bool,
    /// the blend state draws set as they go.
    pub(crate) dynamic: crate::raster::hardware::Dynamic,
}

impl SharedDevice {
    /// a device on the GPU the renderer would pick, with no presenter, to
    /// time showing the screens straight from the GPU without a window.
    pub fn new() -> Result<SharedDevice, String> {
        crate::raster::hardware::own_device()
    }
}

/// the system's Vulkan library. on macOS there is none of the system's,
/// MoltenVK comes beside the app or from Homebrew, whose folder the default
/// search leaves out, ZAKURO_VULKAN_LIB names another.
pub(crate) fn load_entry() -> Result<ash::Entry, String> {
    // SAFETY: loading the Vulkan library has no other requirements
    let error = match unsafe { ash::Entry::load() } {
        Ok(entry) => return Ok(entry),
        Err(error) => error,
    };
    if !cfg!(target_os = "macos") {
        return Err(error.to_string());
    }
    let mut candidates: Vec<std::path::PathBuf> = std::env::var_os("ZAKURO_VULKAN_LIB").into_iter().map(Into::into).collect();
    if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.to_owned())) {
        candidates.push(dir.join("../Frameworks/libMoltenVK.dylib"));
        candidates.push(dir.join("libMoltenVK.dylib"));
    }
    for name in ["libvulkan.1.dylib", "/opt/homebrew/lib/libvulkan.1.dylib", "/opt/homebrew/lib/libMoltenVK.dylib", "/usr/local/lib/libvulkan.1.dylib", "/usr/local/lib/libMoltenVK.dylib"] {
        candidates.push(name.into());
    }
    for path in candidates {
        // SAFETY: as above, a Vulkan library or a driver that is one
        if let Ok(entry) = unsafe { ash::Entry::load_from(&path) } {
            log::info!("Vulkan from {}", path.display());
            return Ok(entry);
        }
    }
    Err(format!("{error}, and no MoltenVK either, brew install molten-vk"))
}

/// the extensions and flags an instance needs to list GPUs that are not
/// wholly Vulkan, MoltenVK's on macOS, when the loader knows of them.
pub(crate) fn portability(entry: &ash::Entry) -> (Vec<*const std::ffi::c_char>, vk::InstanceCreateFlags) {
    // SAFETY: a query of the loaded library
    let available = unsafe { entry.enumerate_instance_extension_properties(None) }.unwrap_or_default();
    let name = ash::khr::portability_enumeration::NAME;
    if available.iter().any(|e| e.extension_name_as_c_str() == Ok(name)) {
        (vec![name.as_ptr()], vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR)
    } else {
        (Vec::new(), vk::InstanceCreateFlags::empty())
    }
}

/// the extension a device that is not wholly Vulkan has to be made with, by
/// the spec, when it is one.
pub(crate) fn portability_subset(instance: &ash::Instance, physical: vk::PhysicalDevice) -> Option<*const std::ffi::c_char> {
    // SAFETY: a query of a GPU of the instance
    let available = unsafe { instance.enumerate_device_extension_properties(physical) }.unwrap_or_default();
    let name = ash::khr::portability_subset::NAME;
    available.iter().any(|e| e.extension_name_as_c_str() == Ok(name)).then_some(name.as_ptr())
}

impl std::fmt::Debug for SharedDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedDevice").field("family", &self.family).finish_non_exhaustive()
    }
}

impl Drop for SharedDevice {
    fn drop(&mut self) {
        // SAFETY: whoever made something from the device destroyed it before
        // letting go, the surface of the instance went with the presenter
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
