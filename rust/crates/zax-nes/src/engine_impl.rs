//! [`Engine`] for Zaxnes's own core: thin delegation, nothing more.
//!
//! Every method below forwards to the `NesCore` method the application already called, marked
//! `#[inline]` so that the indirection a `Box<dyn Engine>` adds is the vtable call and nothing
//! else. No emulation code is touched by this file, which is the owner's rule for the lot: with
//! the `fceux-core` feature off, the only engine is this one and the application behaves exactly
//! as it did.
//!
//! **Zaxnes-only things are deliberately absent here**, not forwarded through a wider trait:
//! `wide_picture_into` and the widescreen fields, the hack packs and `options_patch`,
//! `set_sprite_limit`, `cartridge()`/`bus()`/`ppu()`, `run_frame_ahead`/`restore_real_frame` and
//! `capture_core_state`/`restore_core_state`. A caller reaches them by downcasting through
//! [`Engine::as_any`] / [`Engine::as_any_mut`] after
//! [`Capabilities`](zax_core_api::Capabilities) has said the engine has them — so rollback and
//! run-ahead keep using the fast in-memory path on this core, which is why the flags exist.

use std::any::Any;

use zax_core_api::{
    AudioError, Capabilities, ControllerPortType, Engine, FrameBatch, InputWord, InvalidPort,
    InvalidState, NesRegion, NoRomLoaded, RomLoadResult, StateLoaded,
};

use crate::timing_profile::TimingProfile;
use crate::{NesCoreOf, RunFrame};

/// **One implementation, two engines** (lot NESHAWK-PROFILE step 4).
///
/// The `impl` is generic over the timing profile, so `NesCoreOf<Standard>` — which is `NesCore` —
/// and `NesCoreOf<NesHawkCompat>` are each an `Engine` with their own vtable and their own
/// monomorphised methods, and this file is written once. The `.bk2` player picks the second when
/// the movie's header names `NesHawk`; everything else, including every `.fm2` and ordinary play,
/// gets the first.
///
/// It is behind **no feature**: step 4 measured the binary and the second core is 44 KiB of
/// `.text`, far under the 2 MB the brief set as the threshold for gating it.
///
/// `run_frame` comes through [`RunFrame`] because the inherent one is declared per instantiation
/// (see `nes_core.rs`); it is the same function.
impl<P: TimingProfile> Engine for NesCoreOf<P> {
    #[inline]
    fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult {
        NesCoreOf::<P>::load_rom(self, image, name)
    }

    #[inline]
    fn close_rom(&mut self) {
        NesCoreOf::<P>::close_rom(self);
    }

    #[inline]
    fn is_running(&self) -> bool {
        NesCoreOf::<P>::is_running(self)
    }

    #[inline]
    fn rom_name(&self) -> Option<&str> {
        NesCoreOf::<P>::rom_name(self)
    }

    /// `NesCoreOf::<P>::reset` **is** the power cycle (DEC-02-Q3), so powering on and a hard reset are
    /// the same call on this core; the trait keeps them apart because FCEUX's are two.
    #[inline]
    fn power(&mut self) {
        NesCoreOf::<P>::reset(self);
    }

    #[inline]
    fn reset(&mut self) {
        NesCoreOf::<P>::reset(self);
    }

    #[inline]
    fn soft_reset(&mut self) {
        NesCoreOf::<P>::soft_reset(self);
    }

    #[inline]
    fn run_frame(&mut self, input: InputWord) -> i64 {
        RunFrame::run_frame(self, input)
    }

    #[inline]
    fn frame_count(&self) -> i64 {
        NesCoreOf::<P>::frame_count(self)
    }

    #[inline]
    fn set_frame_count(&mut self, value: i64) {
        NesCoreOf::<P>::set_frame_count(self, value);
    }

    #[inline]
    fn pixel_frame_indices_into(&self, out: &mut Vec<u8>) {
        NesCoreOf::<P>::pixel_frame_indices_into(self, out);
    }

    #[inline]
    fn frame_batch(&self) -> FrameBatch<'_> {
        NesCoreOf::<P>::frame_batch(self)
    }

    #[inline]
    fn read_audio(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError> {
        NesCoreOf::<P>::read_audio(self, destination, stereo)
    }

    #[inline]
    fn set_sample_rate(&mut self, value: i32) -> Result<(), AudioError> {
        NesCoreOf::<P>::set_sample_rate(self, value)
    }

    #[inline]
    fn sample_rate(&self) -> i32 {
        NesCoreOf::<P>::sample_rate(self)
    }

    #[inline]
    fn read_memory(&self, address: u16) -> u8 {
        NesCoreOf::<P>::read_memory(self, address)
    }

    #[inline]
    fn write_memory(&mut self, address: u16, value: u8) {
        NesCoreOf::<P>::write_memory(self, address, value);
    }

    #[inline]
    fn save_state_to(
        &self,
        include_rom_image: bool,
        buffer: &mut Vec<u8>,
    ) -> Result<(), NoRomLoaded> {
        NesCoreOf::<P>::save_state_to(self, include_rom_image, buffer)
    }

    #[inline]
    fn max_state_size(&self) -> usize {
        NesCoreOf::<P>::max_state_size(self)
    }

    #[inline]
    fn try_load_state(&mut self, data: &[u8], offset: usize) -> Result<StateLoaded, InvalidState> {
        NesCoreOf::<P>::try_load_state(self, data, offset)
    }

    /// The one place this file is not a bare forward: the trait copies, because FCEUX's bytes
    /// come across a C API and cannot be lent out of a `&self`. `NesCoreOf::<P>::export_battery_ram`
    /// still lends, and a caller that holds the concrete core keeps using it.
    #[inline]
    fn export_battery_ram(&self, out: &mut Vec<u8>) {
        out.clear();
        out.extend_from_slice(NesCoreOf::<P>::export_battery_ram(self));
    }

    #[inline]
    fn import_battery_ram(&mut self, data: &[u8]) -> bool {
        NesCoreOf::<P>::import_battery_ram(self, data)
    }

    #[inline]
    fn region(&self) -> NesRegion {
        NesCoreOf::<P>::region(self)
    }

    #[inline]
    fn set_region(&mut self, region: NesRegion) {
        NesCoreOf::<P>::set_region(self, region);
    }

    #[inline]
    fn port_type(&self, port: usize) -> Result<ControllerPortType, InvalidPort> {
        NesCoreOf::<P>::port_type(self, port)
    }

    #[inline]
    fn set_port_type(&mut self, port: usize, kind: ControllerPortType) -> Result<(), InvalidPort> {
        NesCoreOf::<P>::set_port_type(self, port, kind)
    }

    #[inline]
    fn capabilities(&self) -> Capabilities {
        Capabilities::ZAXNES
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
