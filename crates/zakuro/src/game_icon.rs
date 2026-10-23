//! the running game's own icon, the one the 3DS HOME Menu shows, on the
//! window and, on macOS, in the Dock, which takes it from the application
//! rather than the window, and Zakuro's again back in the library.

#[cfg(not(target_os = "macos"))]
use winit::window::Icon;
use winit::window::Window;

use crate::library::ICON_SIZE;

/// how many times the icon's 48 pixels it is shown at, in square blocks
const TIMES: usize = 4;

/// shows a game's icon, RGBA, ICON_SIZE square, as library::Smdh has it.
pub fn show(window: &Window, icon: &[u8]) {
    let size = ICON_SIZE * TIMES;
    let mut rgba = vec![0u8; size * size * 4];
    for (i, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y) = ((i % size) / TIMES, (i / size) / TIMES);
        let at = (y * ICON_SIZE + x) * 4;
        pixel.copy_from_slice(&icon[at..at + 4]);
    }
    #[cfg(target_os = "macos")]
    {
        let _ = window;
        dock(Some((&rgba, size)));
    }
    #[cfg(not(target_os = "macos"))]
    window.set_window_icon(Icon::from_rgba(rgba, size as u32, size as u32).ok());
}

/// Zakuro's own icon again.
pub fn clear(window: &Window) {
    #[cfg(target_os = "macos")]
    {
        let _ = window;
        dock(None);
    }
    #[cfg(not(target_os = "macos"))]
    window.set_window_icon(None::<Icon>);
}

/// the Dock's icon, the picture given, size square RGBA, or the
/// application's own with none.
#[cfg(target_os = "macos")]
fn dock(picture: Option<(&[u8], usize)>) {
    use objc2::{AnyThread as _, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage};
    use objc2_foundation::NSSize;

    // AppKit is the main thread's, which the window's events come on
    let Some(main) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(main);
    let Some((rgba, size)) = picture else {
        // SAFETY: none puts the application's own icon back
        unsafe { app.setApplicationIconImage(None) };
        return;
    };
    // SAFETY: no planes given, so the image allocates its own pixels, which
    // are written below, size by size RGBA, 4 bytes a pixel
    unsafe {
        let Some(image) = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            size as isize,
            size as isize,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            (size * 4) as isize,
            32,
        ) else {
            return;
        };
        let pixels = image.bitmapData();
        if pixels.is_null() {
            return;
        }
        std::ptr::copy_nonoverlapping(rgba.as_ptr(), pixels, size * size * 4);
        let icon = NSImage::initWithSize(NSImage::alloc(), NSSize::new(size as f64, size as f64));
        icon.addRepresentation(&image);
        app.setApplicationIconImage(Some(&icon));
    }
}
