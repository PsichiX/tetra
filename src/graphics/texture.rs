//! Functions and types relating to textures.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use crate::error::{Result, TetraError};
use crate::fs;
use crate::graphics::{self, DrawParams, Rectangle};
use crate::platform::{GraphicsDevice, RawTexture};
use crate::Context;

#[derive(Debug)]
pub(crate) struct TextureSharedData {
    pub(crate) handle: RawTexture,
    filter_mode: Cell<FilterMode>,
}

impl PartialEq for TextureSharedData {
    fn eq(&self, other: &TextureSharedData) -> bool {
        // filter_mode should always match what's set on the GPU,
        // so we can ignore it for equality checks.

        self.handle.eq(&other.handle)
    }
}

/// A texture, held in GPU memory.
///
/// # Supported Formats
///
/// Various file formats are supported, and can be enabled or disabled via Cargo features:
///
/// | Format | Cargo feature | Enabled by default? |
/// |-|-|-|
/// | PNG | `texture_png` | Yes |
/// | JPEG | `texture_jpeg` | Yes |
/// | GIF | `texture_gif` | Yes |
/// | BMP | `texture_bmp` | Yes |
/// | TIFF | `texture_tiff` | No |
/// | TGA | `texture_tga` | No |
/// | WebP | `texture_webp` | No |
/// | ICO | `texture_ico` | No |
/// | PNM | `texture_pnm` | No |
/// | DDS/DXT | `texture_dds` | No |
///
/// # Performance
///
/// Creating a `Texture` is a relatively expensive operation. If you can, store them in your
/// [`State`](crate::State) struct rather than recreating them each frame.
///
/// Cloning a `Texture` is a very cheap operation, as the underlying data is shared between the
/// original instance and the clone via [reference-counting](https://doc.rust-lang.org/std/rc/struct.Rc.html).
/// This does mean, however, that updating a `Texture` (for example, changing its filter mode) will also
/// update any other clones of that `Texture`.
///
/// # Examples
///
/// The [`texture`](https://github.com/17cupsofcoffee/tetra/blob/main/examples/texture.rs)
/// example demonstrates how to draw a simple texture.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub(crate) data: Rc<TextureSharedData>,
}

impl Texture {
    /// Creates a new texture from the given file.
    ///
    /// The format will be determined based on the file extension.
    ///
    /// # Errors
    ///
    /// * [`TetraError::PlatformError`] will be returned if the underlying graphics API encounters an error.
    /// * [`TetraError::FailedToLoadAsset`] will be returned if the file could not be loaded.
    /// * [`TetraError::InvalidTexture`] will be returned if the texture data was invalid.
    pub fn new<P>(ctx: &mut Context, path: P) -> Result<Texture>
    where
        P: AsRef<Path>,
    {
        let image = fs::read_to_image(path)?.to_rgba8();
        let (width, height) = image.dimensions();

        Texture::from_rgba(
            ctx,
            width as i32,
            height as i32,
            image.into_raw().as_slice(),
        )
    }

    /// Creates a new texture from a slice of data, encoded in one of Tetra's supported
    /// file formats (except for TGA).
    ///
    /// This is useful in combination with [`include_bytes`](std::include_bytes), as it
    /// allows you to include your textures directly in the binary.
    ///
    /// The format will be determined based on the 'magic bytes' at the beginning of the
    /// data. This should be reasonably reliable, but a `from_data_with_format` function
    /// might have to be added later. Note that TGA files do not have recognizable magic
    /// bytes, so this function will not recognize them.
    ///
    /// # Errors
    ///
    /// * [`TetraError::PlatformError`] will be returned if the underlying graphics API encounters an error.
    /// * [`TetraError::InvalidTexture`] will be returned if the texture data was invalid.
    pub fn from_file_data(ctx: &mut Context, data: &[u8]) -> Result<Texture> {
        let image = image::load_from_memory(data)
            .map_err(TetraError::InvalidTexture)?
            .to_rgba8();

        let (width, height) = image.dimensions();

        Texture::from_rgba(
            ctx,
            width as i32,
            height as i32,
            image.into_raw().as_slice(),
        )
    }

    /// Creates a new texture from a slice of RGBA pixel data.
    ///
    /// This is useful if you wish to create a texture at runtime.
    ///
    /// This method requires you to provide enough data to fill the texture.
    /// If you provide too little data, an error will be returned.
    /// If you provide too much data, it will be truncated.
    ///
    /// # Errors
    ///
    /// * [`TetraError::NotEnoughData`] will be returned if not enough data is provided to fill
    /// the texture. This is to prevent the graphics API from trying to read uninitialized memory.
    pub fn from_rgba(ctx: &mut Context, width: i32, height: i32, data: &[u8]) -> Result<Texture> {
        Texture::with_device(
            &mut ctx.device,
            width,
            height,
            data,
            ctx.graphics.default_filter_mode,
        )
    }

    pub(crate) fn with_device(
        device: &mut GraphicsDevice,
        width: i32,
        height: i32,
        data: &[u8],
        filter_mode: FilterMode,
    ) -> Result<Texture> {
        let handle = device.new_texture(width, height)?;

        device.set_texture_data(&handle, &data, 0, 0, width, height)?;
        device.set_texture_filter_mode(&handle, filter_mode);

        Ok(Texture {
            data: Rc::new(TextureSharedData {
                handle,
                filter_mode: Cell::new(FilterMode::Linear),
            }),
        })
    }

    pub(crate) fn with_device_empty(
        device: &mut GraphicsDevice,
        width: i32,
        height: i32,
        filter_mode: FilterMode,
    ) -> Result<Texture> {
        let handle = device.new_texture(width, height)?;
        device.set_texture_filter_mode(&handle, filter_mode);

        Ok(Texture {
            data: Rc::new(TextureSharedData {
                handle,
                filter_mode: Cell::new(filter_mode),
            }),
        })
    }

    /// Draws the texture to the screen (or to a canvas, if one is enabled).
    pub fn draw<P>(&self, ctx: &mut Context, params: P)
    where
        P: Into<DrawParams>,
    {
        let params = params.into();

        graphics::set_texture(ctx, self);
        graphics::push_quad(
            ctx,
            0.0,
            0.0,
            self.width() as f32,
            self.height() as f32,
            0.0,
            0.0,
            1.0,
            1.0,
            &params,
        );
    }

    /// Draws a region of the texture to the screen (or to a canvas, if one is enabled).
    pub fn draw_region<P>(&self, ctx: &mut Context, region: Rectangle, params: P)
    where
        P: Into<DrawParams>,
    {
        let params = params.into();

        let texture_width = self.width() as f32;
        let texture_height = self.height() as f32;

        graphics::set_texture(ctx, self);
        graphics::push_quad(
            ctx,
            0.0,
            0.0,
            region.width,
            region.height,
            region.x / texture_width,
            region.y / texture_height,
            region.right() / texture_width,
            region.bottom() / texture_height,
            &params,
        );
    }

    /// Draws a region of the texture by splitting it into nine slices, allowing it to be stretched or
    /// squashed without distorting the borders.
    pub fn draw_nine_slice<P>(
        &self,
        ctx: &mut Context,
        config: &NineSlice,
        width: f32,
        height: f32,
        params: P,
    ) where
        P: Into<DrawParams>,
    {
        let params = params.into();

        let texture_width = self.width() as f32;
        let texture_height = self.height() as f32;

        let x1 = 0.0;
        let y1 = 0.0;
        let x2 = config.left;
        let y2 = config.top;
        let x3 = width - config.right;
        let y3 = height - config.bottom;
        let x4 = width;
        let y4 = height;

        let u1 = config.region.x / texture_width;
        let v1 = config.region.y / texture_height;
        let u2 = (config.region.x + config.left) / texture_width;
        let v2 = (config.region.y + config.top) / texture_height;
        let u3 = (config.region.x + config.region.width - config.right) / texture_width;
        let v3 = (config.region.y + config.region.height - config.bottom) / texture_height;
        let u4 = (config.region.x + config.region.width) / texture_width;
        let v4 = (config.region.y + config.region.height) / texture_height;

        graphics::set_texture(ctx, self);

        // Top left
        graphics::push_quad(ctx, x1, y1, x2, y2, u1, v1, u2, v2, &params);

        // Top
        graphics::push_quad(ctx, x2, y1, x3, y2, u2, v1, u3, v2, &params);

        // Top right
        graphics::push_quad(ctx, x3, y1, x4, y2, u3, v1, u4, v2, &params);

        // Left
        graphics::push_quad(ctx, x1, y2, x2, y3, u1, v2, u2, v3, &params);

        // Center
        graphics::push_quad(ctx, x2, y2, x3, y3, u2, v2, u3, v3, &params);

        // Right
        graphics::push_quad(ctx, x3, y2, x4, y3, u3, v2, u4, v3, &params);

        // Bottom left
        graphics::push_quad(ctx, x1, y3, x2, y4, u1, v3, u2, v4, &params);

        // Bottom
        graphics::push_quad(ctx, x2, y3, x3, y4, u2, v3, u3, v4, &params);

        // Bottom right
        graphics::push_quad(ctx, x3, y3, x4, y4, u3, v3, u4, v4, &params);
    }

    /// Returns the width of the texture.
    pub fn width(&self) -> i32 {
        self.data.handle.width()
    }

    /// Returns the height of the texture.
    pub fn height(&self) -> i32 {
        self.data.handle.height()
    }

    /// Returns the size of the canvas.
    pub fn size(&self) -> (i32, i32) {
        (self.data.handle.width(), self.data.handle.height())
    }

    /// Returns the filter mode being used by the texture.
    pub fn filter_mode(&self) -> FilterMode {
        self.data.filter_mode.get()
    }

    /// Sets the filter mode that should be used by the texture.
    pub fn set_filter_mode(&mut self, ctx: &mut Context, filter_mode: FilterMode) {
        ctx.device
            .set_texture_filter_mode(&self.data.handle, filter_mode);

        self.data.filter_mode.set(filter_mode);
    }

    /// Writes RGBA pixel data to a specified region of the texture.
    ///
    /// This method requires you to provide enough data to fill the target rectangle.
    /// If you provide too little data, an error will be returned.
    /// If you provide too much data, it will be truncated.
    ///
    /// If you want to overwrite the entire texture, the [`replace_data`](Self::replace_data)
    /// method offers a more concise way of doing this.
    ///
    /// # Errors
    ///
    /// * [`TetraError::NotEnoughData`] will be returned if not enough data is provided to fill
    /// the target rectangle. This is to prevent the graphics API from trying to read
    /// uninitialized memory.
    ///
    /// # Panics
    ///
    /// Panics if any part of the target rectangle is outside the bounds of the texture.
    pub fn set_data(
        &self,
        ctx: &mut Context,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        data: &[u8],
    ) -> Result {
        ctx.device
            .set_texture_data(&self.data.handle, &data, x, y, width, height)
    }

    /// Overwrites the entire texture with new RGBA pixel data.
    ///
    /// This method requires you to provide enough data to fill the texture.
    /// If you provide too little data, an error will be returned.
    /// If you provide too much data, it will be truncated.
    ///
    /// If you only want to write to a subsection of the texture, use the [`set_data`](Self::set_data)
    /// method instead.
    ///
    /// # Errors
    ///
    /// * [`TetraError::NotEnoughData`] will be returned if not enough data is provided to fill
    /// the texture. This is to prevent the graphics API from trying to read uninitialized memory.
    pub fn replace_data(&self, ctx: &mut Context, data: &[u8]) -> Result {
        let (width, height) = self.size();
        self.set_data(ctx, 0, 0, width, height, data)
    }
}

/// Filtering algorithms that can be used when scaling an image.
///
/// Tetra currently defaults to using `Nearest` for all newly created textures.
#[derive(Debug, Clone, Copy)]
pub enum FilterMode {
    /// Nearest-neighbor interpolation. This preserves hard edges and details, but may look pixelated.
    ///
    /// If you're using pixel art, this is probably the scaling mode you should use.
    Nearest,

    /// Linear interpolation. This smooths images when scaling them up or down.
    Linear,
}

/// Information on how to slice a texture so that it can be stretched or squashed without
/// distorting the borders.
///
/// This can be used with [`Texture::draw_nine_slice`] to easily draw things like UI panels.
///
/// # Examples
///
/// The [`nineslice`](https://github.com/17cupsofcoffee/tetra/blob/main/examples/nineslice.rs)
/// example demonstrates how to draw a `NineSlice` panel.
#[derive(Debug, Clone)]
pub struct NineSlice {
    /// The region of the texture that should be used.
    pub region: Rectangle,

    /// The offset of the border on the left side.
    pub left: f32,

    /// The offset of the border on the right side.
    pub right: f32,

    /// The offset of the border on the top side.
    pub top: f32,

    /// The offset of the border on the bottom side.
    pub bottom: f32,
}

impl NineSlice {
    /// Creates a new nine slice configuration with the given offsets.
    pub fn new(region: Rectangle, left: f32, right: f32, top: f32, bottom: f32) -> NineSlice {
        NineSlice {
            region,
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates a new nine slice configuration, using the same offset for all edges.
    pub fn with_border(region: Rectangle, border: f32) -> NineSlice {
        NineSlice {
            region,
            left: border,
            right: border,
            top: border,
            bottom: border,
        }
    }
}
