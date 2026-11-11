//! The widescreen hack turned on or off while its game plays (owner request). A game whose hack
//! pack says that its states load both with and without the hack (the `to_off` and `to_on` of
//! the pack's states section) switches at once: the state of the frame on screen, held in memory
//! and never in a slot, is restored onto the ROM patched, or not, as the settings now say. Any
//! other game keeps what it loaded with until the next load, as before.

use zax_nes::NesCore;

use super::{EmulationHost, GameView};
use crate::emulation::game_catalog::GameCatalog;

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Puts the game in play on the ROM the patch settings now call for, when they changed what
    /// it gets and its pack carries its states across; does nothing otherwise.
    pub(super) fn follow_patch_switch(&mut self) {
        if self.loading || self.job.is_some() || self.movie_timeline.is_some() {
            return;
        }
        let wanted = (
            self.builtin_patches_wanted(),
            self.advanced_patches_wanted(),
        );
        if !self
            .session
            .as_ref()
            .is_some_and(|session| session.has_rom() && session.core().patch_switches() != wanted)
        {
            return;
        }
        self.complete_frame_in_flight();
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let before = session.core().patch_switches();
        let Ok(state) = session.core().save_state(true) else {
            return;
        };
        // The settings tell the core what a restore gets; asking the state of the frame on
        // screen whether it fits them tells whether that changed for this game.
        session.core_mut().set_builtin_patches(wanted.0);
        session.core_mut().set_advanced_patches(wanted.1);
        let switch = session.core().state_fits_builtin_patches(&state, 0) == Some(false);
        let patch = NesCore::state_builtin_patch(&state, 0).filter(|_| switch);
        let to_off = NesCore::state_uses_widescreen(&state, 0) == Some(true);
        let carried = patch.is_some_and(|patch| {
            let states = patch.states();
            if to_off {
                states.to_off
            } else {
                states.to_on
            }
        });
        let Some(patch) = patch.filter(|_| carried) else {
            // The game keeps what it loaded with, its core as it was, until the next load.
            session.core_mut().set_builtin_patches(before.0);
            session.core_mut().set_advanced_patches(before.1);
            return;
        };
        let ports = [session.core().port_type(0), session.core().port_type(1)];
        let margin = session.core().widescreen_margin();
        if let Err(error) = session.restore_core(&state) {
            self.log(&format!("hack switch refused: {error}"));
            return;
        }
        for (port, kind) in ports.into_iter().enumerate() {
            if let Ok(kind) = kind {
                let _ = session.core_mut().set_port_type(port, kind);
            }
        }
        session.core_mut().set_widescreen_margin(margin);
        let bits = self.hack_option_bits();
        if let Some(session) = self.session.as_mut() {
            session.core_mut().set_hack_options(bits);
        }
        // The history was made on the other ROM.
        self.rollback.clear();
        self.frame_steps.clear();
        self.pacer.reset();
        self.resynchronise_audio();
        self.log(&format!(
            "{} {} while playing",
            patch.name,
            if to_off { "taken off" } else { "put on" }
        ));
    }
}
