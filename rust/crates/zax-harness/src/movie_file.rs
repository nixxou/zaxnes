//! The TAS movie files [`crate::movie_sync`] plays, reduced to the one thing both engines can be
//! driven from: **a region, a power-on work RAM, and one input word with its command per frame**.
//!
//! `.fm2` (FCEUX) is read here through `zax_nes::movie::fm2`, which is the application's own
//! reader. `.bk2` (`BizHawk`) is read into the same type by the module step 4 of this lot adds,
//! so that `movie-sync` needs to know nothing about either format.
//!
//! # Why the header's own fields are kept
//!
//! A movie replays only on the machine it was recorded on, and three header fields *are* that
//! machine: the region, the power-on contents of the 2 KiB of work RAM, and whether a frame
//! carries a reset. Getting any of the three wrong parts the two engines in a way no accuracy
//! work could ever close, so each one is read, named in the report, and applied to **both** sides.
//!
//! The one that is easy to miss is the second. An `.fm2` with no `RAMInitOption` line is **not**
//! a movie that wants zeroes: `MovieData`'s constructor defaults the option to 0 and
//! `FCEUI_LoadMovie` copies it into FCEUX's global, and option 0 is FCEUX's
//! `00 00 00 00 FF FF FF FF` pattern. `zax-nes` powers on all-zero (REQ-CORE-004). So the default
//! of the format is a pattern, and [`RamPattern::FceuxDefault`] is what a header that says
//! nothing asks for.

use std::collections::BTreeMap;
use std::fmt;

use zax_nes::input::ControllerPortType;
use zax_nes::movie::fm2::{Fm2Movie, COMMAND_HARD_RESET, COMMAND_SOFT_RESET};


/// What a frame's command column asks for before the frame runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameCommand {
    /// Nothing.
    #[default]
    None,
    /// The console's reset button (`.fm2` command 1).
    SoftReset,
    /// A power cycle (`.fm2` command 2). On the movie's own first frame this is how FCEUX writes
    /// "recorded from power-on" and means nothing more, which [`MovieFile::read_fm2`] drops.
    HardReset,
}

/// One frame of a movie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameInput {
    /// The input word as `NesCore::run_frame` takes it: port 1 in bits 0-7 (A, B, Select, Start,
    /// Up, Down, Left, Right), port 2 in bits 8-15.
    pub word: u16,
    /// What happens to the machine before the frame.
    pub command: FrameCommand,
}

/// What a machine's 2 KiB of work RAM holds at power-on.
///
/// The four FCEUX `RAMInitOption` values, under the names the report uses. Applied to **both**
/// engines by `movie-sync`: FCEUX through `RamInit`, Zaxnes by writing the pattern over
/// `NesBus::ram_mut` after the power-on that zeroed it — which touches no emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RamPattern {
    /// `RAMInitOption` 0, the format's default: `00 00 00 00 FF FF FF FF` repeating.
    #[default]
    FceuxDefault,
    /// `RAMInitOption` 1: all `$FF`.
    Ff,
    /// `RAMInitOption` 2: all `$00` — what `zax-nes` does of its own accord.
    Zero,
    /// `RAMInitOption` 3: `xoroshiro128+` seeded from `RAMInitSeed`, one draw per byte.
    Random(i32),
}

impl RamPattern {
    /// The pattern named on a command line (`default`, `ff`, `zero`, `random:<seed>`).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "default" | "fceux" => Some(Self::FceuxDefault),
            "ff" => Some(Self::Ff),
            "zero" => Some(Self::Zero),
            _ => text
                .strip_prefix("random:")
                .and_then(|seed| seed.parse().ok())
                .map(Self::Random),
        }
    }

    /// Writes the pattern over `ram`, which is one machine's whole work RAM.
    ///
    /// `Random` is `FCEU_MemoryRand`'s own generator (`fceu.cpp:945-988`): `splitmix64` over the
    /// seed fills the two words of state, then one `xoroshiro128+` draw per byte, of which the
    /// **low** byte is kept. Both algorithms are published (Vigna, Blackman); the test
    /// `the_random_pattern_is_the_one_fceux_writes` pins this against FCEUX itself rather than
    /// against the reading.
    #[allow(
        // `RAMInitSeed` is a C `int` reinterpreted as the `u32` the seeding takes, and the draw
        // keeps its low byte -- both are what `FCEU_MemoryRand` itself does, and reproducing the
        // pattern means reproducing them exactly.
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    pub fn fill(self, ram: &mut [u8]) {
        match self {
            Self::FceuxDefault => {
                for (index, byte) in ram.iter_mut().enumerate() {
                    *byte = if index & 4 != 0 { 0xFF } else { 0x00 };
                }
            }
            Self::Ff => ram.fill(0xFF),
            Self::Zero => ram.fill(0),
            Self::Random(seed) => {
                let mut state = splitmix64_pair(seed as u32);
                for byte in ram.iter_mut() {
                    *byte = xoroshiro128plus_next(&mut state) as u8;
                }
            }
        }
    }
}

impl fmt::Display for RamPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Named by its bytes rather than by one emulator, because lot EVAL-SYNC-2 measured
            // the same pattern on `NesHawk`: it is FCEUX's `RAMInitOption 0` *and* what a `.bk2`
            // turns out to want.
            Self::FceuxDefault => {
                f.write_str("the 00 00 00 00 FF FF FF FF pattern (FCEUX `RAMInitOption 0`)")
            }
            Self::Ff => f.write_str("all $FF"),
            Self::Zero => f.write_str("all $00"),
            Self::Random(seed) => write!(f, "xoroshiro128+ from seed {seed}"),
        }
    }
}

/// The two words of `xoroshiro128+` state, seeded as `xoroshiro128plus_seed` seeds them.
fn splitmix64_pair(seed: u32) -> [u64; 2] {
    let mut x = u64::from(seed);
    std::array::from_fn(|_| {
        x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    })
}

/// One draw of `xoroshiro128+`.
fn xoroshiro128plus_next(state: &mut [u64; 2]) -> u64 {
    let s0 = state[0];
    let mut s1 = state[1];
    let result = s0.wrapping_add(s1);
    s1 ^= s0;
    state[0] = s0.rotate_left(55) ^ s1 ^ (s1 << 14);
    state[1] = s1.rotate_left(36);
    result
}

/// A movie reduced to what drives a machine, with everything the report needs to say about it.
#[derive(Debug, Clone, Default)]
pub struct MovieFile {
    /// `fm2` or `bk2`.
    pub kind: &'static str,
    /// The movie was recorded on a PAL console.
    ///
    /// Meaningless unless [`Self::pal_stated`] is true.
    pub pal: bool,
    /// Whether the file **states** a region at all.
    ///
    /// An `.fm2` always does, in `palFlag`. A `.bk2` does not: the region comes from the core's
    /// own game database, and the only thing the movie carries is a `RegionOverride` that
    /// [`crate::bk2`] refuses unless it is 0. So when this is false the caller takes the region
    /// from the ROM's own iNES header, which is `InesHeader`'s rule and REQ-CORE-013's.
    pub pal_stated: bool,
    /// What the machine's work RAM holds at power-on.
    pub ram: RamPattern,
    /// The MD5 of the ROM data the movie names, when it names one (`.fm2`'s `romChecksum`).
    pub rom_md5: Option<[u8; 16]>,
    /// The SHA-1 of the ROM data the movie names, when it names one (`.bk2`'s `SHA1`).
    pub rom_sha1: Option<[u8; 20]>,
    /// One entry per frame, from power-on.
    pub frames: Vec<FrameInput>,
    /// Whether the file's command column means "**after** this frame" rather than "before it".
    ///
    /// The two formats differ, and both answers were measured rather than read off a sentence:
    ///
    /// * an **`.fm2`**'s commands are "interpreted as a bit field corresponding to miscellaneous
    ///   input states which are **valid at the start of the frame**" (the public format page), and
    ///   FCEUX's own movie mode confirms it — lot EVAL-SYNC-3 step 1 plays the same synthetic
    ///   movie through `FCEUI_LoadMovie` and through this instrument's fed inputs and gets the
    ///   same 2 KiB at every frame only with the reset applied **before** the flagged frame. So
    ///   this is `false`;
    /// * a **`.bk2`**'s `Reset` column is the other way round: `BizHawk` runs the flagged frame
    ///   normally and resets after it. See [`crate::bk2::read`], whose note carries the
    ///   measurement.
    ///
    /// `movie-sync` turns this into the base placement of the command, which `--resets` then
    /// probes ±1 around.
    pub reset_after_frame: bool,
    /// What is plugged into each controller port, port 0 first.
    ///
    /// A `.bk2` names it per port in `SyncSettings.json` (`NesLeftPort` / `NesRightPort`), and
    /// `UnpluggedNES` is an **empty socket**, not a pad with no button held — the two differ from
    /// the ninth read after a strobe, which only a DMA that adds a clock ever reaches. An `.fm2`
    /// states nothing of the kind, so it is two pads.
    pub ports: [ControllerPortType; 2],
    /// The header, key by key, as the file wrote it — for the report, and for nothing else.
    pub header: BTreeMap<String, String>,
    /// What the reader had to decide for itself, each a sentence. A reader that cannot honour
    /// something **refuses** instead; these are the things it honoured but had to choose about.
    pub notes: Vec<String>,
}

impl MovieFile {
    /// The **engine** this movie should be replayed on, `None` when nothing in the header asks
    /// for a particular one.
    ///
    /// The sibling of [`Self::recorded_profile`]: that one picks a *timing chip* for Zaxnes's own
    /// core, this one picks the core. A `.bk2` names it in `Header.txt`, and
    /// [`zax_core_api::EngineKind::for_bizhawk_core`] is the one place the mapping lives — so
    /// `movie-sync`'s default engine and any future importer agree. `NesHawk` is the one name
    /// this build answers; `QuickNes` has no port here.
    #[must_use]
    pub fn recorded_engine(&self) -> Option<zax_core_api::EngineKind> {
        if self.kind != "bk2" {
            return None;
        }
        zax_core_api::EngineKind::for_bizhawk_core(self.header.get("Core")?)
    }

    /// Reads an `.fm2`.
    ///
    /// `rom_is_cartridge` is whether the image the movie will be played on is an iNES cartridge.
    /// It decides the one contradiction a real TAS turns out to carry: a header that says `FDS 1`
    /// for a cartridge game. The *image* decides whether the machine has a disk drive, so the
    /// claim is dropped with a note rather than obeyed — and the line is removed before the
    /// application's own reader sees it, because that reader (rightly) refuses the format
    /// outright and this one must not change.
    ///
    /// # Errors
    /// The file is not a version-3 `.fm2` this instrument can replay; the message says why.
    pub fn read_fm2(bytes: &[u8], rom_is_cartridge: bool) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "the file is not text".to_owned())?;
        let header = fm2_header(text);
        let mut notes = Vec::new();
        let mut cleaned = None;
        if header.get("FDS").is_some_and(|value| value != "0") {
            if !rom_is_cartridge {
                return Err("the movie is of a Famicom Disk System disk".to_owned());
            }
            notes.push(
                "the header says `FDS 1` but the image is an iNES cartridge, so the claim is \
                 dropped: the image decides whether the machine has a disk drive"
                    .to_owned(),
            );
            cleaned = Some(
                text.lines()
                    .filter(|line| !line.starts_with("FDS "))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        if header.get("NewPPU").is_some_and(|value| value != "0") {
            return Err("the movie was recorded on FCEUX's new PPU".to_owned());
        }
        let source = cleaned.as_deref().map_or(bytes, str::as_bytes);
        let movie = Fm2Movie::parse(source).map_err(|error| error.to_string())?;

        let ram = ram_pattern_of(&header, &mut notes);
        let frames = movie
            .frames
            .iter()
            .enumerate()
            .map(|(index, frame)| FrameInput {
                word: frame.word,
                command: match frame.commands {
                    // FCEUX writes a power-on command on the first frame of every movie it
                    // records from power-on; it says "from power-on" and nothing more.
                    _ if index == 0 && frame.commands & COMMAND_HARD_RESET != 0 => {
                        FrameCommand::None
                    }
                    commands if commands & COMMAND_HARD_RESET != 0 => FrameCommand::HardReset,
                    commands if commands & COMMAND_SOFT_RESET != 0 => FrameCommand::SoftReset,
                    _ => FrameCommand::None,
                },
            })
            .collect();

        Ok(Self {
            kind: "fm2",
            pal: movie.header.pal,
            pal_stated: true,
            ram,
            rom_md5: movie.header.rom_md5,
            rom_sha1: None,
            frames,
            // Measured in step 1 of lot EVAL-SYNC-3 against FCEUX's own movie mode, and what the
            // public format page says in words: a frame's commands are "valid at the start of the
            // frame".
            reset_after_frame: false,
            // An `.fm2` has no field for it: FCEUX's movie format states the buttons and not the
            // sockets, so both hold a standard pad, which is what this reader has always done.
            ports: [ControllerPortType::Controller; 2],
            header,
            notes,
        })
    }

    /// The frames that carry a command, with it.
    #[must_use]
    pub fn commands(&self) -> Vec<(usize, FrameCommand)> {
        self.frames
            .iter()
            .enumerate()
            .filter(|(_, frame)| frame.command != FrameCommand::None)
            .map(|(index, frame)| (index, frame.command))
            .collect()
    }
}

/// `RAMInitOption` / `RAMInitSeed`, with the note the absence of the first one deserves.
fn ram_pattern_of(header: &BTreeMap<String, String>, notes: &mut Vec<String>) -> RamPattern {
    let seed = header
        .get("RAMInitSeed")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    match header.get("RAMInitOption").map(String::as_str) {
        None => {
            notes.push(
                "the header states no `RAMInitOption`, which in the format means option 0 — \
                 FCEUX's `00 00 00 00 FF FF FF FF` pattern, not zeroes"
                    .to_owned(),
            );
            RamPattern::FceuxDefault
        }
        Some("0") => RamPattern::FceuxDefault,
        Some("1") => RamPattern::Ff,
        Some("2") => RamPattern::Zero,
        Some("3") => RamPattern::Random(seed),
        Some(other) => {
            notes.push(format!(
                "`RAMInitOption {other}` is not one of FCEUX's four; option 0 was used"
            ));
            RamPattern::FceuxDefault
        }
    }
}

/// The `key value` lines of an `.fm2` header, in order of key, with the frame lines left out.
///
/// Repeated keys (`comment`, `subtitle`) keep the first, which is all the report wants of them.
fn fm2_header(text: &str) -> BTreeMap<String, String> {
    let mut header = BTreeMap::new();
    for line in text.lines().take_while(|line| !line.starts_with('|')) {
        let line = line.trim_end_matches('\r');
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        if !key.is_empty() {
            header
                .entry(key.to_owned())
                .or_insert_with(|| value.trim().to_owned());
        }
    }
    header
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-player `.fm2` of `frames` frames, each line `line`, with `extra` header lines.
    fn movie(extra: &str, lines: &[&str]) -> Vec<u8> {
        let mut text =
            format!("version 3\nemuVersion 22020\npalFlag 0\nport0 1\nport1 0\nport2 0\n{extra}");
        for line in lines {
            text.push_str(line);
            text.push('\n');
        }
        text.into_bytes()
    }

    #[test]
    fn an_absent_ram_init_option_is_fceuxs_pattern_and_says_so() {
        let file = MovieFile::read_fm2(&movie("", &["|0|........|"]), true).unwrap_or_default();
        assert_eq!(file.ram, RamPattern::FceuxDefault);
        assert!(
            file.notes.iter().any(|note| note.contains("not zeroes")),
            "the note has to be there: {:?}",
            file.notes
        );
        let mut ram = [0x55u8; 16];
        file.ram.fill(&mut ram);
        assert_eq!(
            ram,
            [0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn the_four_ram_init_options_are_read() {
        let of = |extra: &str| {
            MovieFile::read_fm2(&movie(extra, &["|0|........|"]), true)
                .unwrap_or_default()
                .ram
        };
        assert_eq!(of("RAMInitOption 0\n"), RamPattern::FceuxDefault);
        assert_eq!(of("RAMInitOption 1\n"), RamPattern::Ff);
        assert_eq!(of("RAMInitOption 2\n"), RamPattern::Zero);
        assert_eq!(
            of("RAMInitOption 3\nRAMInitSeed 12345\n"),
            RamPattern::Random(12345)
        );
        // An option the format does not have is not a reason to refuse a movie, but it is a
        // reason to say so.
        let odd = MovieFile::read_fm2(&movie("RAMInitOption 9\n", &["|0|........|"]), true)
            .unwrap_or_default();
        assert_eq!(odd.ram, RamPattern::FceuxDefault);
        assert!(odd
            .notes
            .iter()
            .any(|note| note.contains("RAMInitOption 9")));
    }

    #[test]
    fn a_power_on_command_on_the_first_frame_is_not_a_reset_but_a_later_one_is() {
        let file = MovieFile::read_fm2(
            &movie(
                "",
                &[
                    "|2|........|",
                    "|0|........|",
                    "|1|........|",
                    "|2|........|",
                ],
            ),
            true,
        )
        .unwrap_or_default();
        assert_eq!(
            file.commands(),
            [(2, FrameCommand::SoftReset), (3, FrameCommand::HardReset)]
        );
    }

    #[test]
    fn an_fds_header_on_a_cartridge_is_dropped_with_a_note_and_refused_otherwise() {
        let bytes = movie("FDS 1\n", &["|0|.......A|", "|0|R.......|"]);
        let file = MovieFile::read_fm2(&bytes, true).unwrap_or_default();
        // The frames are still read: dropping the claim must not drop the movie.
        assert_eq!(file.frames.len(), 2);
        assert_eq!(file.frames[0].word, 0x01);
        assert_eq!(file.frames[1].word, 0x80);
        assert!(file.notes.iter().any(|note| note.contains("FDS 1")));
        // The application's own reader, which this one must not change, refuses it outright --
        // which is why the line is removed before it is handed over.
        assert!(Fm2Movie::parse(&bytes).is_err());
        // And with an image that is not a cartridge, the header is believed.
        assert_eq!(
            MovieFile::read_fm2(&bytes, false).err(),
            Some("the movie is of a Famicom Disk System disk".to_owned())
        );
    }

    #[test]
    fn the_new_ppu_and_rubbish_are_refused_by_name() {
        assert_eq!(
            MovieFile::read_fm2(&movie("NewPPU 1\n", &["|0|........|"]), true).err(),
            Some("the movie was recorded on FCEUX's new PPU".to_owned())
        );
        assert!(MovieFile::read_fm2(b"not a movie at all\n", true).is_err());
        assert!(MovieFile::read_fm2(&[0xFF, 0xFE, 0x00], true).is_err());
    }

    #[test]
    fn the_header_is_kept_for_the_report() {
        let file = MovieFile::read_fm2(
            &movie(
                "rerecordCount 99\ncomment author someone\n",
                &["|0|........|"],
            ),
            true,
        )
        .unwrap_or_default();
        assert_eq!(
            file.header.get("rerecordCount").map(String::as_str),
            Some("99")
        );
        assert_eq!(file.header.get("port0").map(String::as_str), Some("1"));
        assert_eq!(file.kind, "fm2");
    }

    #[test]
    fn a_ram_pattern_is_named_on_the_command_line() {
        assert_eq!(RamPattern::parse("zero"), Some(RamPattern::Zero));
        assert_eq!(RamPattern::parse("ff"), Some(RamPattern::Ff));
        assert_eq!(RamPattern::parse("default"), Some(RamPattern::FceuxDefault));
        assert_eq!(RamPattern::parse("random:7"), Some(RamPattern::Random(7)));
        assert_eq!(RamPattern::parse("nonsense"), None);
    }

    #[test]
    fn the_random_pattern_is_reproducible_and_is_not_one_of_the_other_three() {
        let mut a = [0u8; 2048];
        let mut b = [0u8; 2048];
        RamPattern::Random(12345).fill(&mut a);
        RamPattern::Random(12345).fill(&mut b);
        assert_eq!(a, b);
        RamPattern::Random(999).fill(&mut b);
        assert_ne!(a, b);
        assert!(a.iter().any(|&byte| byte != 0 && byte != 0xFF));
    }
}
