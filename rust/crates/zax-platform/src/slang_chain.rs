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
use std::time::Instant;

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
    /// The width over the height of the picture the preset draws in, centred in the window;
    /// none for the whole window (a bezel around the picture, or the 3D scene).
    pub picture_aspect: Option<f32>,
    /// The width over the height the scene is drawn at for the preset, none for the window's:
    /// the 3D scene framed on the game's picture.
    pub scene_aspect: Option<f32>,
    /// The share of the window the preset draws at (0.5 to 1), stretched over it afterwards.
    pub resolution: f32,
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
    started: Instant,
    result: Receiver<Result<FilterChain, String>>,
    /// The thread, joined when it ends without an answer to tell why.
    thread: Option<std::thread::JoinHandle<()>>,
}

/// Runs `work` with the validation errors of `device` caught instead of panicking, and returns
/// them as text. Error scopes belong to the thread that pushes them.
fn validated<T>(device: &wgpu::Device, work: impl FnOnce() -> T) -> (T, Option<String>) {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = work();
    let error = pollster::block_on(scope.pop()).map(|error| error.to_string());
    (value, error)
}

/// The text of a thread's panic.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic without a message".to_owned())
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
    /// The line for the log of a preset dropped while drawing, given at the next update.
    dropped: Option<String>,
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
            dropped: None,
            frame: 0,
        }
    }

    /// Asks for `request` (none for no shader). The same preset again only updates its lines
    /// and parameters; another one starts compiling.
    pub fn request(&mut self, request: Option<&ShaderRequest>) {
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
        // Without a thread the receiver reads a disconnection, reported as a stopped compilation.
        let thread = std::thread::Builder::new()
            .name("zax slang".to_owned())
            .spawn(move || {
                let (chain, error) = validated(&device, || {
                    let preset = parse_combination(&path)?;
                    FilterChain::load_from_preset(preset, &device, &queue, Some(&options))
                        .map_err(|error| error.to_string())
                });
                let chain = match (chain, error) {
                    (Ok(chain), None) => Ok(chain),
                    (Ok(_), Some(error)) | (Err(error), _) => Err(error),
                }
                .map_err(|error| format!("{}: {error}", path.display()));
                // The window may have gone, or asked for another preset since.
                let _ = sender.send(chain);
            })
            .ok();
        Loading {
            preset,
            started: Instant::now(),
            result,
            thread,
        }
    }

    /// Takes the preset that finished compiling, if one did, with a line for the log.
    fn poll(&mut self) -> Option<String> {
        let loading = self.loading.as_mut()?;
        let result = match loading.result.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                let why = match loading.thread.take().map(std::thread::JoinHandle::join) {
                    Some(Err(payload)) => panic_text(payload.as_ref()),
                    _ => "the compilation stopped".to_owned(),
                };
                Err(format!("{}: {why}", loading.preset.display()))
            }
        };
        let preset = loading.preset.clone();
        let elapsed = loading.started.elapsed();
        self.loading = None;
        match result {
            Ok(chain) => {
                let line = format!(
                    "slang preset {} compiled in {} ms",
                    preset.display(),
                    elapsed.as_millis()
                );
                let mut active = Active::new(chain, preset);
                if let Some(wanted) = &self.wanted {
                    active.apply(&wanted.parameters);
                }
                self.active = Some(active);
                Some(line)
            }
            Err(problem) => {
                let line = format!("slang preset refused: {problem}");
                self.active = None;
                self.refused = Some((preset, problem));
                Some(line)
            }
        }
    }

    /// Takes a finished compilation, called once a frame before drawing with the emulated
    /// frame count: the line for the log when a preset was compiled or refused.
    pub fn update(&mut self, frame: usize) -> Option<String> {
        self.frame = frame;
        self.dropped.take().or_else(|| self.poll())
    }

    /// The width over the height of the rectangle the preset draws in (see
    /// [`ShaderRequest::picture_aspect`]).
    #[must_use]
    pub fn picture_aspect(&self) -> Option<f32> {
        self.wanted.as_ref()?.picture_aspect
    }

    /// The width over the height the scene has to be drawn at for the preset in use (see
    /// [`ShaderRequest::scene_aspect`]).
    #[must_use]
    pub fn scene_aspect(&self) -> Option<f32> {
        self.active.as_ref()?;
        self.wanted.as_ref()?.scene_aspect
    }

    /// The share of the window the preset in use draws at (1 without one).
    #[must_use]
    pub fn resolution(&self) -> f32 {
        if self.active.is_none() {
            return 1.0;
        }
        self.wanted
            .as_ref()
            .map_or(1.0, |wanted| wanted.resolution.clamp(0.25, 1.0))
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

    /// The commands filtering `input` into `output` (of `format`), over the rectangle at
    /// (`x`, `y`) of `width` x `height` pixels, to submit before the window's: none without a
    /// preset. A preset whose passes fail (or do not validate) is dropped with its problem, and
    /// nothing of it is submitted; the frame is then drawn without it.
    #[allow(clippy::cast_precision_loss)] // Window sizes are far below 2^24.
    pub fn record(
        &mut self,
        input: &wgpu::Texture,
        output: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        (x, y, width, height): (u32, u32, u32, u32),
        target: (u32, u32),
    ) -> Option<wgpu::CommandBuffer> {
        let active = self.active.as_mut()?;
        let viewport = Viewport {
            x: x as f32,
            y: y as f32,
            mvp: None,
            output: WgpuOutputView::new_from_raw(output, Size::new(target.0, target.1), format),
            size: Size::new(width.max(1), height.max(1)),
        };
        let frame = self.frame;
        let device = &self.device;
        let ((result, commands), error) = validated(device, || {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("zax slang"),
            });
            let result = active
                .chain
                .frame(input, &viewport, &mut encoder, frame, None);
            (result, encoder.finish())
        });
        let problem = match (result, error) {
            (Ok(()), None) => return Some(commands),
            (Err(error), _) => format!("a pass failed: {error}"),
            (Ok(()), Some(error)) => error,
        };
        let preset = active.preset.clone();
        self.dropped = Some(format!(
            "slang preset dropped: {}: {problem}",
            preset.display()
        ));
        self.refused = Some((preset, problem));
        self.active = None;
        None
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

/// Adds the parameter names `file` declares (`#pragma parameter NAME ...`) to `order`, in the
/// order of the text, the files it includes (`#include "..."`, relative to it) at their place.
/// The preprocessed source of librashader no longer has the pragma lines, hence this reading of
/// the files themselves; a file that does not read only loses the order of its parameters.
fn declared_names(file: &Path, order: &mut Vec<String>, visited: &mut Vec<PathBuf>) {
    if visited.iter().any(|known| known == file) || visited.len() > 256 {
        return;
    }
    visited.push(file.to_path_buf());
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("#pragma parameter ") {
            if let Some(name) = rest.split_whitespace().next() {
                if !order.iter().any(|known| known == name) {
                    order.push(name.to_owned());
                }
            }
        } else if let Some(rest) = line.strip_prefix("#include") {
            let included = rest
                .trim()
                .trim_matches(|c| c == '"' || c == '<' || c == '>');
            if let Some(folder) = file.parent() {
                declared_names(&folder.join(included), order, visited);
            }
        }
    }
}

/// A parameter a preset's shaders declare (`#pragma parameter`), for the interface.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterInfo {
    /// The name the shaders read it by.
    pub name: String,
    /// The text the shader gives it.
    pub description: String,
    /// The value the preset gives it (the shader's own unless the `.slangp` sets another).
    pub initial: f32,
    /// The lowest value.
    pub minimum: f32,
    /// The highest value.
    pub maximum: f32,
    /// The step of a slider (0 for none).
    pub step: f32,
}

impl ParameterInfo {
    /// Whether the parameter only titles the ones after it: presets such as Mega Bezel declare
    /// headings as parameters that cannot move.
    #[must_use]
    pub fn is_heading(&self) -> bool {
        self.maximum <= self.minimum
    }
}

/// The parameters of `preset`, in the order its passes declare them (the first pass that
/// declares one gives its place). Reads the preset and its shader files; compiles nothing.
///
/// # Errors
///
/// The message of librashader when the preset or one of its shaders cannot be read.
pub fn preset_parameters(preset: &Path) -> Result<Vec<ParameterInfo>, String> {
    use librashader::preprocess::ShaderSource;

    let parsed = parse_combination(preset)?;
    let mut order: Vec<String> = Vec::new();
    let mut found: HashMap<String, ParameterInfo> = HashMap::new();
    for pass in &parsed.passes {
        let source = ShaderSource::load(&pass.path, parsed.features)
            .map_err(|error| format!("{}: {error}", pass.path.display()))?;
        declared_names(&pass.path, &mut order, &mut Vec::new());
        for (name, parameter) in source.parameters {
            found.entry(name.to_string()).or_insert(ParameterInfo {
                name: name.to_string(),
                description: parameter.description.trim().to_owned(),
                initial: parameter.initial,
                minimum: parameter.minimum,
                maximum: parameter.maximum,
                step: parameter.step,
            });
        }
    }
    for set in &parsed.parameters {
        if let Some(info) = found.get_mut(set.name.as_str()) {
            info.initial = set.value;
        }
    }
    let mut parameters: Vec<ParameterInfo> =
        order.iter().filter_map(|name| found.remove(name)).collect();
    let mut rest: Vec<ParameterInfo> = found.into_values().collect();
    rest.sort_by(|a, b| a.name.cmp(&b.name));
    parameters.extend(rest);
    Ok(parameters)
}

/// What separates the presets of a combination in a stored preset: "a.slangp|b.slangp". The bar
/// cannot be in a Windows file name, and no preset of the pack has one in its path.
pub const COMBINATION_SEPARATOR: char = '|';

/// The preset `path` names, parsed: one file, or a combination of several (owner request: a
/// pre-filter such as `ScaleFX` or nnedi3 before a CRT, as `RetroArch`'s "Append" builds it). The
/// passes of each preset follow those of the one before; its textures and parameters are added
/// (a parameter set twice keeps the later value).
///
/// The last pass of every preset but the last one stops being the end of the chain. When it drew
/// at the size of the window (`viewport`, the final resampling of nnedi3 and xBRZ), it draws at
/// the size of its own input instead: the next preset is written for the picture, and would draw
/// one scanline per pixel of the window otherwise.
///
/// # Errors
///
/// The message of librashader when one of the presets cannot be read.
pub fn parse_combination(path: &Path) -> Result<librashader::presets::ShaderPreset, String> {
    use librashader::presets::{ScaleFactor, ScaleType, ShaderPreset};

    let text = path.to_string_lossy();
    let mut parts = text.split(COMBINATION_SEPARATOR).map(Path::new);
    let parse = |part: &Path| {
        ShaderPreset::try_parse(part, ShaderFeatures::NONE)
            .map_err(|error| format!("{}: {error}", part.display()))
    };
    let mut combined = parse(parts.next().unwrap_or(path))?;
    for part in parts {
        let next = parse(part)?;
        if let Some(last) = combined.passes.last_mut() {
            let scaling = &mut last.meta.scaling;
            if scaling.valid
                && (matches!(scaling.x.scale_type, ScaleType::Viewport)
                    || matches!(scaling.y.scale_type, ScaleType::Viewport))
            {
                for axis in [&mut scaling.x, &mut scaling.y] {
                    axis.scale_type = ScaleType::Input;
                    axis.factor = ScaleFactor::Float(1.0);
                }
            }
        }
        let first = i32::try_from(combined.passes.len()).unwrap_or(i32::MAX);
        for mut pass in next.passes {
            pass.meta.id = pass.meta.id.saturating_add(first);
            combined.passes.push(pass);
        }
        for texture in next.textures {
            if !combined
                .textures
                .iter()
                .any(|known| known.meta.name == texture.meta.name)
            {
                combined.textures.push(texture);
            }
        }
        combined.parameters.extend(next.parameters);
        combined.features |= next.features;
        combined.pass_count = i32::try_from(combined.passes.len()).unwrap_or(i32::MAX);
    }
    Ok(combined)
}
