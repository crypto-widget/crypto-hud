use std::{cell::RefCell, collections::VecDeque};

use anyhow::{Context, Result};
use i_slint_core::graphics::ImageInner;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use slint_interpreter::{ComponentInstance, Value};

pub(crate) const RESAMPLE_IMAGE_CALLBACK: &str = "resample-image";
pub(crate) const RESAMPLE_ICON_CALLBACK: &str = "resample-icon";
const MAX_IMAGE_DIMENSION: u32 = 2048;
const ICON_CACHE_MAX_ENTRIES: usize = 64;
const ICON_CACHE_MAX_BYTES: usize = 32 * 1024 * 1024;

/// Bind once, before the component's image properties are evaluated. Slint caches
/// the resulting image until a source image or requested physical size changes.
pub(crate) fn install_image_resampling(instance: &ComponentInstance) -> Result<()> {
    for callback in [RESAMPLE_IMAGE_CALLBACK, RESAMPLE_ICON_CALLBACK] {
        if !instance
            .definition()
            .callbacks()
            .any(|name| name == callback)
        {
            continue;
        }
        let icons = RefCell::new(IconResamplingCache::default());
        instance
            .set_callback(callback, move |arguments| {
                let [Value::Image(source), Value::Number(width), Value::Number(height)] = arguments
                else {
                    eprintln!("invalid arguments to dynamic widget {callback} callback");
                    return Value::Image(Image::default());
                };
                let (Some(width), Some(height)) =
                    (image_dimension(*width), image_dimension(*height))
                else {
                    return Value::Image(source.clone());
                };
                Value::Image(if callback == RESAMPLE_ICON_CALLBACK {
                    icons
                        .borrow_mut()
                        .resample(source, width as i32, height as i32)
                } else {
                    resample_image(source, width, height)
                })
            })
            .with_context(|| format!("failed to bind dynamic widget {callback} callback"))?;
    }
    Ok(())
}

struct CachedIcon {
    source: Image,
    width: u32,
    height: u32,
    result: Image,
    retained_bytes: usize,
}

/// Keep one cache per widget, shared by its repeaters and refreshed quote models.
/// Both the number of sizes and retained source/output memory remain bounded.
#[derive(Default)]
pub(crate) struct IconResamplingCache {
    entries: VecDeque<CachedIcon>,
    retained_bytes: usize,
}

impl IconResamplingCache {
    /// Fit a raster icon inside a physical-pixel box without enlarging it. SVG
    /// images retain their vector representation, including SVG loaded from data.
    pub(crate) fn resample(&mut self, source: &Image, box_width: i32, box_height: i32) -> Image {
        let inner: &ImageInner = source.into();
        if !matches!(
            inner,
            ImageInner::EmbeddedImage { .. } | ImageInner::StaticTextures(_)
        ) {
            return source.clone();
        }
        let Some((width, height)) = icon_contain_size(source, box_width, box_height) else {
            return source.clone();
        };
        // Slint's Image equality compares the decoded buffer identity as well as
        // its resource key. A replacement loaded at the same path is not a hit.
        // Retaining the source also prevents its buffer address from being reused.
        let hit = self.entries.iter().position(|entry| {
            entry.source == *source && entry.width == width && entry.height == height
        });
        if let Some(entry) = hit.and_then(|index| self.entries.remove(index)) {
            let result = entry.result.clone();
            self.entries.push_front(entry);
            return result;
        }
        let result = resample_image(source, width, height);
        if result == *source {
            return result;
        }
        let size = source.size();
        // Count source storage conservatively as RGBA, even when source images
        // are shared between entries or use three-channel RGB buffers.
        let retained_bytes =
            (size.width as usize * size.height as usize + width as usize * height as usize) * 4;
        if retained_bytes > ICON_CACHE_MAX_BYTES {
            return result;
        }
        while self.entries.len() >= ICON_CACHE_MAX_ENTRIES
            || self.retained_bytes + retained_bytes > ICON_CACHE_MAX_BYTES
        {
            let Some(oldest) = self.entries.pop_back() else {
                break;
            };
            self.retained_bytes -= oldest.retained_bytes;
        }
        self.retained_bytes += retained_bytes;
        self.entries.push_front(CachedIcon {
            source: source.clone(),
            width,
            height,
            result: result.clone(),
            retained_bytes,
        });
        result
    }
}

fn icon_contain_size(source: &Image, box_width: i32, box_height: i32) -> Option<(u32, u32)> {
    let (width, height) = (
        u32::try_from(box_width).ok()?,
        u32::try_from(box_height).ok()?,
    );
    let size = source.size();
    if width == 0
        || height == 0
        || width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || size.width == 0
        || size.height == 0
        || size.width > MAX_IMAGE_DIMENSION
        || size.height > MAX_IMAGE_DIMENSION
        || (width >= size.width && height >= size.height)
    {
        return None;
    }
    let (width, height) = if size.width * height >= size.height * width {
        let width = width.min(size.width);
        (
            width,
            ((size.height * width + size.width / 2) / size.width).max(1),
        )
    } else {
        let height = height.min(size.height);
        (
            ((size.width * height + size.height / 2) / size.height).max(1),
            height,
        )
    };
    Some((width, height))
}

fn image_dimension(value: f64) -> Option<u32> {
    (value.is_finite()
        && value.fract() == 0.0
        && (1.0..=f64::from(MAX_IMAGE_DIMENSION)).contains(&value))
    .then_some(value as u32)
}

/// Area-filter raster artwork to its physical display size. Accumulate RGB with
/// alpha so transparent source pixels cannot introduce dark or colored halos.
/// Unsupported images, invalid sizes, and requests to upscale preserve the input.
pub(crate) fn resample_image(source: &Image, width: u32, height: u32) -> Image {
    let size = source.size();
    if width == 0
        || height == 0
        || width > size.width
        || height > size.height
        || size.width > MAX_IMAGE_DIMENSION
        || size.height > MAX_IMAGE_DIMENSION
        || (width == size.width && height == size.height)
    {
        return source.clone();
    }
    let Some(pixels) = source.to_rgba8() else {
        return source.clone();
    };
    if pixels.width() != size.width || pixels.height() != size.height {
        return source.clone();
    }
    let mut output = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let input = pixels.as_slice();
    let destination = output.make_mut_slice();
    // Axis coordinates use target dimensions as denominators. Integer overlap
    // weights make fractional shrink ratios exact without rounding source edges.
    let area = u64::from(size.width) * u64::from(size.height);
    for y in 0..height {
        let top = y * size.height;
        let bottom = (y + 1) * size.height;
        for x in 0..width {
            let left = x * size.width;
            let right = (x + 1) * size.width;
            let mut alpha = 0_u64;
            let mut red = 0_u64;
            let mut green = 0_u64;
            let mut blue = 0_u64;
            for source_y in top / height..bottom.div_ceil(height) {
                let vertical = bottom.min((source_y + 1) * height) - top.max(source_y * height);
                for source_x in left / width..right.div_ceil(width) {
                    let horizontal = right.min((source_x + 1) * width) - left.max(source_x * width);
                    let pixel = input[(source_y * size.width + source_x) as usize];
                    let weight = u64::from(horizontal) * u64::from(vertical) * u64::from(pixel.a);
                    alpha += weight;
                    red += weight * u64::from(pixel.r);
                    green += weight * u64::from(pixel.g);
                    blue += weight * u64::from(pixel.b);
                }
            }
            let a = ((alpha + area / 2) / area) as u8;
            destination[(y * width + x) as usize] = if a == 0 {
                Rgba8Pixel::default()
            } else {
                Rgba8Pixel {
                    r: ((red + alpha / 2) / alpha) as u8,
                    g: ((green + alpha / 2) / alpha) as u8,
                    b: ((blue + alpha / 2) / alpha) as u8,
                    a,
                }
            };
        }
    }
    Image::from_rgba8(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, pixels: &[[u8; 4]]) -> Image {
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
        for (output, &[r, g, b, a]) in buffer.make_mut_slice().iter_mut().zip(pixels) {
            *output = Rgba8Pixel { r, g, b, a };
        }
        Image::from_rgba8(buffer)
    }

    #[test]
    fn image_resampling_averages_pixel_coverage_instead_of_skipping_detail() {
        let source = image(
            2,
            2,
            &[
                [255, 255, 255, 255],
                [0, 0, 0, 255],
                [0, 0, 0, 255],
                [255, 255, 255, 255],
            ],
        );
        let result = resample_image(&source, 1, 1).to_rgba8().unwrap();
        assert_eq!(
            result.as_slice(),
            &[Rgba8Pixel {
                r: 128,
                g: 128,
                b: 128,
                a: 255
            }]
        );
    }

    #[test]
    fn image_resampling_handles_fractional_pixel_coverage() {
        let source = image(
            3,
            1,
            &[[0, 0, 0, 255], [255, 255, 255, 255], [0, 0, 0, 255]],
        );
        let result = resample_image(&source, 2, 1).to_rgba8().unwrap();
        assert_eq!(
            result.as_slice(),
            &[Rgba8Pixel {
                r: 85,
                g: 85,
                b: 85,
                a: 255
            }; 2]
        );
    }

    #[test]
    fn image_resampling_preserves_transparent_edge_color_without_halos() {
        let source = image(2, 1, &[[240, 120, 30, 255], [0, 0, 255, 0]]);
        let result = resample_image(&source, 1, 1).to_rgba8().unwrap();
        assert_eq!(
            result.as_slice(),
            &[Rgba8Pixel {
                r: 240,
                g: 120,
                b: 30,
                a: 128
            }]
        );
        let transparent = image(2, 1, &[[240, 120, 30, 0], [0, 0, 255, 0]]);
        assert_eq!(
            resample_image(&transparent, 1, 1)
                .to_rgba8()
                .unwrap()
                .as_slice(),
            &[Rgba8Pixel::default()]
        );
    }

    #[test]
    fn image_resampling_preserves_images_when_no_bounded_shrink_is_possible() {
        let source = image(2, 1, &[[240, 120, 30, 255], [0, 0, 255, 0]]);
        for (width, height) in [(2, 1), (3, 1), (1, 2), (0, 1), (1, 0), (2049, 1)] {
            assert_eq!(resample_image(&source, width, height), source);
        }
        let oversized = Image::from_rgba8(SharedPixelBuffer::new(MAX_IMAGE_DIMENSION + 1, 1));
        assert_eq!(resample_image(&oversized, 1, 1), oversized);
        let empty = resample_image(&Image::default(), 1, 1);
        assert_eq!((empty.size().width, empty.size().height), (0, 0));
        assert!(empty.to_rgba8().is_none());
        for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 1.5, 2049.0] {
            assert_eq!(image_dimension(value), None);
        }
        assert_eq!(image_dimension(2048.0), Some(2048));
    }

    #[test]
    fn icon_resampling_contains_non_square_rasters_without_distortion_or_upscaling() {
        let source = image(6, 3, &[[200, 100, 50, 255]; 18]);
        let mut cache = IconResamplingCache::default();
        for (width, height, expected) in [(3, 3, (3, 2)), (4, 2, (4, 2)), (6, 1, (2, 1))] {
            let result = cache.resample(&source, width, height);
            assert_eq!((result.size().width, result.size().height), expected);
            assert!(result.to_rgba8().unwrap().as_slice().iter().all(|pixel| {
                *pixel
                    == Rgba8Pixel {
                        r: 200,
                        g: 100,
                        b: 50,
                        a: 255,
                    }
            }));
        }
        for (width, height) in [(6, 3), (20, 20), (0, 3), (-1, 3), (3, 2049)] {
            assert_eq!(cache.resample(&source, width, height), source);
        }
    }

    #[test]
    fn icon_resampling_preserves_svg_identity_and_its_vector_representation() {
        let source = Image::load_from_svg_data(
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="128"><path fill="#f00" d="M0 0H256V128H0Z"/></svg>"##,
        ).unwrap();
        let mut cache = IconResamplingCache::default();
        assert_eq!(cache.resample(&source, 9, 9), source);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn icon_resampling_cache_reuses_images_for_repeated_models_and_contain_boxes() {
        let source = image(8, 4, &[[200, 100, 50, 255]; 32]);
        let mut cache = IconResamplingCache::default();
        let first = cache.resample(&source, 4, 4);
        assert_eq!(cache.resample(&source.clone(), 4, 4), first);
        // Different boxes with the same actual raster size also share the result.
        assert_eq!(cache.resample(&source, 4, 8), first);
        let smaller = cache.resample(&source, 2, 2);
        assert_ne!(first, smaller);
        assert_eq!(cache.resample(&source, 2, 2), smaller);
        assert_eq!(cache.resample(&source, 4, 4), first);
        assert_eq!(cache.entries.len(), 2);
    }

    #[test]
    fn icon_resampling_cache_does_not_reuse_replaced_pixels_with_the_same_resource_key() {
        use i_slint_core::graphics::{ImageCacheKey, SharedImageBuffer};

        let keyed = |color| {
            let source = image(2, 2, &[color; 4]);
            Image::from(ImageInner::EmbeddedImage {
                // As with equal path/mtime keys, equality must also check the
                // underlying decoded buffer when its contents have been replaced.
                cache_key: ImageCacheKey::EmbeddedData(1),
                buffer: SharedImageBuffer::RGBA8(source.to_rgba8().unwrap()),
            })
        };
        let original = keyed([255, 0, 0, 255]);
        let replacement = keyed([0, 0, 255, 255]);
        let mut cache = IconResamplingCache::default();
        let first = cache.resample(&original, 1, 1);
        let second = cache.resample(&replacement, 1, 1);
        assert_ne!(first, second);
        assert_eq!(second.to_rgba8().unwrap().as_slice()[0].b, 255);
        assert_eq!(cache.resample(&replacement.clone(), 1, 1), second);
    }

    #[test]
    fn icon_resampling_cache_evicts_unused_images_and_bounds_retained_memory() {
        let mut cache = IconResamplingCache::default();
        let first_source = image(2, 2, &[[200, 100, 50, 255]; 4]);
        let first = cache.resample(&first_source, 1, 1);
        for _ in 0..ICON_CACHE_MAX_ENTRIES {
            cache.resample(&image(2, 2, &[[50, 100, 200, 255]; 4]), 1, 1);
        }
        assert_eq!(cache.entries.len(), ICON_CACHE_MAX_ENTRIES);
        assert_ne!(cache.resample(&first_source, 1, 1), first);

        let mut cache = IconResamplingCache::default();
        for _ in 0..3 {
            let source = Image::from_rgba8(SharedPixelBuffer::new(2048, 2048));
            cache.resample(&source, 1, 1);
            assert!(cache.retained_bytes <= ICON_CACHE_MAX_BYTES);
        }
        assert_eq!(cache.entries.len(), 1);
    }
}
