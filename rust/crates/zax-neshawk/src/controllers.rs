//! The controller ports.
//!
//! Ported from BizHawk 2.11.1 `Consoles/Nintendo/NES/NESControllers.cs`: `StrobeInfo`, `NesDeck`,
//! `ControllerNES` and `UnpluggedNES`, which are the only two port devices this lot needs (the
//! *Kirby* movie records `UnpluggedNES` in port 2).
//! MIT; see `LICENSE-BizHawk`.

/// `ControllerNES.Buttons`, in index order: `SerialUtil.Latch` puts button `i` in bit `i`, and
/// every bit from 8 up is 1, which is what makes the shift register return an endless stream of
/// ones once the eight buttons have been read out.
pub const BUTTON_ORDER: [&str; 8] = ["A", "B", "Select", "Start", "Up", "Down", "Left", "Right"];

/// One button of a standard pad, as a bit of [`Pad`].
pub mod button {
    pub const A: u8 = 1 << 0;
    pub const B: u8 = 1 << 1;
    pub const SELECT: u8 = 1 << 2;
    pub const START: u8 = 1 << 3;
    pub const UP: u8 = 1 << 4;
    pub const DOWN: u8 = 1 << 5;
    pub const LEFT: u8 = 1 << 6;
    pub const RIGHT: u8 = 1 << 7;
}

/// What is plugged into a port. `NESControlSettings` can name a dozen devices; these are the two
/// the port implements, and anything else is a gap this lot states rather than fakes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PortDevice {
    /// `UnpluggedNES`: reads 0, ignores the strobe.
    #[default]
    Unplugged,
    /// `ControllerNES`.
    ControllerNes,
    /// `ControllerNES(famicomP2: true)`, whose Select and Start are not wired.
    ControllerNesFamicomP2,
}

/// `ControllerNES`'s state. "reset is not edge triggered; so long as it's high, the latch is
/// continuously reloading", which is why `Read` latches too.
#[derive(Debug, Clone, Copy, Default)]
struct ControllerNes {
    resetting: bool,
    latched_value: i32,
}

impl ControllerNes {
    /// `SerialUtil.Latch` over `Buttons` (or `FamicomP2Buttons`, where index 2 and 3 are null and
    /// so contribute nothing).
    fn latch(&mut self, buttons: u8, famicom_p2: bool) {
        let mask = if famicom_p2 { buttons & !0x0C } else { buttons };
        // bits 0-7 from the pad, bits 8-31 set: "1 in all other bits"
        self.latched_value = i32::from(mask) | -256; // 0xFFFFFF00 as i32
    }

    /// `Strobe`.
    fn strobe(&mut self, out0: i32, out0_old: i32, buttons: u8, famicom_p2: bool) {
        self.resetting = out0 != 0;
        if out0 < out0_old {
            self.latch(buttons, famicom_p2);
        }
    }

    /// `Read`.
    fn read(&mut self, buttons: u8, famicom_p2: bool) -> u8 {
        if self.resetting {
            self.latch(buttons, famicom_p2);
        }
        let ret = (self.latched_value & 1) as u8;
        if !self.resetting {
            self.latched_value >>= 1; // ASR not LSR, so endless stream of 1s after data
        }
        ret
    }
}

/// `NesDeck`: a NES with two attached devices.
#[derive(Debug, Clone, Default)]
pub struct ControllerDeck {
    pub left: PortDevice,
    pub right: PortDevice,
    /// The pressed buttons of each port, which stands in for `IController.IsPressed`.
    pub pad: [u8; 2],
    left_state: ControllerNes,
    right_state: ControllerNes,
}

impl ControllerDeck {
    #[must_use]
    pub fn new(left: PortDevice, right: PortDevice) -> Self {
        Self {
            left,
            right,
            pad: [0; 2],
            left_state: ControllerNes::default(),
            right_state: ControllerNes::default(),
        }
    }

    /// Plugs a device into port 0 or 1, clearing that port's shift register — which is what
    /// `NES.PutSyncSettings` does by rebuilding the deck. Out-of-range ports are ignored; the
    /// engine checks them and reports [`zax_core_api::InvalidPort`].
    pub fn set_device(&mut self, port: usize, device: PortDevice) {
        match port {
            0 => {
                self.left = device;
                self.left_state = ControllerNes::default();
            }
            1 => {
                self.right = device;
                self.right_state = ControllerNes::default();
            }
            _ => {}
        }
    }

    /// `NesDeck.Strobe`, with `StrobeInfo(oldValue, newValue)` decomposed inline: only `OUT0`
    /// reaches a standard pad.
    pub fn strobe(&mut self, old_value: u8, new_value: u8) {
        let out0_old = i32::from(old_value & 1);
        let out0 = i32::from(new_value & 1);

        match self.left {
            PortDevice::Unplugged => {}
            PortDevice::ControllerNes => {
                self.left_state.strobe(out0, out0_old, self.pad[0], false);
            }
            PortDevice::ControllerNesFamicomP2 => {
                self.left_state.strobe(out0, out0_old, self.pad[0], true);
            }
        }
        match self.right {
            PortDevice::Unplugged => {}
            PortDevice::ControllerNes => {
                self.right_state.strobe(out0, out0_old, self.pad[1], false);
            }
            PortDevice::ControllerNesFamicomP2 => {
                self.right_state.strobe(out0, out0_old, self.pad[1], true);
            }
        }
    }

    /// `NesDeck.ReadA`: `_left.Read(...) & 0x19`.
    pub fn read_a(&mut self) -> u8 {
        let v = match self.left {
            PortDevice::Unplugged => 0,
            PortDevice::ControllerNes => self.left_state.read(self.pad[0], false),
            PortDevice::ControllerNesFamicomP2 => self.left_state.read(self.pad[0], true),
        };
        v & 0x19
    }

    /// `NesDeck.ReadB`: `_right.Read(...) & 0x19`.
    pub fn read_b(&mut self) -> u8 {
        let v = match self.right {
            PortDevice::Unplugged => 0,
            PortDevice::ControllerNes => self.right_state.read(self.pad[1], false),
            PortDevice::ControllerNesFamicomP2 => self.right_state.read(self.pad[1], true),
        };
        v & 0x19
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pad_shifts_out_its_eight_buttons_then_ones() {
        // REQ: the latched value is `buttons | 0xFFFFFF00`, read out LSB first, and an arithmetic
        // shift keeps feeding 1s afterwards — `ControllerNES.Read`'s "ASR not LSR" comment.
        let mut deck = ControllerDeck::new(PortDevice::ControllerNes, PortDevice::Unplugged);
        deck.pad[0] = button::A | button::RIGHT;

        // strobe high then low, which is what a game's `LDA #1 / STA $4016 / LDA #0 / STA $4016`
        // does, and is the only transition that latches
        deck.strobe(0, 1);
        deck.strobe(1, 0);

        let bits: Vec<u8> = (0..8).map(|_| deck.read_a()).collect();
        assert_eq!(bits, vec![1, 0, 0, 0, 0, 0, 0, 1], "A and Right pressed");
        // and then ones for ever
        assert_eq!(deck.read_a(), 1);
        assert_eq!(deck.read_a(), 1);
    }

    #[test]
    fn an_unplugged_port_reads_zero() {
        let mut deck = ControllerDeck::new(PortDevice::Unplugged, PortDevice::Unplugged);
        deck.strobe(0, 1);
        deck.strobe(1, 0);
        assert_eq!(deck.read_a(), 0);
        assert_eq!(deck.read_b(), 0);
    }
}

// -------------------------------------------------------------------- the savestate

// `NESControllers.cs`: `ControllerNES.SyncState` and the deck's. The two `PortDevice`s are the
// deck's configuration rather than its state, but they are cheap and a block that carries them
// restores the machine the state was taken on rather than the one the host happens to hold.
crate::state::sync_enum!(PortDevice {
    0 => PortDevice::Unplugged,
    1 => PortDevice::ControllerNes,
    2 => PortDevice::ControllerNesFamicomP2,
});

crate::state::sync_struct!(ControllerNes {
    resetting,
    latched_value
});

crate::state::sync_struct!(ControllerDeck {
    left,
    right,
    pad,
    left_state,
    right_state
});
