//! `RetroArch` slang shaders on the game picture, through librashader (`shaders` feature).
//!
//! The application asks every frame for the preset of the display mode ([`ShaderRequest`]); a
//! new preset compiles on a thread of its own while the picture goes on (without a shader the
//! first time, through the previous preset after), then takes over. The parameters the player
//! changed are applied over the preset's own values at once, without a recompilation. The chain
//! filters the scene picture only; the interface is drawn over its output afterwards.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use librashader::presets::ShaderFeatures;
use librashader::runtime::wgpu::{FilterChain, FilterChainOptions, WgpuOutputView};
use librashader::runtime::{FilterChainParameters, Size, Viewport};

/// The environment variable naming a preset that overrides the settings (debugging).
pub const PRESET_VARIABLE: &str = "ZAXNES_SHADER";

/// The preset `ZAXNES_SHADER` names, if any (read once).
#[must_use]
pub fn preset_from_environment() -> Option<PathBuf> {
    static PRESET: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    PRESET
        .get_or_init(|| {
            std::env::var_os(PRESET_VARIABLE)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .clone()
}

/// The shader the application wants on the picture.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderRequest {
    /// The `.slangp` file.
    pub preset: PathBuf,
    /// The lines of the picture the preset reads, 0 for the window's own.
    pub lines: u32,
    /// The parameters the player changed, over the preset's own values.
    pub parameters: Vec<(String, f32)>,
}

/// A compiled preset.
struct Active {
    chain: FilterChain,
    preset: PathBuf,
    /// The values the preset gives its parameters, to go back to when the player resets one.
    defaults: HashMap<String, f32>,
    /// The parameters last applied.
    applied: Vec<(String, f32)>,
}

impl Active {
    fn new(chain: FilterChain, preset: PathBuf) -> Self {
        let defaults = chain
            .parameters()
            .parameters()
            .iter()
            .map(|(name, value)| (name.to_string(), *value))
            .collect();
        Self {
            chain,
            preset,
            defaults,
            applied: Vec::new(),
        }
    }

    /// Sets every parameter to the player's value, or back to the preset's own.
    fn apply(&mut self, parameters: &[(String, f32)]) {
        if self.applied == parameters {
            return;
        }
        let defaults = &self.defaults;
        self.chain.parameters().update_parameters(|map| {
            for (name, value) in map.iter_mut() {
                let name: &str = name.as_ref();
                let wanted = parameters
                    .iter()
                    .find(|(known, _)| known == name)
                    .map(|(_, value)| *value)
                    .or_else(|| defaults.get(name).copied());
                if let Some(wanted) = wanted {
                    *value = wanted;
                }
            }
        });
        self.applied = parameters.to_vec();
    }
}

/// A preset compiling on its thread.
struct Loading {
    preset: PathBuf,
    result: Receiver<Result<FilterChain, String>>,
}

/// The slang shaders of the window: the preset in use, the one compiling and the last refusal.
pub struct SlangShaders {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
    wanted: Option<ShaderRequest>,
    active: Option<Active>,
    loading: Option<Loading>,
    /// The preset that failed and why: it is not tried again until another one is asked for.
    refused: Option<(PathBuf, String)>,
    /// The frame count the shaders read (the emulated frames, so that an animated effect keeps
    /// the game's pace at 120 Hz).
    frame: usize,
}

impl SlangShaders {
    /// No shader yet, on `device`.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, adapter: &wgpu::Adapter) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),
            adapter_info: adapter.get_info(),
            wanted: None,
            active: None,
            loading: None,
            refused: None,
            frame: 0,
        }
    }

    /// Asks for `request` (none for no shader). The same preset again only updates its lines
    /// and parameters; another one starts compiling.
    pub fn request(&mut self, request: Option<&ShaderRequest>) {
        self.poll();
        if self.wanted.as_ref() == request {
            return;
        }
        self.wanted = request.cloned();
        let Some(request) = request else {
            self.active = None;
            self.loading = None;
            return;
        };
        if let Some(active) = self.active.as_mut().filter(|a| a.preset == request.preset) {
            active.apply(&request.parameters);
            self.loading = None;
            return;
        }
        if self
            .loading
            .as_ref()
            .is_some_and(|loading| loading.preset == request.preset)
        {
            return;
        }
        if self
            .refused
            .as_ref()
            .is_some_and(|(preset, _)| *preset == request.preset)
        {
            self.active = None;
            return;
        }
        self.refused = None;
        self.loading = Some(self.start(request.preset.clone()));
    }

    /// Compiles `preset` on a thread of its own.
    fn start(&self, preset: PathBuf) -> Loading {
        let (sender, result) = mpsc::channel();
        let device = self.device.clone();
        let queue = self.queue.clone();
        let options = FilterChainOptions {
            force_no_mipmaps: false,
            // The pipeline cache needs a device feature the window does not ask for.
            enable_cache: device.features().contains(wgpu::Features::PIPELINE_CACHE),
            adapter_info: Some(self.adapter_info.clone()),
        };
        let path = preset.clone();
        std::thread::spawn(move || {
            let chain = FilterChain::load_from_path(
                &path,
                ShaderFeatures::NONE,
                &device,
                &queue,
                Some(&options),
            )
            .map_err(|error| format!("{}: {error}", path.display()));
            // The window may have gone, or asked for another preset since.
            let _ = sender.send(chain);
        });
        Loading { preset, result }
    }

    /// Takes the preset that finished compiling, if one did.
    fn poll(&mut self) {
        let Some(loading) = &self.loading else {
            return;
        };
        let result = match loading.result.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(format!(
                "{}: the compilation stopped",
                loading.preset.display()
            )),
        };
        let preset = loading.preset.clone();
        self.loading = None;
        match result {
            Ok(chain) => {
                let mut active = Active::new(chain, preset);
                if let Some(wanted) = &self.wanted {
                    active.apply(&wanted.parameters);
                }
                self.active = Some(active);
            }
            Err(problem) => {
                self.active = None;
                self.refused = Some((preset, problem));
            }
        }
    }

    /// Takes a finished compilation; called once a frame before drawing.
    pub fn update(&mut self, frame: usize) {
        self.frame = frame;
        self.poll();
    }

    /// The lines the scene has to be drawn at for the preset in use: none without a preset, 0
    /// for the window's own.
    #[must_use]
    pub fn lines(&self) -> Option<u32> {
        self.active.as_ref()?;
        self.wanted.as_ref().map(|wanted| wanted.lines)
    }

    /// The preset in use, the one compiling, or why the one asked for was refused.
    #[must_use]
    pub fn status(&self) -> ShaderStatus<'_> {
        if let Some(loading) = &self.loading {
            return ShaderStatus::Compiling(&loading.preset);
        }
        if let Some(active) = &self.active {
            return ShaderStatus::Active(&active.preset);
        }
        if let Some((preset, problem)) = &self.refused {
            if self
                .wanted
                .as_ref()
                .is_some_and(|wanted| wanted.preset == *preset)
            {
                return ShaderStatus::Refused(preset, problem);
            }
        }
        ShaderStatus::None
    }

    /// Records the passes filtering `input` into `output` (of `format`), over the rectangle at
    /// (`x`, `y`) of `width` x `height` pixels: `false` without a preset. A pass that fails
    /// drops the preset (the frame is then drawn without it) and keeps the problem.
    #[allow(clippy::cast_precision_loss)] // Window sizes are far below 2^24.
    pub fn record(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::Texture,
        output: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        (x, y, width, height): (u32, u32, u32, u32),
        target: (u32, u32),
    ) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        let viewport = Viewport {
            x: x as f32,
            y: y as f32,
            mvp: None,
            output: WgpuOutputView::new_from_raw(output, Size::new(target.0, target.1), format),
            size: Size::new(width.max(1), height.max(1)),
        };
        match active
            .chain
            .frame(input, &viewport, encoder, self.frame, None)
        {
            Ok(()) => true,
            Err(error) => {
                let preset = active.preset.clone();
                self.refused = Some((preset, error.to_string()));
                self.active = None;
                false
            }
        }
    }
}

/// What the shaders are doing, for the log and the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStatus<'a> {
    /// No preset asked for.
    None,
    /// The preset is compiling.
    Compiling(&'a Path),
    /// The preset filters the picture.
    Active(&'a Path),
    /// The preset asked for failed, and why.
    Refused(&'a Path, &'a str),
}
