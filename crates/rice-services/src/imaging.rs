//! Image files → JPEG data URLs (wallpaper today, covers and icons later).
//!
//! Data URLs keep the frontend's canvas untainted, so it can extract colours
//! and pre-render the glass layers (docs/BACKEND.md §2).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageReader};
use std::io::Cursor;
use std::path::Path;

/// JPEG quality of every data URL the backend produces.
pub const JPEG_QUALITY: u8 = 85;

/// How an image is fitted into the bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaling {
    /// Shrink until the image just covers the box (fill, fit, stretch, span).
    Cover,
    /// Shrink until the image fits inside the box (icons, covers).
    Contain,
    /// Keep the native size (centre, tile).
    Native,
}

/// Size an image of `width` × `height` gets for the box; never enlarges.
pub fn scaled_size(width: u32, height: u32, max_width: u32, max_height: u32, scaling: Scaling) -> (u32, u32) {
    if width == 0 || height == 0 || max_width == 0 || max_height == 0 {
        return (width, height);
    }
    let sx = f64::from(max_width) / f64::from(width);
    let sy = f64::from(max_height) / f64::from(height);
    let factor = match scaling {
        Scaling::Cover => sx.max(sy),
        Scaling::Contain => sx.min(sy),
        Scaling::Native => 1.0,
    }
    .min(1.0);
    let scale = |v: u32| ((f64::from(v) * factor).round() as u32).max(1);
    (scale(width), scale(height))
}

/// Decodes an image file (format sniffed from its content), shrinks it to the
/// box and encodes it as a `data:image/jpeg;base64,…` URL.
pub fn file_to_jpeg_data_url(path: &Path, max_width: u32, max_height: u32, scaling: Scaling) -> Result<String, String> {
    let image = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
    image_to_jpeg_data_url(&image, max_width, max_height, scaling)
}

/// Decodes image bytes (cover art, icons) like [`file_to_jpeg_data_url`].
pub fn bytes_to_jpeg_data_url(bytes: &[u8], max_width: u32, max_height: u32, scaling: Scaling) -> Result<String, String> {
    let image = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    image_to_jpeg_data_url(&image, max_width, max_height, scaling)
}

/// Encodes an already decoded image like [`file_to_jpeg_data_url`].
pub fn image_to_jpeg_data_url(image: &DynamicImage, max_width: u32, max_height: u32, scaling: Scaling) -> Result<String, String> {
    let (width, height) = scaled_size(image.width(), image.height(), max_width, max_height, scaling);
    let resized;
    let source = if (width, height) == (image.width(), image.height()) {
        image
    } else {
        resized = image.resize_exact(width, height, FilterType::Triangle);
        &resized
    };
    let rgb = source.to_rgb8();
    let mut bytes = Vec::with_capacity(rgb.len() / 8);
    JpegEncoder::new_with_quality(Cursor::new(&mut bytes), JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|e| e.to_string())?;
    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn computes_sizes_without_enlarging() {
        assert_eq!(scaled_size(3840, 2160, 1920, 1080, Scaling::Cover), (1920, 1080));
        // Portrait image on a landscape monitor: cover keeps the width.
        assert_eq!(scaled_size(2000, 4000, 1920, 1080, Scaling::Cover), (1920, 3840));
        assert_eq!(scaled_size(2000, 4000, 300, 300, Scaling::Contain), (150, 300));
        assert_eq!(scaled_size(800, 600, 1920, 1080, Scaling::Cover), (800, 600));
        assert_eq!(scaled_size(4000, 3000, 1920, 1080, Scaling::Native), (4000, 3000));
    }

    #[test]
    fn encodes_any_format_as_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        // No extension, like Windows' TranscodedWallpaper.
        let path = dir.path().join("TranscodedWallpaper");
        let png = RgbaImage::from_pixel(64, 32, Rgba([30, 120, 200, 255]));
        png.save_with_format(&path, image::ImageFormat::Png).unwrap();

        let url = file_to_jpeg_data_url(&path, 32, 16, Scaling::Cover).unwrap();
        let encoded = url.strip_prefix("data:image/jpeg;base64,").unwrap();
        let bytes = STANDARD.decode(encoded).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 16));
    }

    #[test]
    fn encodes_covers_from_bytes() {
        let mut png = Vec::new();
        RgbaImage::from_pixel(600, 400, Rgba([200, 30, 30, 255]))
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let url = bytes_to_jpeg_data_url(&png, 300, 300, Scaling::Contain).unwrap();
        let bytes = STANDARD.decode(url.strip_prefix("data:image/jpeg;base64,").unwrap()).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (300, 200));
        assert!(bytes_to_jpeg_data_url(b"junk", 300, 300, Scaling::Contain).is_err());
    }

    #[test]
    fn reports_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        assert!(file_to_jpeg_data_url(&dir.path().join("missing.jpg"), 10, 10, Scaling::Cover).is_err());
        let junk = dir.path().join("junk.jpg");
        std::fs::write(&junk, b"not an image").unwrap();
        assert!(file_to_jpeg_data_url(&junk, 10, 10, Scaling::Cover).is_err());
    }
}
