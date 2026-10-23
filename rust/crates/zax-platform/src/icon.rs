//! The application icon on the window (lot ICON-1).
//!
//! The icon is cosmetic: a resource that is missing, truncated or not a PNG must never keep the
//! program from starting. Everything here therefore reports rather than fails, and the caller
//! logs the single line [`WindowIconState`]'s `Display` returns and carries on.

use std::fmt;

/// One size of the application icon, decoded: 8-bit RGBA, rows top-down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowIconImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width × height × 4` bytes, four per pixel in R, G, B, A order, the first row first.
    pub pixels: Vec<u8>,
}

impl WindowIconImage {
    /// Whether the buffer really holds `width × height` RGBA pixels. The window-icon call reads
    /// the buffer natively, so a short one would be read past its end.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.width > 0
            && self.height > 0
            && u64::try_from(self.pixels.len())
                .is_ok_and(|len| len == u64::from(self.width) * u64::from(self.height) * 4)
    }
}

/// What setting the window icon needs of a window. It exists so that the rule — which images
/// are offered, in which order, and what happens when none is usable — is testable without a
/// window; [`crate::HostWindow`] is the real target, the tests pass a recording double.
pub trait WindowIconTarget {
    /// Hands the window the icon, in every size available, smallest first.
    ///
    /// # Errors
    /// The backend's own message when it dislikes the icon.
    fn set_icon(&mut self, images: &[WindowIconImage]) -> Result<(), String>;
}

/// What [`apply`] actually put in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowIconState {
    /// Whether the window was given an icon at all.
    pub applied: bool,
    /// The sizes that were handed over, smallest first.
    pub sizes: Vec<u32>,
    /// How many images were dropped as malformed.
    pub rejected: usize,
    /// Why nothing was applied, or `None` on success.
    pub failure: Option<String>,
}

impl fmt::Display for WindowIconState {
    /// The one log line of the start-up sequence.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.applied {
            return write!(
                f,
                "window icon not set: {}",
                self.failure.as_deref().unwrap_or("no image was usable")
            );
        }
        let sizes: Vec<String> = self.sizes.iter().map(u32::to_string).collect();
        write!(f, "window icon: {} size(s) {} px", self.sizes.len(), sizes.join(", "))?;
        if self.rejected > 0 {
            write!(f, ", {} unusable image(s) dropped", self.rejected)?;
        }
        Ok(())
    }
}

fn not_applied(rejected: usize, failure: impl Into<String>) -> WindowIconState {
    WindowIconState {
        applied: false,
        sizes: Vec::new(),
        rejected,
        failure: Some(failure.into()),
    }
}

/// Offers `images` to `target`, smallest first, keeping only the well-formed ones.
///
/// `target` is `None` when there is no window.
pub fn apply(
    target: Option<&mut dyn WindowIconTarget>,
    images: &[WindowIconImage],
) -> WindowIconState {
    let Some(target) = target else {
        return not_applied(0, "there is no window");
    };
    if images.is_empty() {
        return not_applied(0, "no icon resource was decoded");
    }

    let mut usable: Vec<WindowIconImage> =
        images.iter().filter(|i| i.is_well_formed()).cloned().collect();
    let rejected = images.len() - usable.len();
    if usable.is_empty() {
        return not_applied(rejected, "every icon resource was malformed");
    }

    // Smallest first: the taskbar and the title bar ask for small sizes, and a window manager
    // that takes the first image rather than the closest match gets a sane one.
    usable.sort_by_key(|image| u64::from(image.width) * u64::from(image.height));

    // Whatever the backend does with an icon it dislikes, the window is already open and the
    // program keeps running without one.
    if let Err(message) = target.set_icon(&usable) {
        return not_applied(rejected, message);
    }

    WindowIconState {
        applied: true,
        sizes: usable.iter().map(|image| image.width).collect(),
        rejected,
        failure: None,
    }
}

/// The sizes embedded from `branding/`, the whole set of `branding/zaxnes.ico` (ICON-1 §3).
pub const SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

const EMBEDDED: [(u32, &[u8]); 7] = [
    (16, include_bytes!("../../../../branding/zaxnes-icon-16.png")),
    (24, include_bytes!("../../../../branding/zaxnes-icon-24.png")),
    (32, include_bytes!("../../../../branding/zaxnes-icon-32.png")),
    (48, include_bytes!("../../../../branding/zaxnes-icon-48.png")),
    (64, include_bytes!("../../../../branding/zaxnes-icon-64.png")),
    (128, include_bytes!("../../../../branding/zaxnes-icon-128.png")),
    (256, include_bytes!("../../../../branding/zaxnes-icon-256.png")),
];

/// The embedded PNG of `size` px, if that size is part of [`SIZES`].
#[must_use]
pub fn embedded_png(size: u32) -> Option<&'static [u8]> {
    EMBEDDED
        .iter()
        .find_map(|&(candidate, bytes)| (candidate == size).then_some(bytes))
}

/// Decodes a PNG into the RGBA image of `expected_size` px square. A resource that was replaced
/// by another image is refused, so that it cannot reach the native call with a lying size.
///
/// # Errors
/// The reason, as one line for the log.
pub fn decode_png(bytes: &[u8], expected_size: u32) -> Result<WindowIconImage, String> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    buffer.truncate(info.buffer_size());

    if info.width != expected_size || info.height != expected_size {
        return Err(format!(
            "the {expected_size} px icon is {}x{}",
            info.width, info.height
        ));
    }

    let pixels = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buffer
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buffer.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG left unexpanded".to_owned()),
    };
    Ok(WindowIconImage { width: info.width, height: info.height, pixels })
}

/// Decodes every embedded size, smallest first. The second value lists the problems, one per
/// resource that could not be decoded; the others are still returned.
#[must_use]
pub fn load_embedded() -> (Vec<WindowIconImage>, Vec<String>) {
    let mut images = Vec::new();
    let mut problems = Vec::new();
    for &(size, bytes) in &EMBEDDED {
        match decode_png(bytes, size) {
            Ok(image) => images.push(image),
            Err(reason) => problems.push(format!("{size} px icon: {reason}")),
        }
    }
    (images, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An icon target that records what it was handed.
    #[derive(Default)]
    struct RecordingWindow {
        images: Vec<WindowIconImage>,
        calls: usize,
    }

    impl WindowIconTarget for RecordingWindow {
        fn set_icon(&mut self, images: &[WindowIconImage]) -> Result<(), String> {
            self.calls += 1;
            self.images = images.to_vec();
            Ok(())
        }
    }

    struct FailingWindow;

    impl WindowIconTarget for FailingWindow {
        fn set_icon(&mut self, _: &[WindowIconImage]) -> Result<(), String> {
            Err("the window manager refused the icon".to_owned())
        }
    }

    fn square(size: u32) -> WindowIconImage {
        WindowIconImage { width: size, height: size, pixels: vec![7; (size * size * 4) as usize] }
    }

    /// ICON-1 §3
    #[test]
    fn every_embedded_icon_size_decodes_to_a_square_rgba_image() {
        for size in [16, 32, 48, 256] {
            assert!(SIZES.contains(&size));
        }

        let (images, problems) = load_embedded();

        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(images.len(), SIZES.len());
        for image in &images {
            assert!(SIZES.contains(&image.width));
            assert_eq!(image.width, image.height);
            assert_eq!(image.pixels.len(), (image.width * image.height * 4) as usize);
            assert!(image.is_well_formed());
        }

        // Smallest first, and every size present exactly once.
        let widths: Vec<u32> = images.iter().map(|i| i.width).collect();
        assert_eq!(widths, SIZES);

        // The icon is drawn, not blank.
        for image in &images {
            assert!(image.pixels.iter().skip(3).step_by(4).any(|&a| a == 255));
            let mut distinct = image.pixels.clone();
            distinct.sort_unstable();
            distinct.dedup();
            assert!(distinct.len() > 2, "the {} px icon has no content", image.width);
        }
    }

    /// ICON-1 §3
    #[test]
    fn the_window_icon_setter_is_called_once_with_every_decoded_size() {
        let (images, _) = load_embedded();
        let mut window = RecordingWindow::default();

        let state = apply(Some(&mut window), &images);

        assert!(state.applied);
        assert_eq!(window.calls, 1);
        assert_eq!(state.sizes, SIZES);
        assert_eq!(state.rejected, 0);
        assert!(state.to_string().contains("256"));
    }

    /// ICON-1 §3
    #[test]
    fn a_broken_resource_is_refused_rather_than_decoded() {
        // Not a PNG at all.
        assert!(decode_png(b"not a png", 32).is_err());

        // A PNG header followed by nothing: a truncated resource.
        let truncated = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        assert!(decode_png(&truncated, 16).is_err());

        // Empty.
        assert!(decode_png(&[], 16).is_err());

        // A real icon offered under the wrong size is refused too.
        let real = embedded_png(32).unwrap();
        assert_eq!(decode_png(real, 32).unwrap().width, 32);
        assert!(decode_png(real, 48).unwrap_err().contains("32"));
    }

    /// ICON-1 §3
    #[test]
    fn nothing_usable_leaves_the_window_alone_without_throwing() {
        let mut window = RecordingWindow::default();

        let none = apply(Some(&mut window), &[]);
        assert!(!none.applied);
        assert_eq!(window.calls, 0);
        assert!(none.to_string().contains("window icon not set"));

        let malformed = apply(
            Some(&mut window),
            &[WindowIconImage { width: 16, height: 16, pixels: vec![1, 2, 3] }],
        );
        assert!(!malformed.applied);
        assert_eq!(malformed.rejected, 1);
        assert_eq!(window.calls, 0);

        // A mixed list keeps the usable images and counts the dropped ones.
        let (images, _) = load_embedded();
        let mut mixed_input = vec![WindowIconImage { width: 8, height: 8, pixels: Vec::new() }];
        mixed_input.extend(images);
        let mixed = apply(Some(&mut window), &mixed_input);
        assert!(mixed.applied);
        assert_eq!(mixed.rejected, 1);
        assert!(mixed.to_string().contains("1 unusable image(s) dropped"));

        // No window at all.
        let (images, _) = load_embedded();
        let no_window = apply(None, &images);
        assert!(!no_window.applied);
    }

    #[test]
    fn images_are_offered_smallest_first() {
        let mut window = RecordingWindow::default();
        let state = apply(Some(&mut window), &[square(32), square(16), square(24)]);
        assert_eq!(state.sizes, [16, 24, 32]);
        assert_eq!(window.images[0].width, 16);
    }

    /// ICON-1 §3
    #[test]
    fn a_window_that_refuses_the_icon_does_not_stop_the_start_up() {
        let (images, _) = load_embedded();
        let state = apply(Some(&mut FailingWindow), &images);
        assert!(!state.applied);
        assert!(state.to_string().contains("refused the icon"));
    }
}
