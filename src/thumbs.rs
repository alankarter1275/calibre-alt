//! A0 step 3 — persistent cover thumbnails.
//!
//! The library grid was decoding the *full* cover (often 1000×1500+) on the UI
//! thread for every card, on every rebuild. The in-memory `COVER_CACHE` died at
//! relaunch, so a fresh launch re-decoded everything again.
//!
//! This module generates a small thumbnail at import time (a pure, headless,
//! CI-testable function) into `cache/thumbs/<uuid>.png`. The grid then decodes
//! that tiny PNG instead of the full cover, and because it survives relaunch,
//! the first grid render after startup is cheap too.
//!
//! gdk-pixbuf in this toolchain can decode but not encode PNG, so we use the
//! `image` crate (pure Rust, no system deps) for the resize + PNG encode.

use std::path::Path;

/// Thumbnail size. 2× the grid card (128×204) to stay crisp on HiDPI, while
/// still being tiny enough to decode and cache cheaply.
pub const THUMB_W: u32 = 256;
pub const THUMB_H: u32 = 408;

/// Generate a thumbnail for `source` (any image `image` can read) at
/// `THUMB_W`×`THUMB_H`, writing a PNG to `dest`.
///
/// Returns `false` (never panics) when the source is missing, unreadable, or
/// not a supported image — the caller then simply falls back to decoding the
/// full cover. Purely best-effort: a missing thumbnail is not an error.
pub fn generate_thumbnail(source: &Path, dest: &Path) -> bool {
    if !source.is_file() {
        return false;
    }
    let Ok(img) = image::open(source) else {
        return false;
    };
    let resized = img.resize_exact(THUMB_W, THUMB_H, image::imageops::FilterType::Triangle);
    if let Some(parent) = dest.parent() {
        if !parent.exists() && std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    resized.save(dest).is_ok()
}

/// Remove a book's thumbnail (used on book deletion so the cache cannot grow).
pub fn remove_thumbnail(uuid: &str) {
    let _ = std::fs::remove_file(crate::paths::thumbnail_path(uuid));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A unique scratch dir under the OS temp dir, cleaned up on drop.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "kalam-thumbs-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(&p).expect("scratch dir");
            Scratch(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_solid_png(path: &Path, w: u32, h: u32, rgb: [u8; 3]) {
        let buf: Vec<u8> = (0..(w * h)).flat_map(|_| rgb).collect();
        image::save_buffer(path, &buf, w, h, image::ExtendedColorType::Rgb8).expect("write png");
    }

    #[test]
    fn generates_an_exact_size_png() {
        let dir = Scratch::new();
        let src = dir.join("cover.png");
        let dst = dir.join("thumb.png");
        // A cover larger than the thumbnail, e.g. 600×900.
        write_solid_png(&src, 600, 900, [30, 60, 120]);

        assert!(generate_thumbnail(&src, &dst));
        assert!(dst.is_file());

        let reopened = image::open(&dst).expect("reopen thumb");
        assert_eq!(reopened.dimensions(), (THUMB_W, THUMB_H));
    }

    #[test]
    fn missing_source_is_not_an_error() {
        let dir = Scratch::new();
        let dst = dir.join("thumb.png");
        assert!(!generate_thumbnail(&dir.join("nope.png"), &dst));
        assert!(!dst.exists());
    }

    #[test]
    fn non_image_source_returns_false() {
        use std::io::Write;
        let dir = Scratch::new();
        let src = dir.join("cover.txt");
        let mut f = std::fs::File::create(&src).unwrap();
        f.write_all(b"this is not an image").unwrap();
        let dst = dir.join("thumb.png");
        assert!(!generate_thumbnail(&src, &dst));
        assert!(!dst.exists());
    }
}
