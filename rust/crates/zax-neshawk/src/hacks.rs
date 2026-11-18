//! The built-in patches, the hack packs and their aids on the NesHawk engine (lot
//! NESHAWK-HACKS step 3).
//!
//! **Not a port of anything**: this is `zax_nes::NesCore`'s handling of the same things
//! (`patch_image`, `install_pack`, `sync_aids`, `boot_pack_zone`, `boot_patch_window`,
//! `set_hack_options`), done against this crate's machine and calling the very same
//! `zax_nes::builtin_patches` and `zax_nes::hack_pack` code, so that a game gets the same patch,
//! the same pack and the same hooks on both engines.
//!
//! # The rule that keeps the timings
//!
//! Nothing here touches the machine unless the host asked for it: the built-in patches are **off
//! by default** on this engine (the application turns them on with the widescreen only), no aid
//! is on until the host sets its bit, and the widescreen extension is attached
//! ([`crate::wide`]) only when a patch runs, an aid is on, or a margin is offered. A `.bk2`
//! played with none of those runs the NesHawk port exactly as before the lot.

use zax_nes::builtin_patches::{self, BuiltinPatch, HackOption, PatchedRom};
use zax_nes::hack_pack::AppliedPack;

use crate::nes::Nes;
use crate::wide::WideHooks;

/// What the host chose and what the loaded ROM got.
#[derive(Debug, Clone, Default)]
pub struct Hacks {
    /// Whether the next load applies the built-in patch made for the ROM (`NesCore`'s
    /// `builtin_patches_enabled`, which defaults to **true** there; **false** here, for the
    /// reason in this module's header).
    pub builtin_patches: bool,
    /// Whether an advanced patch (`BuiltinPatch::advanced`) applies too.
    pub advanced_patches: bool,
    /// The patch applied to a ROM none is made for, while the patches are on.
    pub forced: Option<&'static BuiltinPatch>,
    /// The widescreen margin the host offers on each side, in pixels (0 = off).
    pub host_margin: i32,
    /// The margin it offers above and below.
    pub host_vertical_margin: i32,
    /// The options of the patch, one bit each (`$500A` the low byte; bits 8-15 aids only).
    pub host_options: u16,
    /// The built-in patch the loaded ROM runs, by name.
    pub applied: Option<&'static str>,
    /// The hack pack made for the loaded ROM, resolved on its original image.
    pub pack: Option<(&'static BuiltinPatch, AppliedPack)>,
    /// The aids whose hooks are in the board's PRG-ROM now, one bit each.
    pub aids_on: u16,
    /// The CRC of the loaded ROM's data (`builtin_patches::data_crc`), which finds a decoder.
    pub rom_crc: Option<u32>,
}

impl Hacks {
    /// `NesCore::patch_image`: the built-in patch `image` gets as the switches are set.
    #[must_use]
    pub fn patch_image(&self, image: &[u8]) -> Option<PatchedRom> {
        if !self.builtin_patches {
            return None;
        }
        builtin_patches::apply_with(image, self.forced).filter(|patched| {
            self.advanced_patches
                || builtin_patches::by_name(patched.name).is_none_or(|patch| !patch.advanced)
        })
    }

    /// A ROM is being loaded from `image` (the host's, before any patch): the patch it gets,
    /// the pack made for it, and the patch's default options.
    pub fn on_load(&mut self, image: &[u8]) {
        self.applied = self.patch_image(image).map(|patched| patched.name);
        self.pack = builtin_patches::pack_for(image);
        self.aids_on = 0;
        self.rom_crc = Some(builtin_patches::data_crc(image));
        self.host_options = self
            .applied
            .and_then(builtin_patches::by_name)
            .or_else(|| builtin_patches::patch_for(image))
            .map_or(0, BuiltinPatch::default_options);
    }

    /// The ROM was closed.
    pub fn on_close(&mut self) {
        self.applied = None;
        self.pack = None;
        self.aids_on = 0;
        self.rom_crc = None;
    }

    /// `NesCore::find_margin_decoder`: the margin decoder of the loaded ROM, while the
    /// built-in patches are on.
    #[must_use]
    pub fn decoder(&self) -> Option<&'static dyn zax_nes::wide_decode::MarginDecoder> {
        if !self.builtin_patches {
            return None;
        }
        self.applied
            .and_then(zax_nes::wide_decode::for_patch)
            .or_else(|| self.rom_crc.and_then(zax_nes::wide_decode::for_rom))
            .or_else(|| {
                self.pack
                    .as_ref()
                    .and_then(|(patch, _)| zax_nes::wide_decode::for_patch(patch.name))
            })
    }

    /// The built-in patch whose properties count: the one applied, or the one whose pack gives
    /// the ROM its aids (`NesCore::options_patch_of_the_rom`).
    #[must_use]
    pub fn options_patch(&self) -> Option<&'static BuiltinPatch> {
        self.applied
            .and_then(builtin_patches::by_name)
            .or_else(|| self.pack.as_ref().map(|(patch, _)| *patch))
    }

    /// `NesCore::hack_options`: the options of the patch applied, or of the pack's aids.
    #[must_use]
    pub fn hack_options(&self) -> Vec<HackOption> {
        if let Some(patch) = self.applied.and_then(builtin_patches::by_name) {
            return patch.options.to_vec();
        }
        self.pack.as_ref().map_or_else(Vec::new, |(patch, pack)| {
            patch
                .options
                .iter()
                .filter(|option| pack.component(option.key).is_some())
                .copied()
                .collect()
        })
    }

    /// Whether an aid of the pack has its option on.
    fn aids_wanted(&self) -> bool {
        self.pack.as_ref().is_some_and(|(patch, pack)| {
            patch.options.iter().any(|option| {
                pack.component(option.key).is_some()
                    && self.host_options & (1 << (option.bit & 15)) != 0
            })
        })
    }

    /// Whether the machine needs the widescreen extension: a patch runs, an aid is on, or the
    /// host offers a margin. `false` is the C#'s machine.
    #[must_use]
    pub fn needs_extension(&self) -> bool {
        self.applied.is_some() || self.aids_wanted() || self.host_margin > 0
    }

    /// A machine was just built and powered on for the loaded ROM (a load or a hard reset):
    /// the extension attached if needed, booted as the patch asks, and the aids applied.
    pub fn attach(&mut self, nes: &mut Nes) {
        nes.b.wide = None;
        self.aids_on = 0;
        if !self.needs_extension() {
            return;
        }
        let mut wide = WideHooks::new();
        wide.ext.set_host_margin(self.host_margin);
        wide.ext.set_host_vertical_margin(self.host_vertical_margin);
        wide.ext
            .set_host_options(self.host_options.to_le_bytes()[0]);
        // `NesCore::install_pack` and `boot_patch_window`: the pack's code while its widescreen
        // runs, the window and the zone the patch boots with.
        let running = match (&self.pack, self.applied) {
            (Some((patch, pack)), Some(name)) if patch.name == name => Some((*patch, pack.code)),
            _ => None,
        };
        wide.ext.set_code(running.map_or(&[], |(_, code)| code));
        if let Some(patch) = self.applied.and_then(builtin_patches::by_name) {
            if let Some((bank, page)) = patch.boot_window {
                wide.ext.boot(bank, page, !patch.boot_window_keeps_prg_ram);
            }
            if patch.boot_zone {
                wide.ext.boot_zone(patch.zone_data);
            }
        }
        nes.b.wide = Some(wide);
        self.refresh(nes);
        self.sync_aids(nes);
    }

    /// The values the extension reads from the host and the patch, after either changed.
    fn refresh(&self, nes: &mut Nes) {
        let idle_loop = self.options_patch().and_then(|patch| patch.idle_loop);
        let decoded = self.decoder().is_some();
        if let Some(wide) = nes.b.wide.as_deref_mut() {
            wide.idle_loop = idle_loop;
            // A game with a margin decoder gets the host's margin whatever its ROM asks for.
            wide.ext.set_decoded(decoded);
            wide.capture_rows = decoded;
            wide.ext.set_host_margin(self.host_margin);
            wide.ext.set_host_vertical_margin(self.host_vertical_margin);
            wide.ext
                .set_host_options(self.host_options.to_le_bytes()[0]);
        }
    }

    /// Attaches a fresh, locked extension to a running machine that has none and now needs one
    /// (an aid turned on, a margin offered mid-game). Locked, it changes nothing until a ROM
    /// unlocks it or a pack boots it.
    fn ensure_extension(&self, nes: &mut Nes) {
        if nes.b.wide.is_none() && self.needs_extension() {
            nes.b.wide = Some(WideHooks::new());
            self.refresh(nes);
        }
    }

    /// `NesCore::set_hack_options`.
    pub fn set_hack_options(&mut self, options: u16, nes: Option<&mut Nes>) {
        self.host_options = options;
        if let Some(nes) = nes {
            self.ensure_extension(nes);
            self.refresh(nes);
            self.sync_aids(nes);
        }
    }

    /// `NesCore::set_widescreen_margin` and its vertical twin.
    pub fn set_margins(&mut self, horizontal: i32, vertical: i32, nes: Option<&mut Nes>) {
        self.host_margin = horizontal;
        self.host_vertical_margin = vertical;
        if let Some(nes) = nes {
            self.ensure_extension(nes);
            self.refresh(nes);
        }
    }

    /// `NesCore::sync_aids`: the hooks of the aids whose option is on put into the board's
    /// PRG-ROM, the game's bytes back where it is off, then `boot_pack_zone`.
    fn sync_aids(&mut self, nes: &mut Nes) {
        let Some((patch, pack)) = &self.pack else {
            return;
        };
        let mut turned_off = false;
        // The fallback board (`boards::zaxnes`) keeps its PRG-ROM inside Zaxnes's cartridge, out
        // of reach here: its aids stay off rather than being half applied.
        if !nes.b.board.base().rom.is_empty() {
            for option in patch.options {
                let Some(component) = pack.component(option.key) else {
                    continue;
                };
                let mask = 1 << (option.bit & 15);
                let on = self.host_options & mask != 0;
                if on != (self.aids_on & mask != 0) {
                    component.apply(&mut nes.b.board.base_mut().rom, on);
                    self.aids_on ^= mask;
                    turned_off |= !on;
                }
            }
        }
        let Some(wide) = nes.b.wide.as_deref_mut() else {
            return;
        };
        if turned_off {
            wide.ext.clear_overclock();
        }
        // `boot_pack_zone`: the zone on the pack's code while its widescreen runs or an aid is
        // on (an aid's hooks may call the pack's code).
        let running = self.applied == Some(patch.name);
        if !patch.boot_zone || pack.code.is_empty() || !(running || self.aids_on != 0) {
            return;
        }
        if wide.ext.code() != pack.code {
            wide.ext.set_code(pack.code);
        }
        if !wide.ext.unlocked() {
            wide.ext.boot_zone(patch.zone_data);
        }
    }

    /// `NesCore::applied_components`: the pack's components the machine runs (the widescreen
    /// while the patch runs, each aid whose hooks are in).
    #[must_use]
    pub fn applied_components(&self) -> Vec<&'static str> {
        let Some((patch, pack)) = &self.pack else {
            return Vec::new();
        };
        let mut components = Vec::new();
        if self.applied == Some(patch.name)
            && pack.component(zax_nes::hack_pack::WIDESCREEN).is_some()
        {
            components.push(zax_nes::hack_pack::WIDESCREEN);
        }
        for option in patch.options {
            if self.aids_on & (1 << (option.bit & 15)) != 0 && pack.component(option.key).is_some()
            {
                components.push(option.key);
            }
        }
        components
    }
}
