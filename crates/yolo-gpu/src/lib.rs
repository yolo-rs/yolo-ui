// SPDX-License-Identifier: Apache-2.0
#![recursion_limit = "256"]
//! `yolo-gpu`: Unified WebGPU/WGPU backend, SPIR-V -> WGSL shader translator, 2D sprite sheet
//! & texture atlas engine, and SVG sanitizer / optimizer / theme / glyph / rasterizer for `yolo-ui`.
//!
//! Unifies:
//! - GPU device & surface management (`wgpu`, [`GpuDevice`], [`SurfaceContext`], [`hub`], [`vkwebgpu`])
//! - 2D Sprite Sheet & Atlas primitives ([`sprite_sheet`], [`Sprite`], [`SpriteSheet`])
//! - SVG processing & rasterization ([`sanitize`], [`viewbox`], [`theme`], [`sprite`], [`glyph`], [`symbols`], [`raster`], [`optimize`], [`diff`])

pub mod cuda_report;
pub mod diff;
mod error;
pub mod glyph;
#[cfg(not(target_arch = "wasm32"))]
pub mod hub;
pub mod optimize;
pub mod raster;
pub mod sanitize;
pub mod sprite;
pub mod sprite_sheet;
pub mod symbols;
pub mod theme;
pub mod viewbox;
pub mod vkwebgpu;
#[cfg(feature = "wasm-abi")]
pub mod vkwebgpu_wasm;
mod xml;

pub use error::{Error, Result};
pub use sprite_sheet::{ErreurSpriteSheet, Sprite, SpriteSheet, SpriteSheetError};
pub use wgpu;

/// Grouped SVG module re-exporting all SVG submodules (`yolo_gpu::svg::*`).
pub mod svg {
    pub use crate::diff::{self, DiffReport, diff_rgba, diff_svg};
    pub use crate::error::{Error, Result};
    pub use crate::glyph::{self, Glyph, GlyphOptions, GlyphOutline, NumberFormat, SvgPathPen, glyph_outline, ligature_glyph};
    pub use crate::optimize::{self, OptimizeOptions, optimize};
    pub use crate::raster::{self, FAVICON_ICO_SIZES, IcoImage, Raster, RenderOptions, WINDOWS_ICO_SIZES, WINDOWS_SHELL_ICO_SIZES, pack_ico, pack_ico_mixed, render, render_ico, render_ico_windows, render_png};
    pub use crate::sanitize::{self, SanitizeOptions, is_safe_url, sanitize};
    pub use crate::sprite::{self, SpriteEntry, sprite, symbol, use_ref};
    pub use crate::symbols::{self, GRID, SymbolAxes, SymbolStyle, cdn_path, decode_font, is_symbol_name, symbol_path, symbol_svg};
    pub use crate::theme::{self, ThemeOptions, apply_theme, role_hex};
    pub use crate::viewbox::{self, Geometry, Sizing, ViewBox, geometry, length_to_px, normalize};
    pub use crate::{SVG_NS, XLINK_NS, format_number};
}

/// The SVG 1.1 / SVG 2 namespace URI.
pub const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// The legacy XLink namespace URI, normalized to plain `href` when rewriting.
pub const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

/// Formats a finite `f64` with at most `precision` fractional digits and no trailing zeros.
#[must_use]
pub fn format_number(value: f64, precision: u8) -> String {
    if !value.is_finite() {
        return "0".to_owned();
    }
    let scale = 10_f64.powi(i32::from(precision));
    let rounded = (value * scale).round() / scale;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    let text = format!("{rounded:.prec$}", prec = usize::from(precision));
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    }
}

// ============================================================================
// WGPU Instance, Adapter, Device & Surface Primitives
// ============================================================================

/// Limits policy for [`request_device`].
#[derive(Clone, Debug, Default)]
pub enum DeviceLimits {
    /// `wgpu::Limits::default()`.
    #[default]
    Default,
    /// `adapter.limits()`: every capability the physical adapter reports.
    Adapter,
    /// Explicit limits supplied by the caller.
    Custom(wgpu::Limits),
}

impl DeviceLimits {
    fn resolve(self, adapter: &wgpu::Adapter) -> wgpu::Limits {
        match self {
            Self::Default => wgpu::Limits::default(),
            Self::Adapter => adapter.limits(),
            Self::Custom(limits) => limits,
        }
    }
}

/// Parameters for [`request_device`].
#[derive(Clone, Debug)]
pub struct DeviceRequest<'a> {
    pub label: wgpu::Label<'a>,
    pub required_features: wgpu::Features,
    pub limits: DeviceLimits,
    pub memory_hints: wgpu::MemoryHints,
}

impl<'a> DeviceRequest<'a> {
    /// Creates a request with no extra features and `wgpu::MemoryHints::Performance`.
    #[must_use]
    pub fn new(label: wgpu::Label<'a>, limits: DeviceLimits) -> Self {
        Self {
            label,
            required_features: wgpu::Features::empty(),
            limits,
            memory_hints: wgpu::MemoryHints::Performance,
        }
    }
}

impl Default for DeviceRequest<'_> {
    fn default() -> Self {
        Self::new(None, DeviceLimits::Default)
    }
}

/// A `wgpu::Device` paired with its submission `wgpu::Queue`.
#[derive(Clone, Debug)]
pub struct GpuDevice {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// Creates a `wgpu::Instance` from `descriptor`.
#[must_use]
pub fn create_instance(descriptor: wgpu::InstanceDescriptor) -> wgpu::Instance {
    wgpu::Instance::new(descriptor)
}

/// Enumerates adapters visible to `instance` for `backends`.
#[cfg(not(target_arch = "wasm32"))]
pub async fn enumerate_adapters(
    instance: &wgpu::Instance,
    backends: wgpu::Backends,
) -> Vec<wgpu::Adapter> {
    instance.enumerate_adapters(backends).await
}

/// Requests an adapter matching `options` from `instance`.
pub async fn request_adapter(
    instance: &wgpu::Instance,
    options: &wgpu::RequestAdapterOptions<'_, '_>,
) -> std::result::Result<wgpu::Adapter, wgpu::RequestAdapterError> {
    instance.request_adapter(options).await
}

/// Opens a logical [`GpuDevice`] on `adapter` using `request`.
pub async fn request_device(
    adapter: &wgpu::Adapter,
    request: DeviceRequest<'_>,
) -> std::result::Result<GpuDevice, wgpu::RequestDeviceError> {
    let descriptor = wgpu::DeviceDescriptor {
        label: request.label,
        required_features: request.required_features,
        required_limits: request.limits.resolve(adapter),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: request.memory_hints,
        trace: wgpu::Trace::Off,
    };
    let (device, queue) = adapter.request_device(&descriptor).await?;
    Ok(GpuDevice { device, queue })
}

/// A `wgpu::Surface` paired with the [`GpuDevice`] and `wgpu::SurfaceConfiguration` driving it.
pub struct SurfaceContext<'window> {
    pub surface: wgpu::Surface<'window>,
    pub gpu: GpuDevice,
    pub config: wgpu::SurfaceConfiguration,
}

/// Errors returned while creating a [`SurfaceContext`].
#[derive(Debug)]
pub enum SurfaceInitError {
    Adapter(wgpu::RequestAdapterError),
    Device(wgpu::RequestDeviceError),
    UnsupportedSurface,
}

impl std::fmt::Display for SurfaceInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adapter(err) => write!(f, "failed to request wgpu adapter: {err}"),
            Self::Device(err) => write!(f, "failed to request wgpu device: {err}"),
            Self::UnsupportedSurface => {
                f.write_str("wgpu surface is not supported by the selected adapter")
            }
        }
    }
}

impl std::error::Error for SurfaceInitError {}

impl<'window> SurfaceContext<'window> {
    /// Selects a high-performance adapter compatible with `surface`, opens a device with
    /// `device_request`, and configures the surface for `(width, height)`.
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'window>,
        width: u32,
        height: u32,
        power_preference: wgpu::PowerPreference,
        device_request: DeviceRequest<'_>,
    ) -> std::result::Result<Self, SurfaceInitError> {
        let adapter = request_adapter(
            instance,
            &wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            },
        )
        .await
        .map_err(SurfaceInitError::Adapter)?;

        let gpu = request_device(&adapter, device_request)
            .await
            .map_err(SurfaceInitError::Device)?;

        let width = width.max(1);
        let height = height.max(1);
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or(SurfaceInitError::UnsupportedSurface)?;
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&gpu.device, &config);

        Ok(Self {
            surface,
            gpu,
            config,
        })
    }

    /// Reconfigures the surface after a window resize. Zero dimensions are ignored.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.gpu.device, &self.config);
        }
    }
}
