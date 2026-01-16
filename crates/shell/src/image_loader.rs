//! Image Loading
//!
//! Loads and decodes images from URLs or files.

use gugalanna_layout::{ImagePixels, LayoutBox, BoxType};
use gugalanna_net::HttpClient;
use image::GenericImageView;
use log::{debug, warn};
use std::fs;
use url::Url;

/// Image loading error
#[derive(Debug)]
pub enum ImageLoadError {
    /// Invalid URL format
    InvalidUrl(String),
    /// Network fetch failed
    FetchFailed(String),
    /// HTTP error status
    HttpError(u16),
    /// Image decoding failed
    DecodeFailed(String),
    /// Data URLs not supported yet
    DataUrlNotSupported,
    /// File read error
    FileReadError(String),
}

impl std::fmt::Display for ImageLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageLoadError::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            ImageLoadError::FetchFailed(e) => write!(f, "Fetch failed: {}", e),
            ImageLoadError::HttpError(code) => write!(f, "HTTP error: {}", code),
            ImageLoadError::DecodeFailed(e) => write!(f, "Decode failed: {}", e),
            ImageLoadError::DataUrlNotSupported => write!(f, "Data URLs not supported"),
            ImageLoadError::FileReadError(e) => write!(f, "File read error: {}", e),
        }
    }
}

/// Load an image from a URL (relative or absolute)
pub fn load_image(
    client: &HttpClient,
    base_url: &Url,
    src: &str,
) -> Result<DecodedImage, ImageLoadError> {
    if src.is_empty() {
        return Err(ImageLoadError::InvalidUrl("Empty src".to_string()));
    }

    // Handle data URLs
    if src.starts_with("data:") {
        return Err(ImageLoadError::DataUrlNotSupported);
    }

    // Resolve URL
    let url = resolve_image_url(base_url, src)?;

    // Check if it's a file URL
    if url.scheme() == "file" {
        return load_image_from_file(&url);
    }

    // Fetch image bytes from network
    let bytes = fetch_image_bytes(client, &url)?;

    // Decode the image
    decode_image(&bytes)
}

/// Resolve image source to absolute URL
fn resolve_image_url(base: &Url, src: &str) -> Result<Url, ImageLoadError> {
    // Already absolute?
    if src.contains("://") {
        return Url::parse(src).map_err(|e| ImageLoadError::InvalidUrl(e.to_string()));
    }

    // Cannot resolve relative URLs against certain base URLs
    if base.cannot_be_a_base() {
        return Err(ImageLoadError::InvalidUrl(format!(
            "Cannot resolve relative path '{}' against '{}'",
            src, base.scheme()
        )));
    }

    base.join(src).map_err(|e| ImageLoadError::InvalidUrl(e.to_string()))
}

/// Load image from a file:// URL
fn load_image_from_file(url: &Url) -> Result<DecodedImage, ImageLoadError> {
    let path = url
        .to_file_path()
        .map_err(|_| ImageLoadError::InvalidUrl("Invalid file path".to_string()))?;

    let bytes = fs::read(&path)
        .map_err(|e| ImageLoadError::FileReadError(format!("{}: {}", path.display(), e)))?;

    decode_image(&bytes)
}

/// Fetch image bytes from a URL using the HTTP client
fn fetch_image_bytes(client: &HttpClient, url: &Url) -> Result<Vec<u8>, ImageLoadError> {
    debug!("Fetching image: {}", url);

    // Use tokio to run the async fetch
    let response = tokio::task::block_in_place(|| {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| ImageLoadError::FetchFailed("No tokio runtime".to_string()))?;

        rt.block_on(client.get(url))
            .map_err(|e| ImageLoadError::FetchFailed(e.to_string()))
    })?;

    if !response.is_success() {
        return Err(ImageLoadError::HttpError(response.status));
    }

    Ok(response.body)
}

/// Decode image bytes to RGBA pixel data
fn decode_image(bytes: &[u8]) -> Result<DecodedImage, ImageLoadError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| ImageLoadError::DecodeFailed(e.to_string()))?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
    let data = rgba.into_raw();

    debug!("Decoded image: {}x{}", width, height);

    Ok(DecodedImage { width, height, data })
}

/// Decoded image data
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Load images in a layout tree (modifies ImageData in-place)
pub fn load_images_in_tree(
    layout_box: &mut LayoutBox,
    client: &HttpClient,
    base_url: &Url,
) {
    load_images_recursive(layout_box, client, base_url);
}

fn load_images_recursive(
    layout_box: &mut LayoutBox,
    client: &HttpClient,
    base_url: &Url,
) {
    // Check if this is an image box
    if let BoxType::Image(_, ref mut image_data, _) = layout_box.box_type {
        // Only load if we don't have pixel data yet
        if image_data.pixels.is_none() && !image_data.src.is_empty() {
            match load_image(client, base_url, &image_data.src) {
                Ok(decoded) => {
                    // Update intrinsic dimensions from decoded image
                    image_data.intrinsic_width = Some(decoded.width as f32);
                    image_data.intrinsic_height = Some(decoded.height as f32);

                    // Store pixel data
                    image_data.pixels = Some(ImagePixels {
                        width: decoded.width,
                        height: decoded.height,
                        data: decoded.data,
                    });

                    debug!(
                        "Loaded image: {} ({}x{})",
                        image_data.src, decoded.width, decoded.height
                    );
                }
                Err(e) => {
                    warn!("Failed to load image '{}': {}", image_data.src, e);
                }
            }
        }
    }

    // Recurse into children
    for child in &mut layout_box.children {
        load_images_recursive(child, client, base_url);
    }
}
