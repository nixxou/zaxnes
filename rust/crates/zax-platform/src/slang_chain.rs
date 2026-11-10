//! `RetroArch` slang shaders on the game picture, through librashader (`shaders` feature).
//!
//! Prototype: the preset is named by `ZAXNES_SHADER` (a `.slangp` path). The chain filters the
//! scene picture only; the interface is drawn over its output afterwards, unfiltered.

use std::path::PathBuf;

use librashader::presets::ShaderFeatures;
use librashader::runtime::wgpu::{FilterChain, FilterChainOptions, WgpuOutputView};
use librashader::runtime::{Size, Viewport};

/// The environment variable naming the preset of the prototype.
pub const PRESET_VARIABLE: &str = "ZAXNES_SHADER";

/// A loaded preset and the frame count its shaders read.
pub struct SlangChain {
    chain: FilterChain,
    frames: usize,
    /// The preset, for the log.
    pub preset: PathBuf,
}

impl SlangChain {
    /// The preset `ZAXNES_SHADER` names, compiled for `device`: `Ok(None)` when the variable is
    /// unset or empty, an error when the preset does not load or compile.
    ///
    /// # Errors
    ///
    /// The message of librashader when the preset cannot be parsed or compiled.
    pub fn from_environment(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        adapter: &wgpu::Adapter,
    ) -> Result<Option<Self>, String> {
        let Some(preset) = std::env::var_os(PRESET_VARIABLE).filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        let preset = PathBuf::from(preset);
        let options = FilterChainOptions {
            force_no_mipmaps: false,
            // The pipeline cache needs a device feature the window does not ask for.
            enable_cache: device.features().contains(wgpu::Features::PIPELINE_CACHE),
            adapter_info: Some(adapter.get_info()),
        };
        let chain = FilterChain::load_from_path(
            &preset,
            ShaderFeatures::NONE,
            device,
            queue,
            Some(&options),
        )
        .map_err(|error| format!("{}: {error}", preset.display()))?;
        Ok(Some(Self {
            chain,
            frames: 0,
            preset,
        }))
    }

    /// Records the passes filtering `input` into `output` (of `format`), over the rectangle at
    /// (`x`, `y`) of `width` x `height` pixels. An error is returned for the log; the frame is
    /// then drawn without the shader.
    ///
    /// # Errors
    ///
    /// The message of librashader when a pass cannot be recorded.
    #[allow(clippy::cast_precision_loss)] // Window sizes are far below 2^24.
    pub fn record(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Texture,
        output: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        (x, y, width, height): (u32, u32, u32, u32),
        target: (u32, u32),
    ) -> Result<(), String> {
        let viewport = Viewport {
            x: x as f32,
            y: y as f32,
            mvp: None,
            output: WgpuOutputView::new_from_raw(output, Size::new(target.0, target.1), format),
            size: Size::new(width.max(1), height.max(1)),
        };
        let frame = self.frames;
        self.frames = self.frames.wrapping_add(1);
        self.chain
            .frame(input, &viewport, encoder, frame, None)
            .map_err(|error| error.to_string())
    }
}
