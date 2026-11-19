//! The console's own settings the core options choose (the application's Emulation tab and
//! console sound profile): which core variant runs it, and the hardware it imitates. The
//! application hands them to the core in `emulation/host/jobs.rs` (`advance_launch`); this is the
//! same, for `NesCore` alone. Most take effect at the next load, as in the application; the core
//! variant and the sound profile at once.

use zax_nes::NesCore;
use zax_nes::{Mmc3Revision, PowerOnPpuRam, PowerOnRam};
use zax_settings::catalogue::SettingId;

/// Which loop runs the console (`emu.fidelity_plus`, `emu.fast_core`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    /// The standard core.
    #[default]
    Standard,
    /// Fidelity+: the accesses land on the clock edges of the transistor-level model.
    FidelityPlus,
    /// The fast core of the common boards.
    Fast,
}

/// The console settings, each as the catalogue's value of its setting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Machine {
    pub variant: Variant,
    /// The settings of [`SETTINGS`] the options give, by their catalogue value.
    pub values: Vec<(SettingId, i32)>,
}

/// The settings the core exposes, in the order of the application's tab.
pub const SETTINGS: [SettingId; 28] = [
    SettingId::EmuCartridgeDb,
    SettingId::EmuPowerOnRam,
    SettingId::EmuPowerOnPpuRam,
    SettingId::EmuCpuPpuAlignment,
    SettingId::EmuCpuApuParity,
    SettingId::EmuPpuRevision,
    SettingId::EmuCpuRevision,
    SettingId::EmuMmc3Revision,
    SettingId::EmuOamCorruption,
    SettingId::EmuPpuBusConflicts,
    SettingId::EmuOamaddrBug,
    SettingId::EmuOamaddrWriteCorruption,
    SettingId::EmuOamDecay,
    SettingId::EmuPowerOnTop,
    SettingId::EmuPpu2006Delay,
    SettingId::EmuPpu2006Glitch,
    SettingId::EmuPpu2000Glitch,
    SettingId::EmuPaletteCorruption,
    SettingId::EmuPpuResetLine,
    SettingId::EmuVsDip1,
    SettingId::EmuVsDip2,
    SettingId::EmuVsDip3,
    SettingId::EmuVsDip4,
    SettingId::EmuVsDip5,
    SettingId::EmuVsDip6,
    SettingId::EmuVsDip7,
    SettingId::EmuVsDip8,
    SettingId::AudioConsoleProfile,
];

/// A seed for what a console draws at power-on (random RAM, random alignment): the clock, as
/// the application's `power_on_seed`.
fn power_on_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_nanos() & u128::from(u64::MAX)).unwrap_or(0)
        })
}

/// The catalogue default of `id`, as a value: 0 or 1 for a switch.
#[must_use]
pub fn default_value(id: SettingId) -> i32 {
    let descriptor = zax_settings::catalogue::descriptor(id);
    let default = descriptor.default_value();
    if descriptor.kind() == zax_settings::meta::SettingKind::Bool {
        i32::from(default.as_bool())
    } else {
        default.as_int()
    }
}

impl Machine {
    /// The value of `id`, or its catalogue default.
    fn value(&self, id: SettingId) -> i32 {
        self.values
            .iter()
            .find(|(setting, _)| *setting == id)
            .map_or_else(|| default_value(id), |(_, value)| *value)
    }

    fn on(&self, id: SettingId) -> bool {
        self.value(id) != 0
    }

    /// Puts every setting in force on `nes`, before its ROM is loaded.
    pub fn apply_before_load(&self, nes: &mut NesCore) {
        nes.set_cartridge_db(self.on(SettingId::EmuCartridgeDb));
        nes.set_power_on_ram(match self.value(SettingId::EmuPowerOnRam) {
            1 => PowerOnRam::Pattern,
            2 => PowerOnRam::Ff,
            3 => PowerOnRam::Random(power_on_seed()),
            _ => PowerOnRam::Zero,
        });
        nes.set_power_on_ppu_ram(match self.value(SettingId::EmuPowerOnPpuRam) {
            1 => PowerOnPpuRam::Blargg,
            2 => PowerOnPpuRam::Random(power_on_seed() ^ 0x9E37),
            _ => PowerOnPpuRam::Zero,
        });
        nes.set_cpu_ppu_alignment(match self.value(SettingId::EmuCpuPpuAlignment) {
            4 => u8::try_from(power_on_seed() & 3).unwrap_or(3),
            value => u8::try_from(value & 3).unwrap_or(3),
        });
        nes.set_netlist_get_put(match self.value(SettingId::EmuCpuApuParity) {
            1 => true,
            2 => power_on_seed() & 0x100 != 0,
            _ => false,
        });
        nes.set_ppu_pre_g_revision(self.value(SettingId::EmuPpuRevision) == 1);
        nes.set_letterless_cpu(self.value(SettingId::EmuCpuRevision) == 1);
        nes.set_mmc3_revision(match self.value(SettingId::EmuMmc3Revision) {
            1 => Mmc3Revision::A,
            2 => Mmc3Revision::B,
            _ => Mmc3Revision::Cartridge,
        });
        nes.set_palette_corruption(self.on(SettingId::EmuPaletteCorruption));
        nes.set_ppu_reset_line(self.on(SettingId::EmuPpuResetLine));
        nes.set_address_load_delay(self.on(SettingId::EmuPpu2006Delay));
        nes.set_glitch_2006(self.on(SettingId::EmuPpu2006Glitch));
        nes.set_glitch_2000(self.on(SettingId::EmuPpu2000Glitch));
        nes.set_oam_corruption(self.on(SettingId::EmuOamCorruption));
        nes.set_bus_conflicts(self.on(SettingId::EmuPpuBusConflicts));
        nes.set_oamaddr_bug(self.on(SettingId::EmuOamaddrBug));
        nes.set_oamaddr_write_corruption(self.on(SettingId::EmuOamaddrWriteCorruption));
        nes.set_oam_decay(self.on(SettingId::EmuOamDecay));
        nes.set_power_on_top(self.on(SettingId::EmuPowerOnTop));
        let dips = [
            SettingId::EmuVsDip1,
            SettingId::EmuVsDip2,
            SettingId::EmuVsDip3,
            SettingId::EmuVsDip4,
            SettingId::EmuVsDip5,
            SettingId::EmuVsDip6,
            SettingId::EmuVsDip7,
            SettingId::EmuVsDip8,
        ];
        let switches = dips
            .iter()
            .enumerate()
            .filter(|(_, id)| self.on(**id))
            .fold(0u8, |byte, (bit, _)| byte | (1 << bit));
        nes.set_vs_dip_switches(switches);
        self.apply_live(nes);
    }

    /// Puts in force what takes effect at once: the core variant and the sound profile.
    pub fn apply_live(&self, nes: &mut NesCore) {
        nes.set_fidelity_plus(self.variant == Variant::FidelityPlus);
        nes.set_fast_core(self.variant == Variant::Fast);
        nes.set_console_audio_profile(
            usize::try_from(self.value(SettingId::AudioConsoleProfile)).unwrap_or(0),
        );
    }
}
